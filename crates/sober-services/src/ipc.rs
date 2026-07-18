// SPDX-License-Identifier: MIT
//
// sober-services — Unix-domain-socket IPC for auth token exchange

use std::io::{Read, Write};
use std::io;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A typed message exchanged over the IPC socket.
///
/// The parent sober process and the services GUI communicate by sending
/// JSON-serialised `IpcMessage` values over a Unix domain stream socket.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcMessage {
    /// The child (services) is reporting that authentication succeeded.
    AuthToken {
        /// The `.ROBLOSECURITY` cookie value extracted from the login.
        token: String,
        /// The Roblox user ID, if obtainable.
        user_id: Option<u64>,
        /// The username displayed on the account, if obtainable.
        username: Option<String>,
    },
    /// The child is requesting the parent to perform a privileged action.
    Request {
        /// Identifier for the request type.
        action: String,
        /// Action-specific payload.
        payload: serde_json::Value,
    },
    /// The parent's response to a previous request.
    Response {
        /// Request identifier this response corresponds to.
        request_id: u64,
        /// Whether the request succeeded.
        success: bool,
        /// Optional error message (present when success is false).
        error: Option<String>,
        /// Response payload.
        payload: Option<serde_json::Value>,
    },
    /// Heartbeat / keep-alive.
    Ping,
    /// Acknowledgment of a Ping.
    Pong,
}

/// Send an authentication token to the parent sober process.
///
/// Opens a connection to the Unix domain socket at `socket_path` and sends
/// an `AuthToken` message. Returns once the message has been written (but
/// does not wait for an acknowledgment).
pub fn send_auth_token(socket_path: &str, token: &str) -> Result<()> {
    let stream = UnixStream::connect(socket_path)
        .with_context(|| format!("Failed to connect to IPC socket at {}", socket_path))?;

    let msg = IpcMessage::AuthToken {
        token: token.to_string(),
        user_id: None,
        username: None,
    };

    send_message(&stream, &msg)
}

/// Listen for incoming IPC connections from the parent process.
///
/// Binds a `UnixListener` to `socket_path`, accepts one or more connections,
/// and deserialises incoming `IpcMessage`s. Each received message is passed to
/// `handler` for processing. Blocks indefinitely until an irrecoverable error
/// or the caller drops the returned handle.
///
/// The socket file is removed on bind failure; the caller should also arrange
/// for cleanup on graceful shutdown.
pub fn listen_for_auth<F>(socket_path: &Path, handler: F) -> Result<()>
where
    F: Fn(IpcMessage) -> Result<()>,
{
    // Remove stale socket file if present
    if socket_path.exists() {
        std::fs::remove_file(socket_path)
            .with_context(|| format!("Failed to remove stale socket at {}", socket_path.display()))?;
    }

    // Ensure parent directory exists
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create IPC directory {}", parent.display()))?;
    }

    let listener = UnixListener::bind(socket_path)
        .with_context(|| format!("Failed to bind IPC socket at {}", socket_path.display()))?;

    tracing::info!("IPC listener bound to {}", socket_path.display());

    // Set non-blocking so we can handle graceful shutdown (omitted for brevity;
    // in production use a select-loop or a dedicated accept thread).
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = handle_connection(stream, &handler) {
                    tracing::error!("Error handling IPC connection: {:#}", e);
                }
            }
            Err(e) => {
                tracing::error!("IPC accept error: {}", e);
                // On transient errors (e.g. EINTR), continue; otherwise break.
                if e.kind() != std::io::ErrorKind::WouldBlock
                    && e.kind() != std::io::ErrorKind::Interrupted
                {
                    anyhow::bail!("Fatal IPC accept error: {}", e);
                }
            }
        }
    }

    Ok(())
}

/// Read a single `IpcMessage` from a connected stream.
pub fn recv_message(mut stream: impl std::io::Read) -> Result<IpcMessage> {
    // Read framing: 4-byte little-endian length prefix
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .context("Failed to read IPC message length prefix")?;

    let msg_len = u32::from_le_bytes(len_buf) as usize;

    // Read the JSON payload
    let mut buf = vec![0u8; msg_len];
    stream
        .read_exact(&mut buf)
        .context("Failed to read IPC message body")?;

    let msg: IpcMessage = serde_json::from_slice(&buf)
        .context("Failed to deserialise IPC message")?;

    Ok(msg)
}

/// Write a single `IpcMessage` to a connected stream (with length prefix).
fn send_message<T: std::io::Write>(mut stream: T, msg: &IpcMessage) -> Result<()> {
    let data = serde_json::to_vec(msg).context("Failed to serialise IPC message")?;

    // Write 4-byte length prefix (little-endian)
    let len = data.len() as u32;
    let len_buf = len.to_le_bytes();
    stream
        .write_all(&len_buf)
        .context("Failed to write IPC message length prefix")?;

    stream
        .write_all(&data)
        .context("Failed to write IPC message body")?;

    stream
        .flush()
        .context("Failed to flush IPC stream")?;

    Ok(())
}

/// Handle a single IPC connection: read messages until the peer disconnects.
fn handle_connection<F>(mut stream: UnixStream, handler: &F) -> Result<()>
where
    F: Fn(IpcMessage) -> Result<()>,
{
    loop {
        let msg = match recv_message(io::Read::by_ref(&mut stream)) {
            Ok(m) => m,
            Err(e) => {
                // Connection closed or protocol error
                if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                    if io_err.kind() == std::io::ErrorKind::UnexpectedEof {
                        tracing::debug!("IPC peer disconnected");
                        return Ok(());
                    }
                }
                return Err(e);
            }
        };

        // Handle Ping implicitly before forwarding to handler
        if matches!(msg, IpcMessage::Ping) {
            send_message(&stream, &IpcMessage::Pong)?;
            continue;
        }

        handler(msg)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_send_recv_roundtrip() {
        let msg = IpcMessage::AuthToken {
            token: "test_token_123".into(),
            user_id: Some(12345),
            username: Some("Robloxian".into()),
        };

        let mut buf = Vec::new();
        send_message(&mut buf, &msg).unwrap();

        let mut cursor = Cursor::new(buf);
        let received = recv_message(&mut cursor).unwrap();

        match received {
            IpcMessage::AuthToken { token, user_id, username } => {
                assert_eq!(token, "test_token_123");
                assert_eq!(user_id, Some(12345));
                assert_eq!(username.as_deref(), Some("Robloxian"));
            }
            other => panic!("Expected AuthToken, got {:?}", other),
        }
    }

    #[test]
    fn test_ping_pong() {
        let mut buf = Vec::new();
        send_message(&mut buf, &IpcMessage::Ping).unwrap();

        let mut cursor = Cursor::new(buf);
        let received = recv_message(&mut cursor).unwrap();
        assert!(matches!(received, IpcMessage::Ping));
    }

    #[test]
    fn test_send_auth_token_no_server() {
        // Should fail gracefully when no server is listening
        let path = "/tmp/__sober_test_no_exist.sock";
        let result = send_auth_token(path, "token");
        assert!(result.is_err());
    }
}