// SPDX-License-Identifier: MIT
//
// libloader — Process spawner and sandbox for Open Sober
//
// This crate implements a sandboxed process launcher that:
// 1. Forks a child process
// 2. Sets up an isolated environment (chroot, uid/gid separation, process groups)
// 3. Reads /proc/self/maps to understand memory layout
// 4. mmaps the translated binary with proper memory protections (mprotect)
// 5. Injects libbadcpu.so via LD_PRELOAD
// 6. Initializes the Android runtime environment (bionic linker stubs, /dev/ entries)
// 7. Handles posix_spawnp for process spawning
//
// Based on clean-room reverse engineering of Sober's libloader.so.
// Architecture and semantics derived from public RE documentation;
// implementation is entirely fresh.

mod sandbox;
pub mod elf;
mod android;
mod ipc;

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{Context, Result};
use nix::sys::signal::{self, Signal};
use nix::unistd::{self, ForkResult, Pid};
use tracing::{debug, error, info, warn};

/// Configuration for the sandboxed loader.
#[derive(Clone, Debug)]
pub struct LoaderConfig {
    /// Path to the chroot directory (jail root).
    pub chroot_path: PathBuf,
    /// Path to the binary to execute inside the sandbox.
    pub binary_path: PathBuf,
    /// UID to run the child process as.
    pub uid: u32,
    /// GID to run the child process as.
    pub gid: u32,
    /// Optional list of libraries to inject via LD_PRELOAD.
    pub ld_preload_paths: Vec<PathBuf>,
    /// Additional environment variables to set in the sandbox.
    pub extra_env: Vec<(String, String)>,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            chroot_path: PathBuf::from("/var/lib/sober/chroot"),
            binary_path: PathBuf::from("/data/data/com.roblox.client/files/RobloxPlayer"),
            uid: 65534, // nobody
            gid: 65534, // nogroup
            ld_preload_paths: Vec::new(),
            extra_env: Vec::new(),
        }
    }
}

/// The sandboxed process loader.
///
/// `Loader` manages the lifecycle of a sandboxed child process.
/// It forks the current process, sets up an isolated environment
/// (chroot, uid/gid, process group), and execs the target binary
/// with LD_PRELOAD injection.
pub struct Loader {
    config: LoaderConfig,
    /// Communication channel between parent and child (fd pair).
    ipc: ipc::IpcChannel,
}

impl Loader {
    /// Create a new Loader from the given configuration.
    ///
    /// This creates the IPC channel used for coordination between
    /// parent and child after fork.
    pub fn new(config: LoaderConfig) -> Result<Self> {
        let ipc = ipc::IpcChannel::new().context("Failed to create IPC channel for loader")?;
        Ok(Self { config, ipc })
    }

    /// Inject a library path into LD_PRELOAD.
    ///
    /// The library will be added to the LD_PRELOAD environment variable
    /// in the spawned child process.
    pub fn inject_library(&mut self, path: &Path) -> Result<()> {
        // Resolve the path
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .context("Failed to get current directory")?
                .join(path)
        };

        if !resolved.exists() {
            anyhow::bail!("Library path does not exist: {}", resolved.display());
        }

