# Design: Ctrl+Q Exit and Auto-Port Selection

**Date:** 2026-03-25
**Status:** Approved

## Overview

Two independent features:
1. **Ctrl+Q exit** — intercept Ctrl+Q in the terminal and cleanly end the session
2. **Auto-port selection** — find the next available port starting from 7777 so multiple instances can run simultaneously without manually specifying `--port`

---

## Feature 1: Ctrl+Q Exit

### Problem

There is currently no way to quit ShellyMcShellface from the terminal without closing the terminal window. Typing into the terminal sends keystrokes directly to the PTY child. The only natural exit path is waiting for the wrapped program to exit on its own.

### Design

**`src/types.rs`**

Add a `UserQuit` variant to `SseEvent`:

```rust
SseEvent::Status { state: "user_quit", code: None }
```

Add a `user_quit()` constructor alongside the existing `connected()`, `pty_exited()`, etc.

**`src/pty.rs`**

Share the child process handle between `run_pty_session` and the stdin thread using `Arc<Mutex<Option<Box<dyn portable_pty::Child + Send>>>>`.

In the stdin thread, before forwarding each byte to the PTY writer, check for `0x11` (Ctrl+Q):

1. Emit `SseEvent::user_quit()` via the broadcast channel and event buffer
2. Lock the child handle and call `.kill()`
3. Call `crossterm::terminal::disable_raw_mode()`
4. Call `std::process::exit(0)`

The `portable_pty::Child` trait requires `Send` for the `Arc<Mutex>` wrapper to be movable into the stdin thread. If the trait is not `Send`, wrap in a newtype or use `unsafe impl Send` with a comment explaining the single-writer guarantee.

**`src/frontend/app.js`**

In `handleStatus`, handle `payload.state === 'user_quit'`:
- Set status text to "Session ended"
- Set `data-state` to `"exited"`
- Do **not** show the Reconnect button

### Behaviour

- Ctrl+Q is consumed by ShellyMcShellface and never forwarded to the PTY child
- The wrapped program is killed (SIGTERM/TerminateProcess) immediately
- The terminal is restored to cooked mode before the process exits
- The browser shows "Session ended" with no reconnect prompt

---

## Feature 2: Auto-Port Selection

### Problem

Running multiple instances requires manually specifying `--port 7778`, `--port 7779`, etc. There is no way to just launch a new instance and have it find a free port automatically.

### Design

**`src/main.rs`**

Add a `find_available_port(start: u16) -> u16` function. When `--port` is **not** specified, call it with the default start of `7777`. Try binding a `std::net::TcpListener` on `127.0.0.1:<port>` for each port in `start..=start+99`. Return the first that succeeds. Panic (with a clear message) if none succeed within that range.

When `--port` **is** specified explicitly, skip auto-detection and pass the port directly to the server. If that port is taken, axum will fail to bind and the error propagates as before.

Print the resolved URL to stderr before opening the browser:
```
Listening on http://localhost:7778
```

Pass the resolved port to both `server::run_server` and `webbrowser::open`.

### Behaviour

- `ShellyMcShellface.exe powershell` → binds 7777 (or first free port above it)
- Running a second instance → binds 7778 automatically
- `ShellyMcShellface.exe powershell --port 8080` → unchanged: binds 8080 or fails fast
- Port scan is capped at 100 ports above the start to avoid unbounded scanning

---

## Testing

**Ctrl+Q:**
- Unit test: `process_stdin_bytes` (or equivalent) with `0x11` in input — verify `user_quit` event is emitted
- Manual: press Ctrl+Q during a running session, verify terminal restores and browser shows "Session ended"

**Auto-port:**
- Unit test: `find_available_port` with a start port that is already bound — verify it returns the next available port
- Unit test: all ports in range taken — verify it panics with a clear message
- Manual: launch two instances without `--port`, verify they bind to different ports
