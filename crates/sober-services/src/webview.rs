// SPDX-License-Identifier: MIT
//
// sober-services — Browser-based OAuth login handler
//
// Instead of embedding a WebKit webview (which would require gtk3 webkit2gtk
// or raw FFI bindings for webkit2gtk 4.1), we open the system browser for
// Roblox OAuth and run a local HTTP server to capture the redirect callback.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use crate::ServiceConfig;

/// Auth result captured from browser redirect.
#[derive(Debug, Clone)]
pub struct AuthResult {
    pub token: String,
    pub user_id: Option<u64>,
    pub username: Option<String>,
}

/// Manages browser-based OAuth login.
pub struct LoginWebview {
    auth_token: Arc<std::sync::Mutex<Option<String>>>,
    server_running: Arc<AtomicBool>,
    port: u16,
}

impl LoginWebview {
    /// Create a new login handler.
    /// This does NOT start the server yet — call `start_server()` first.
    pub fn new(_config: &ServiceConfig) -> Result<Self> {
        Ok(Self {
            auth_token: Arc::new(std::sync::Mutex::new(None)),
            server_running: Arc::new(AtomicBool::new(false)),
            port: 0,
        })
    }

    /// Start the local redirect HTTP server on a random port.
    /// Returns the port number the server is listening on.
    pub fn start_server(&mut self) -> Result<u16> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .context("Failed to bind local HTTP server for OAuth callback")?;
        let port = listener.local_addr()?.port();
        self.port = port;

        info!("OAuth callback server listening on http://127.0.0.1:{}", port);

        let server_running = self.server_running.clone();
        let token = self.auth_token.clone();

        thread::spawn(move || {
            server_running.store(true, Ordering::SeqCst);
            for stream in listener.incoming() {
                if !server_running.load(Ordering::SeqCst) {
                    break;
                }
                match stream {
                    Ok(stream) => {
                        if let Ok(Some(t)) = Self::handle_callback(stream) {
                            info!("OAuth callback received token");
                            let mut tok = token.lock().unwrap();
                            *tok = Some(t);
                        }
                    }
                    Err(e) => {
                        warn!("OAuth server accept error: {}", e);
                    }
                }
            }
        });

        Ok(port)
    }

    /// Open the system browser to start the OAuth flow.
    pub fn open_browser(&self, auth_url: &str) -> Result<()> {
        // Append the redirect URI pointing to our local server
        let redirect_uri = format!("http://127.0.0.1:{}/callback", self.port);
        let full_url = if auth_url.contains('?') {
            format!("{}&redirect_uri={}", auth_url, urlencoding(&redirect_uri))
        } else {
            format!("{}?redirect_uri={}", auth_url, urlencoding(&redirect_uri))
        };

        info!("Opening browser for Roblox login: {}", &full_url[..full_url.len().min(80)]);

        // Use xdg-open to open the system browser
        let status = std::process::Command::new("xdg-open")
            .arg(&full_url)
            .status()
            .context("Failed to open system browser (xdg-open not available)")?;

        if !status.success() {
            anyhow::bail!("xdg-open exited with error: {}", status);
        }

        Ok(())
    }

    /// Handle an HTTP callback request. Parses the redirect URI for the token.
    fn handle_callback(mut stream: TcpStream) -> Result<Option<String>> {
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf)?;
        let request = String::from_utf8_lossy(&buf[..n]);

        // Parse the HTTP request line to extract the path and query
        let (method, path, _version) = parse_request_line(&request);

        if method != "GET" {
            send_http_response(&stream, 405, "Method Not Allowed")?;
            return Ok(None);
        }

        // Extract token from query parameters
        let token = extract_auth_token(path);
        debug!("OAuth callback path: {} -> token: {:?}", path, token.is_some());

        if let Some(ref t) = token {
            let html = format!(
                "<html><body><h1>Authentication successful!</h1>\
                 <p>You can close this window and return to Open Sober.</p>\
                 <script>window.close()</script></body></html>"
            );
            send_http_response(&stream, 200, &html)?;
            info!("Auth token captured successfully: {}...{}", &t[..t.len().min(8)], &t[t.len().saturating_sub(4)..]);
            Ok(token)
        } else {
            let html = format!(
                "<html><body><h1>Waiting for authentication...</h1>\
                 <p>If you were redirected here, the login is still in progress.</p>\
                 <p>Path received: {}</p></body></html>",
                path
            );
            send_http_response(&stream, 200, &html)?;
            Ok(None)
        }
    }

    /// Check if a token has been received.
    pub fn has_token(&self) -> bool {
        self.auth_token.lock().unwrap().is_some()
    }

    /// Get the received token, if any.
    pub fn get_token(&self) -> Option<String> {
        self.auth_token.lock().unwrap().clone()
    }

    /// Block until a token is received (with configurable timeout).
    pub fn wait_for_token(&self, timeout_secs: u64) -> Option<String> {
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < timeout_secs {
            if let Some(token) = self.get_token() {
                return Some(token);
            }
            thread::sleep(std::time::Duration::from_millis(100));
        }
        None
    }

    /// Create a placeholder widget that does nothing (for API compatibility).
    /// The real webview is the browser.
    pub fn container(&self) -> Option<()> { None }
    pub fn widget(&self) -> Option<()> { None }
    pub fn load(&self, _url: &str) {
        // No-op — we open in system browser instead
    }
    pub fn reload(&self) {}
}