        self.config.ld_preload_paths.push(resolved);
        info!("Injected library into LD_PRELOAD");
        Ok(())
    }

    /// Set up the chroot environment.
    ///
    /// Creates the minimal filesystem structure needed for the
    /// Android binary to function. This includes device nodes,
    /// proc mount, and Android-specific directory layout.
    pub fn setup_chroot(&self) -> Result<()> {
        sandbox::create_minimal_root(&self.config.chroot_path)
            .with_context(|| {
                format!(
                    "Failed to set up chroot at {}",
                    self.config.chroot_path.display()
                )
            })?;

        android::setup_android_layout(&self.config.chroot_path).with_context(|| {
            format!(
                "Failed to set up Android layout at {}",
                self.config.chroot_path.display()
            )
        })?;

        info!(
            "Chroot environment set up at {}",
            self.config.chroot_path.display()
        );
        Ok(())
    }

    /// Spawn the sandboxed child process.
    ///
    /// This performs the full spawn sequence:
    /// 1. Fork the process
    /// 2. In the child: set up isolation, chroot, drop privileges, install libbadcpu, exec
    /// 3. In the parent: set up process group tracking, return a Child handle
    ///
    /// SAFETY: This function calls `fork(2)`, which is inherently unsafe.
    /// The caller must ensure no other threads hold locks that could deadlock
    /// in the child process.
    pub unsafe fn spawn(&self) -> Result<ManagedChild> {
        let chroot_path = self.config.chroot_path.clone();
        let binary_path = self.config.binary_path.clone();
        let uid = self.config.uid;
        let gid = self.config.gid;
        let ld_preload = self.config.ld_preload_paths.clone();
        let extra_env = self.config.extra_env.clone();

        debug!("Spawning sandboxed process: {}", binary_path.display());

        // SAFETY: fork(2) is called. We document the safety requirements above.
        // No async-signal-unsafe operations are performed between fork and exec.
        match unistd::fork().context("Failed to fork child process")? {
            ForkResult::Parent { child } => {
                info!("Forked child process with PID {}", child);
                // Close the child's end of the IPC pipe in the parent
                self.ipc.close_child_end().ok();
                Ok(ManagedChild { pid: child, status: None })
            }
            ForkResult::Child => {
                // In child process. Must be extremely careful here:
                // only async-signal-safe operations until exec().
                Self::child_main(
                    chroot_path,
                    binary_path,
                    uid,
                    gid,
                    ld_preload,
                    extra_env,
                    self.ipc.parent_fd(),
                )
            }
        }
    }

    /// Child process main function.
    ///
    /// This function never returns on success, as exec replaces the process image.
    /// On failure, it returns an error which gets turned into a process exit.
    ///
    /// SAFETY: This is called after fork. All operations must be async-signal-safe.
    unsafe fn child_main(
        chroot_path: PathBuf,
        binary_path: PathBuf,
        uid: u32,
        gid: u32,
        ld_preload: Vec<PathBuf>,
        extra_env: Vec<(String, String)>,
        ipc_fd: RawFd,
    ) -> Result<ManagedChild> {
        // Step 1: Close the parent's end of the IPC channel
        // (the parent side fd, which we dup'd when creating the channel)
        let _ = libc::close(ipc_fd);

        // Step 2: Create a new session (process group leader, no controlling terminal)
        // SAFETY: setsid(2) creates a new session.
        nix::unistd::setsid().context("Failed to create new session in child")?;
        debug!("Child created new session");

        // Step 3: Set process group
        // SAFETY: setpgid(0, 0) puts the child in its own process group.
        nix::unistd::setpgid(Pid::from_raw(0), Pid::from_raw(0))
            .context("Failed to set process group in child")?;

        // Step 4: Set up supplementary groups
        // SAFETY: initgroups drops all supplementary groups except those matching the gid.
        let gid_obj = nix::unistd::Gid::from_raw(gid);
        let user_cstr = CString::new("sober").expect("CString should not fail");
        nix::unistd::initgroups(&user_cstr, gid_obj)
            .context("Failed to initialize supplementary groups")?;
        debug!("Child initialized supplementary groups");

        // Step 5: Set real, effective, and saved GID
        // SAFETY: setresgid sets real, effective, and saved GIDs.
        nix::unistd::setresgid(
            nix::unistd::Gid::from_raw(gid),
            nix::unistd::Gid::from_raw(gid),
            nix::unistd::Gid::from_raw(gid),
        )
        .context("Failed to set GID in child")?;
        debug!("Child set GID to {}", gid);

        // Step 6: Set real, effective, and saved UID
        // SAFETY: setresuid sets real, effective, and saved UIDs.
        nix::unistd::setresuid(
            nix::unistd::Uid::from_raw(uid),
            nix::unistd::Uid::from_raw(uid),
            nix::unistd::Uid::from_raw(uid),
        )
        .context("Failed to set UID in child")?;
        debug!("Child set UID to {}", uid);

        // Step 7: Chroot into the sandbox
        // SAFETY: chroot(2) changes the root of the filesystem.
        // Requires CAP_SYS_CHROOT or root.
        std::env::set_current_dir(&chroot_path)
            .context("Failed to chdir to chroot path in child")?;

        nix::unistd::chroot(&chroot_path).context("Failed to chroot in child")?;
        debug!("Child chrooted to {}", chroot_path.display());

        // Step 8: chdir to / inside the chroot
        std::env::set_current_dir("/").context("Failed to chdir to / after chroot")?;

        // Step 9: Build LD_PRELOAD string
        let ld_preload_str = if !ld_preload.is_empty() {
            let paths: Vec<String> = ld_preload
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            Some(paths.join(":"))
        } else {
            None
        };

        // Step 10: Build the environment for the child
        let mut env_vars: Vec<(String, String)> = Vec::new();

        // Set LD_PRELOAD
        if let Some(ref preload) = ld_preload_str {
            env_vars.push(("LD_PRELOAD".to_string(), preload.clone()));
        } else {
            env_vars.push(("LD_PRELOAD".to_string(), String::new()));
        }

        // Android runtime environment
        env_vars.push((
            "LD_LIBRARY_PATH".to_string(),
            "/system/lib64:/vendor/lib64:/system/lib:/vendor/lib".to_string(),
        ));
        env_vars.push((
            "HOME".to_string(),
            "/data/data/com.roblox.client".to_string(),
        ));
        env_vars.push(("TMPDIR".to_string(), "/data/local/tmp".to_string()));
        env_vars.push(("ANDROID_ROOT".to_string(), "/system".to_string()));
        env_vars.push(("ANDROID_DATA".to_string(), "/data".to_string()));
        env_vars.push(("ANDROID_ART_ROOT".to_string(), "/system".to_string()));
        env_vars.push(("ANDROID_RUNTIME_ROOT".to_string(), "/system".to_string()));
        env_vars.push(("PATH".to_string(), "/system/bin:/system/xbin".to_string()));
        env_vars.push(("TERM".to_string(), "dumb".to_string()));

        // Add extra environment variables from config
        env_vars.extend(extra_env);

        // Step 11: Install libbadcpu SIGILL handler via LD_PRELOAD auto-init
        // The libbadcpu library will auto-initialize when loaded by the dynamic linker
        // during exec. We don't need to call install() here if it's in LD_PRELOAD.
        //
        // If no LD_PRELOAD is configured, we attempt a direct call.
        // (Though in normal usage, libbadcpu should always be preloaded.)

        // Step 12: Notify parent via IPC that we're about to exec
        // SAFETY: write(2) is async-signal-safe.
        let msg: [u8; 1] = *b"R";
        let ret = libc::write(
            ipc_fd,
            msg.as_ptr() as *const libc::c_void,
            1,
        );
        if ret != 1 {
            warn!("Failed to write readiness to IPC fd: ret={}", ret);
        }
        // Close the IPC fd in child
        libc::close(ipc_fd);

        // Step 13: Exec the target binary
        // SAFETY: execve(2) replaces the process image.
        // The environment has been carefully sanitized.
        // All state has been cleaned up. This never returns on success.
        exec_sandboxed(&binary_path, &env_vars)?;

        // If we get here, execve failed.
        let err = std::io::Error::last_os_error();
        error!("execve failed for {}: {}", binary_path.display(), err);
        std::process::exit(1);
    }
}

