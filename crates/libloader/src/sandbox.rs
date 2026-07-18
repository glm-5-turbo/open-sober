// SPDX-License-Identifier: MIT
//
// Sandbox — chroot filesystem setup for the isolated environment.
//
// Creates a minimal chroot jail with the device nodes, proc/sys mounts,
// and directory structure needed by the Android binary.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::{Context, Result};
use nix::mount::{mount, umount2, MntFlags, MsFlags};
use nix::sys::stat::{mknod, makedev, Mode, SFlag};
use tracing::{debug, info};

/// Device major/minor numbers for the nodes we create.
mod devnums {
    /// /dev/null — major 1, minor 3
    pub const NULL_MAJOR: u64 = 1;
    pub const NULL_MINOR: u64 = 3;
    /// /dev/zero — major 1, minor 5
    pub const ZERO_MAJOR: u64 = 1;
    pub const ZERO_MINOR: u64 = 5;
    /// /dev/random — major 1, minor 8
    pub const RANDOM_MAJOR: u64 = 1;
    pub const RANDOM_MINOR: u64 = 8;
    /// /dev/urandom — major 1, minor 9
    pub const URANDOM_MAJOR: u64 = 1;
    pub const URANDOM_MINOR: u64 = 9;
    /// /dev/full — major 1, minor 7
    pub const FULL_MAJOR: u64 = 1;
    pub const FULL_MINOR: u64 = 7;
    /// /dev/tty — major 5, minor 0
    pub const TTY_MAJOR: u64 = 5;
    pub const TTY_MINOR: u64 = 0;
    /// /dev/ptmx — major 5, minor 2
    pub const PTMX_MAJOR: u64 = 5;
    pub const PTMX_MINOR: u64 = 2;
}

/// Create the minimal chroot filesystem at the given path.
///
/// This sets up:
/// - /dev/ with essential device nodes (null, zero, random, urandom, etc.)
/// - /proc/ as a mount point (mounts procfs at runtime)
/// - /sys/ as a mount point (mounts sysfs at runtime)
/// - /tmp/ for temporary files
/// - /dev/pts/ for pseudo-terminals (mounts devpts at runtime)
///
/// SAFETY: This function creates device nodes using `mknod(2)`,
/// which requires either root privileges or `CAP_MKNOD`.
pub fn create_minimal_root(root_path: &Path) -> Result<()> {
    debug!("Creating minimal chroot root at {}", root_path.display());

    // Ensure the root directory exists
    ensure_dir(root_path, 0o755)?;

    // /dev — device nodes
    ensure_dir(&root_path.join("dev"), 0o755)?;
    ensure_dir(&root_path.join("dev/pts"), 0o755)?;
    ensure_dir(&root_path.join("dev/shm"), 0o755)?;

    // Create device nodes
    create_dev_null(root_path)?;
    create_dev_zero(root_path)?;
    create_dev_random(root_path)?;
    create_dev_urandom(root_path)?;
    create_dev_full(root_path)?;
    create_dev_tty(root_path)?;
    create_dev_ptmx(root_path)?;
    create_dev_std_symlinks(root_path)?;

    // /proc — process information
    ensure_dir(&root_path.join("proc"), 0o555)?;

    // /sys — kernel information
    ensure_dir(&root_path.join("sys"), 0o555)?;

    // /tmp — temporary files
    ensure_dir(&root_path.join("tmp"), 0o1777)?;

    // /dev/fd — symlink to /proc/self/fd
    create_symlink(root_path, "../proc/self/fd", "dev/fd")?;

    // /dev/stdin, /dev/stdout, /dev/stderr — symlinks to /proc/self/fd/0,1,2
    create_symlink(root_path, "/proc/self/fd/0", "dev/stdin")?;
    create_symlink(root_path, "/proc/self/fd/1", "dev/stdout")?;
    create_symlink(root_path, "/proc/self/fd/2", "dev/stderr")?;

    info!("Minimal chroot root created at {}", root_path.display());
    Ok(())
}

