mod ansi;
mod event_buffer;
mod line_editor;
mod pty;
mod server;
mod types;

use std::sync::Arc;
use anyhow::{Context, Result};
use tokio::sync::broadcast;
use crate::{event_buffer::EventBuffer, server::AppState, types::SseEvent};

pub struct Args {
    pub command: Vec<String>,
    pub port: Option<u16>,
}

pub fn parse_args(raw: Vec<String>) -> Result<Args> {
    let mut command = Vec::new();
    let mut port: Option<u16> = None;
    let mut i = 1; // skip binary name

    while i < raw.len() {
        if raw[i] == "--port" {
            i += 1;
            port = Some(raw.get(i)
                .context("--port requires a value")?
                .parse::<u16>()
                .context("--port must be a number 1-65535")?);
        } else {
            command.push(raw[i].clone());
        }
        i += 1;
    }

    if command.is_empty() {
        anyhow::bail!("Usage: ShellyMcShellface <command> [args...] [--port <port>]");
    }

    Ok(Args { command, port })
}

pub fn find_available_port(start: u16) -> Result<u16> {
    for port in start..=start.saturating_add(99) {
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Ok(port);
        }
    }
    anyhow::bail!(
        "No available port in range {}–{}",
        start,
        start.saturating_add(99)
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    let raw_args: Vec<String> = std::env::args().collect();
    let args = parse_args(raw_args).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    let (tx, _rx) = broadcast::channel::<SseEvent>(1024);
    let tx = Arc::new(tx);
    let event_buf = Arc::new(EventBuffer::new());

    let state = AppState {
        tx: Arc::clone(&tx),
        event_buf: Arc::clone(&event_buf),
    };

    let port = match args.port {
        Some(p) => p,
        None => find_available_port(7777).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        }),
    };
    eprintln!("Listening on http://localhost:{}", port);

    // Start HTTP server
    tokio::spawn(async move {
        if let Err(e) = server::run_server(state, port).await {
            eprintln!("Server error: {e}");
        }
    });

    // Brief pause to let the server bind before opening browser
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Open browser
    let url = format!("http://localhost:{}", port);
    if let Err(e) = webbrowser::open(&url) {
        eprintln!("Could not open browser: {e}");
    }

    // Run PTY session (blocking)
    let tx2 = Arc::clone(&tx);
    let buf2 = Arc::clone(&event_buf);
    let command = args.command.clone();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = pty::run_pty_session(command, tx2, buf2) {
            eprintln!("PTY error: {e}");
        }
    })
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_extracts_command() {
        let args = parse_args(vec![
            "ShellyMcShellface".into(),
            "echo".into(),
            "hello".into(),
        ]).unwrap();
        assert_eq!(args.command, vec!["echo", "hello"]);
        assert_eq!(args.port, None);
    }

    #[test]
    fn test_parse_args_custom_port() {
        let args = parse_args(vec![
            "ShellyMcShellface".into(),
            "--port".into(),
            "8080".into(),
            "claude".into(),
        ]).unwrap();
        assert_eq!(args.port, Some(8080));
        assert_eq!(args.command, vec!["claude"]);
    }

    #[test]
    fn test_parse_args_port_after_command() {
        let args = parse_args(vec![
            "ShellyMcShellface".into(),
            "ssh".into(),
            "user@server".into(),
            "--port".into(),
            "9000".into(),
        ]).unwrap();
        assert_eq!(args.command, vec!["ssh", "user@server"]);
        assert_eq!(args.port, Some(9000));
    }

    #[test]
    fn test_parse_args_no_command_returns_error() {
        let result = parse_args(vec!["ShellyMcShellface".into()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_available_port_returns_port_in_range() {
        let port = find_available_port(10000).expect("should find a free port near 10000");
        assert!(port >= 10000, "port should be >= start");
        assert!(port <= 10099, "port should be within 100-port window");
    }

    #[test]
    fn test_find_available_port_skips_occupied_port() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let occupied = listener.local_addr().unwrap().port();
        let found = find_available_port(occupied).expect("should find a free port");
        assert_ne!(found, occupied, "should skip the occupied port");
        // listener held open so occupied port stays taken for the duration of the test
    }
}