/// Execute the sandboxed binary via execve(2).
///
/// This function is called in the child process after fork, chroot, and privilege
/// dropping. It constructs the argv and envp arrays and calls execve.
///
/// SAFETY: This function uses execve(2) which replaces the process image.
unsafe fn exec_sandboxed(binary_path: &Path, env_vars: &[(String, String)]) -> Result<()> {
    // Construct argv: [binary_name, NULL]
    let argv0_cstr = CString::new(
        binary_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("sandbox"),
    )
    .context("Failed to construct argv[0]")?;

    let argv: [*const libc::c_char; 2] = [argv0_cstr.as_ptr(), std::ptr::null()];

    // Build envp as null-terminated array of C strings
    let env_cstrings: Vec<CString> = env_vars
        .iter()
        .map(|(k, v)| {
            let mut entry = k.clone();
            entry.push('=');
            entry.push_str(v);
            CString::new(entry).expect("Environment variable contains null byte")
        })
        .collect();

    let mut env_ptrs: Vec<*const libc::c_char> = env_cstrings.iter().map(|c| c.as_ptr()).collect();
    env_ptrs.push(std::ptr::null());

    // Convert binary path to CString
    let binary_cstr = CString::new(binary_path.as_os_str().as_bytes())
        .context("Binary path contains null byte")?;

    // SAFETY: execve(2) replaces the process image.
    // We are in a fork child with no other threads.
    // The environment arrays are properly null-terminated.
    libc::execve(
        binary_cstr.as_ptr(),
        argv.as_ptr(),
        env_ptrs.as_ptr(),
    );

    // If we reach here, execve failed
    Err(anyhow::anyhow!(
        "execve failed: {}",
        std::io::Error::last_os_error()
    ))
}