/// Mount proc, sysfs, and devpts inside the chroot.
///
/// This function must be called AFTER chroot(2) into the jail,
/// or with the chroot path as the target of bind mounts.
///
/// SAFETY: mount(2) requires CAP_SYS_ADMIN in the current namespace.
pub fn mount_chroot_filesystems(root_path: &Path) -> Result<()> {
    debug!("Mounting chroot filesystems at {}", root_path.display());

    // Mount procfs at /proc
    // SAFETY: mount(2) with MS_NOSUID|MS_NODEV|MS_NOEXEC for security.
    mount(
        Some("proc"),
        root_path.join("proc").to_str().unwrap(),
        Some("proc"),
        MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC | MsFlags::MS_RDONLY,
        None::<&str>,
    )
    .context("Failed to mount procfs")?;

    // Mount sysfs at /sys
    mount(
        Some("sys"),
        root_path.join("sys").to_str().unwrap(),
        Some("sysfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC | MsFlags::MS_RDONLY,
        None::<&str>,
    )
    .context("Failed to mount sysfs")?;

    // Mount devpts at /dev/pts
    mount(
        Some("devpts"),
        root_path.join("dev/pts").to_str().unwrap(),
        Some("devpts"),
        MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        Some("newinstance,ptmxmode=0666,mode=620,gid=5"),
    )
    .context("Failed to mount devpts")?;

    // Mount tmpfs at /dev/shm
    mount(
        Some("tmpfs"),
        root_path.join("dev/shm").to_str().unwrap(),
        Some("tmpfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC,
        Some("mode=1777"),
    )
    .context("Failed to mount tmpfs for /dev/shm")?;

    info!("Chroot filesystems mounted");
    Ok(())
}

/// Unmount chroot filesystems.
///
/// Should be called before cleaning up the chroot directory.
pub fn unmount_chroot_filesystems(root_path: &Path) -> Result<()> {
    let paths = ["dev/shm", "dev/pts", "sys", "proc"];
    for p in &paths {
        let full_path = root_path.join(p);
        // SAFETY: umount(2) removes a mount.
        let _ = umount2(&full_path, MntFlags::MNT_DETACH)
            .map_err(|e| debug!("Failed to unmount {}: {}", p, e));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Private helpers: directory creation, device nodes, symlinks
// ---------------------------------------------------------------------------

/// Ensure a directory exists with the given permissions.
fn ensure_dir(path: &Path, mode: u32) -> Result<()> {
    if path.exists() {
        if path.is_dir() {
            // Set permissions regardless
            fs::set_permissions(path, fs::Permissions::from_mode(mode))
                .with_context(|| format!("Failed to set permissions on {}", path.display()))?;
            return Ok(());
        } else {
            anyhow::bail!("{} exists but is not a directory", path.display());
        }
    }

    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory {}", path.display()))?;

    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("Failed to set permissions on {}", path.display()))?;

    Ok(())
}

/// Create a character device node.
///
/// SAFETY: `mknod(2)` requires either root privileges or `CAP_MKNOD`.
fn create_dev_node(root_path: &Path, name: &str, major: u64, minor: u64, mode: u32) -> Result<()> {
    let path = root_path.join("dev").join(name);
    if path.exists() {
        debug!("Device {} already exists, skipping", path.display());
        return Ok(());
    }

    // SAFETY: mknod(2) creates a device node. The caller must have
    // appropriate capabilities. We create character devices (S_IFCHR).
    unsafe {
        mknod(
            &path,
            SFlag::S_IFCHR,
            Mode::from_bits_truncate(mode),
            makedev(major, minor),
        )
        .with_context(|| format!("Failed to create device node {}", path.display()))?;
    }

    debug!("Created device node {} ({}:{})", name, major, minor);
    Ok(())
}

/// Create /dev/null — major 1, minor 3
fn create_dev_null(root_path: &Path) -> Result<()> {
    create_dev_node(root_path, "null", devnums::NULL_MAJOR, devnums::NULL_MINOR, 0o666)
}

/// Create /dev/zero — major 1, minor 5
fn create_dev_zero(root_path: &Path) -> Result<()> {
    create_dev_node(root_path, "zero", devnums::ZERO_MAJOR, devnums::ZERO_MINOR, 0o666)
}

/// Create /dev/random — major 1, minor 8
fn create_dev_random(root_path: &Path) -> Result<()> {
    create_dev_node(root_path, "random", devnums::RANDOM_MAJOR, devnums::RANDOM_MINOR, 0o666)
}

/// Create /dev/urandom — major 1, minor 9
fn create_dev_urandom(root_path: &Path) -> Result<()> {
    create_dev_node(root_path, "urandom", devnums::URANDOM_MAJOR, devnums::URANDOM_MINOR, 0o666)
}

/// Create /dev/full — major 1, minor 7
fn create_dev_full(root_path: &Path) -> Result<()> {
    create_dev_node(root_path, "full", devnums::FULL_MAJOR, devnums::FULL_MINOR, 0o666)
}

/// Create /dev/tty — major 5, minor 0
fn create_dev_tty(root_path: &Path) -> Result<()> {
    create_dev_node(root_path, "tty", devnums::TTY_MAJOR, devnums::TTY_MINOR, 0o666)
}

/// Create /dev/ptmx — major 5, minor 2
fn create_dev_ptmx(root_path: &Path) -> Result<()> {
    create_dev_node(root_path, "ptmx", devnums::PTMX_MAJOR, devnums::PTMX_MINOR, 0o666)
}

/// Create symlinks /dev/stdin, /dev/stdout, /dev/stderr -> /proc/self/fd/{0,1,2}
fn create_dev_std_symlinks(root_path: &Path) -> Result<()> {
    let fd_base = Path::new("/proc/self/fd");
    for (name, fd_num) in [("stdin", 0u8), ("stdout", 1), ("stderr", 2)] {
        let path = root_path.join("dev").join(name);
        if path.exists() {
            continue;
        }
        let target = fd_base.join(fd_num.to_string());
        create_symlink(root_path, &target.to_string_lossy(), &format!("dev/{name}"))?;
    }
    Ok(())
}

/// Create a symlink relative to root_path.
fn create_symlink(root_path: &Path, target: &str, link_name: &str) -> Result<()> {
    let link_path = root_path.join(link_name);

    // Create parent directory if needed
    if let Some(parent) = link_path.parent() {
        ensure_dir(parent, 0o755)?;
    }

    if link_path.exists() {
        return Ok(());
    }

    // SAFETY: symlink(2) is safe.
    std::os::unix::fs::symlink(target, &link_path)
        .with_context(|| format!("Failed to create symlink {} -> {}", link_path.display(), target))?;

    debug!("Created symlink {} -> {}", link_name, target);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_ensure_dir_creates() {
        let dir = std::env::temp_dir().join(&format!("sober_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        ensure_dir(&dir, 0o755).unwrap();
        assert!(dir.exists());
        assert!(dir.is_dir());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_dev_numbers_consistent() {
        // Basic sanity: device number constants are nonzero
        assert!(devnums::NULL_MAJOR > 0);
        assert!(devnums::ZERO_MAJOR > 0);
        assert!(devnums::URANDOM_MAJOR > 0);
    }
}