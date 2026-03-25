# ShellyMcShellface

A Rust CLI tool that wraps any terminal command in a PTY and streams all output to a local browser page via Server-Sent Events. Input stays in the terminal; the browser is the sole display.

## Why

PowerShell stops responding with large terminal output, and plain terminal output lacks semantic structure for screen reader users. ShellyMcShellface sends output to a browser page that groups each command's output under an accessible `<details>`/`<summary>` heading and maintains an `aria-live` region for incremental announcements.

## Usage

```
ShellyMcShellface.exe <command> [args...] [--port <port>]
```

Examples:

```
ShellyMcShellface.exe claude
ShellyMcShellface.exe powershell --port 8080
```

The browser page opens automatically. Type commands in the terminal as normal; output appears grouped in the browser.

## Build

Requires Rust (stable).

```
cargo build --release
```

The binary is at `target/release/ShellyMcShellface.exe`.

## Test

```
cargo test
```

41 tests covering the ANSI parser, PTY output processor, line editor, SSE event formatting, and server state.

## Tech Stack

- **Rust + Tokio** — async runtime
- **portable-pty** — cross-platform PTY
- **axum 0.8** — HTTP server and SSE
- **crossterm** — terminal raw mode
- Vanilla HTML/CSS/JS frontend embedded via `include_str!`