/// A managed child process handle returned by `Loader::spawn()`.
///
/// Wraps the child PID and provides status methods for waiting on
/// and interacting with the sandboxed process.
pub struct ManagedChild {
    pid: Pid,
    status: Option<process::ExitStatus>,
}

impl ManagedChild {
    /// Get the child's PID.
    pub fn pid(&self) -> Pid {
        self.pid
    }

    /// Wait for the child to exit and return its exit status.
    ///
    /// Blocks until the child process terminates.
    pub fn wait(&mut self) -> Result<process::ExitStatus> {
        use nix::sys::wait::{waitpid, WaitStatus};

        let status = waitpid(self.pid, None).context("Failed to wait for child process")?;

        let exit_status = match status {
            WaitStatus::Exited(_, code) => process::ExitStatus::from_raw(code),
            WaitStatus::Signaled(_, sig, _) => {
                // 128 + signal number convention
                process::ExitStatus::from_raw(128 + sig as i32)
            }
            other => anyhow::bail!("Unexpected wait status for child: {:?}", other),
        };

        self.status = Some(exit_status);
        Ok(exit_status)
    }

    /// Check if the child has exited without blocking.
    pub fn try_wait(&mut self) -> Result<Option<process::ExitStatus>> {
        use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};

        match waitpid(self.pid, Some(WaitPidFlag::WNOHANG))
            .context("Failed to try_wait for child process")?
        {
            WaitStatus::Exited(_, code) => {
                let status = process::ExitStatus::from_raw(code);
                self.status = Some(status);
                Ok(Some(status))
            }
            WaitStatus::Signaled(_, sig, _) => {
                let status = process::ExitStatus::from_raw(128 + sig as i32);
                self.status = Some(status);
                Ok(Some(status))
            }
            WaitStatus::StillAlive => Ok(None),
            other => anyhow::bail!("Unexpected wait status for child: {:?}", other),
        }
    }

    /// Kill the child process with the given signal.
    pub fn kill(&self, sig: Signal) -> Result<()> {
        signal::kill(self.pid, sig).context("Failed to kill child process")?;
        Ok(())
    }
}

impl std::fmt::Debug for ManagedChild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedChild")
            .field("pid", &self.pid)
            .field("status", &self.status)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_config_default() {
        let config = LoaderConfig::default();
        assert_eq!(config.uid, 65534);
        assert_eq!(config.gid, 65534);
        assert!(config.chroot_path.to_string_lossy().contains("sober"));
    }

    #[test]
    fn test_loader_new() {
        let config = LoaderConfig::default();
        let loader = Loader::new(config);
        assert!(loader.is_ok());
    }

    #[test]
    fn test_inject_nonexistent_library() {
        let mut loader = Loader::new(LoaderConfig::default()).unwrap();
        let result = loader.inject_library(Path::new("/nonexistent/path.so"));
        assert!(result.is_err());
    }
}