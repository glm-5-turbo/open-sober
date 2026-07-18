// SPDX-License-Identifier: MIT
//
// IPC channel between parent and child processes.
//
// Uses a Unix pipe (socketpair) for bidirectional communication
// between the loader (parent) and the sandboxed child process.
// The primary use is the child notifying the parent of its readiness
// before executing the target binary.

use std::os::unix::io::RawFd;

use anyhow::Result;
use tracing::debug;

/// A simple IPC channel implemented over a Unix pipe.
///
/// After fork, one end is used by the parent and the other by the child.
/// The parent end is typically closed in the child, and vice versa.
pub struct IpcChannel {
    /// File descriptors for the pipe: [0] = read end, [1] = write end.
    /// By convention, fd[0] is the parent end, fd[1] is the child end.
    fds: [RawFd; 2],
}

impl IpcChannel {
    /// Create a new IPC channel.
    ///
    /// Internally creates a Unix pipe(2) with two file descriptors.
    pub fn new() -> Result<Self> {
        // SAFETY: pipe(2) creates a unidirectional data channel.
        // pipe2 with O_CLOEXEC ensures the fds are closed on exec.
        let fds = unsafe {
            let mut pipe_fds: [libc::c_int; 2] = [0, 0];
            let ret = libc::pipe2(pipe_fds.as_mut_ptr(), libc::O_CLOEXEC);
            if ret != 0 {
                anyhow::bail!("pipe2 failed: {}", std::io::Error::last_os_error());
            }
            pipe_fds
        };

        debug!("IPC channel created: read={}, write={}", fds[0], fds[1]);
        Ok(Self { fds })
    }

    /// Get the file descriptor that should be used by the parent.
    pub fn parent_fd(&self) -> RawFd {
        self.fds[0]
    }

    /// Get the file descriptor that should be used by the child.
    pub fn child_fd(&self) -> RawFd {
        self.fds[1]
    }

    /// Close the parent's end of the pipe.
    /// Should be called in the child process after fork.
    pub fn close_parent_end(&self) -> Result<()> {
        // SAFETY: close(2) is async-signal-safe.
        unsafe {
            libc::close(self.fds[0]);
        }
        Ok(())
    }

    /// Close the child's end of the pipe.
    /// Should be called in the parent process after fork.
    pub fn close_child_end(&self) -> Result<()> {
        // SAFETY: close(2) is async-signal-safe.
        unsafe {
            libc::close(self.fds[1]);
        }
        Ok(())
    }

    /// Send a readiness signal from child to parent.
    ///
    /// Writes a single byte to the pipe. The parent blocks on read
    /// until this byte arrives.
    pub fn send_ready(&self) -> Result<()> {
        let byte: [u8; 1] = *b"R";
        // SAFETY: write(2) is async-signal-safe.
        unsafe {
            let ret = libc::write(self.fds[1], byte.as_ptr() as *const libc::c_void, 1);
            if ret != 1 {
                anyhow::bail!("Failed to send readiness signal");
            }
        }
        Ok(())
    }

    /// Wait for the readiness signal from the child.
    ///
    /// Blocks the parent until the child writes the readiness byte.
    /// Returns `true` if the child is ready, `false` if the pipe closed.
    pub fn wait_ready(&self) -> Result<bool> {
        let mut byte: [u8; 1] = [0];
        // SAFETY: read(2) is safe.
        unsafe {
            let ret = libc::read(self.fds[0], byte.as_mut_ptr() as *mut libc::c_void, 1);
            if ret == 1 {
                Ok(byte[0] == b'R')
            } else if ret == 0 {
                // EOF means the child closed the pipe without sending ready
                Ok(false)
            } else {
                anyhow::bail!("Failed to read readiness signal: {}", std::io::Error::last_os_error());
            }
        }
    }

    /// Send a data message from parent to child.
    pub fn send_message(&self, data: &[u8]) -> Result<()> {
        // SAFETY: write(2) is async-signal-safe.
        unsafe {
            let written = libc::write(self.fds[1], data.as_ptr() as *const libc::c_void, data.len());
            if written < 0 {
                anyhow::bail!("Failed to write IPC message: {}", std::io::Error::last_os_error());
            }
        }
        Ok(())
    }

    /// Receive a data message on the parent end.
    pub fn receive_message(&self, buf: &mut [u8]) -> Result<usize> {
        // SAFETY: read(2) is safe.
        unsafe {
            let n = libc::read(self.fds[0], buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n < 0 {
                anyhow::bail!("Failed to read IPC message: {}", std::io::Error::last_os_error());
            }
            Ok(n as usize)
        }
    }
}

impl Drop for IpcChannel {
    fn drop(&mut self) {
        // SAFETY: close(2) is safe to call from Drop.
        unsafe {
            if self.fds[0] >= 0 {
                libc::close(self.fds[0]);
            }
            if self.fds[1] >= 0 && self.fds[1] != self.fds[0] {
                libc::close(self.fds[1]);
            }
        }
    }
}

// IpcChannel is not Send/Sync by default since RawFd is !Send.
// The channel is designed to be used in one process at a time.
unsafe impl Send for IpcChannel {}
unsafe impl Sync for IpcChannel {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_channel_create() {
        let ch = IpcChannel::new().unwrap();
        assert!(ch.parent_fd() >= 0);
        assert!(ch.child_fd() >= 0);
        assert_ne!(ch.parent_fd(), ch.child_fd());
    }

    #[test]
    fn test_ipc_roundtrip() {
        let ch = IpcChannel::new().unwrap();
        ch.send_ready().unwrap();
        let ready = ch.wait_ready().unwrap();
        assert!(ready);
    }

    #[test]
    fn test_ipc_message() {
        let ch = IpcChannel::new().unwrap();
        let msg = b"hello";
        ch.send_message(msg).unwrap();
        let mut buf = [0u8; 16];
        let n = ch.receive_message(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf[..5], b"hello");
    }
}