impl Drop for LoginWebview {
    fn drop(&mut self) {
        self.server_running.store(false, Ordering::SeqCst);
        info!("Login webview destroyed");
    }
}

// --- HTTP helpers ---

fn parse_request_line(request: &str) -> (&str, &str, &str) {
    let first_line = request.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
    (
        parts.first().copied().unwrap_or(""),
        parts.get(1).copied().unwrap_or("/"),
        parts.get(2).copied().unwrap_or(""),
    )
}

fn extract_auth_token(path: &str) -> Option<String> {
    // Check for fragment-based token first (most common in OAuth implicit flow)
    if let Some(pos) = path.find("#access_token=") {
        let token = path[pos + "#access_token=".len()..]
            .split('&')
            .next()
            .unwrap_or("")
            .to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }

    // Parse query parameters
    let query = if let Some(pos) = path.find('?') {
        &path[pos + 1..]
    } else {
        return None;
    };

    for param in query.split('&') {
        let mut parts = param.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();

        match key {
            "code" | "access_token" | "token" => {
                let token = urlencoding_decode(value);
                if !token.is_empty() {
                    return Some(token);
                }
            }
            _ => {}
        }
    }
    None
}

fn send_http_response(mut stream: &TcpStream, status: u16, body: &str) -> Result<()> {
    let status_line = match status {
        200 => "200 OK",
        405 => "405 Method Not Allowed",
        _ => "200 OK",
    };
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status_line,
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_auth_token_query() {
        let token = extract_auth_token("/callback?code=abc123&state=xyz");
        assert_eq!(token, Some("abc123".to_string()));
    }

    #[test]
    fn test_extract_fragment_token() {
        let token = extract_auth_token("/callback#access_token=token123&expires_in=3600");
        assert_eq!(token, Some("token123".to_string()));
    }

    #[test]
    fn test_no_token() {
        let token = extract_auth_token("/home");
        assert!(token.is_none());
    }

    #[test]
    fn test_url_roundtrip() {
        let original = "http://127.0.0.1:8080/callback?code=abc&redirect_uri=http://localhost";
        let encoded = urlencoding(original);
        let decoded = urlencoding_decode(&encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_parse_request_line() {
        let (method, path, version) = parse_request_line("GET /callback?code=abc HTTP/1.1\r\nHost: localhost");
        assert_eq!(method, "GET");
        assert_eq!(path, "/callback?code=abc");
        assert_eq!(version, "HTTP/1.1");
    }
}