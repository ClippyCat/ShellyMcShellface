# Exit and Auto-Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Ctrl+Q to cleanly exit any running session, and auto-detect the next free port from 7777 so multiple instances run side-by-side without manual `--port` flags.

**Architecture:** Ctrl+Q is intercepted in the stdin thread; `Child::clone_killer()` provides a `Send + Sync` kill handle that can be moved into that thread without sharing the full child. Auto-port uses a synchronous `TcpListener::bind` probe loop in `main` before the server task is spawned.

**Tech Stack:** Rust, `portable_pty` 0.8 (`Child`, `ChildKiller` traits), `crossterm`, `axum`, vanilla JS.

---

## Files Changed

| File | Change |
|------|--------|
| `src/types.rs` | Add `UserQuit` to `StatusState`; add `user_quit()` constructor |
| `src/pty.rs` | Intercept `0x11` in stdin thread; use `clone_killer()` to kill child |
| `src/frontend/app.js` | Handle `user_quit` status: "Session ended", hide Reconnect button |
| `src/main.rs` | Change `Args.port` to `Option<u16>`; add `find_available_port`; resolve port before server spawn |

---

## Task 1: Add `UserQuit` to `SseEvent`

**Files:**
- Modify: `src/types.rs`

- [ ] **Step 1: Write the failing test**

Add this test to the `#[cfg(test)]` block in `src/types.rs`, after the existing `test_status_pty_exited_code_zero_has_no_code_field` test:

```rust
#[test]
fn test_status_user_quit_serialises() {
    let ev = SseEvent::user_quit();
    let (event_type, data) = ev.to_sse_parts();
    assert_eq!(event_type, "status");
    let json: serde_json::Value = serde_json::from_str(&data).unwrap();
    assert_eq!(json["state"], "user_quit");
    assert_eq!(json["code"], serde_json::Value::Null);
}
```

- [ ] **Step 2: Run test to confirm it fails**

```
cargo test test_status_user_quit_serialises
```

Expected: compile error — `user_quit` method does not exist.

- [ ] **Step 3: Add `UserQuit` variant to `StatusState`**

In `src/types.rs`, extend the `StatusState` enum:

```rust
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusState {
    Connected,
    PtyExited,
    PtyError,
    UserQuit,
}
```

Then add the constructor to `impl SseEvent`, after `pty_error()`:

```rust
pub fn user_quit() -> Self {
    SseEvent::Status { state: StatusState::UserQuit, code: None }
}
```

- [ ] **Step 4: Run the test to confirm it passes**

```
cargo test test_status_user_quit_serialises
```

Expected: PASS.

- [ ] **Step 5: Run full test suite**

```
cargo test
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```
git add src/types.rs
git commit -m "feat: add UserQuit status variant to SseEvent"
```

---

## Task 2: Intercept Ctrl+Q in the stdin thread

**Files:**
- Modify: `src/pty.rs`

The `Child::clone_killer()` method returns a `Box<dyn ChildKiller + Send + Sync>` — a standalone kill handle that is safe to move into the stdin thread. We call it after spawning the child and before creating the stdin thread, so the main function continues to own `child` for `child.wait()`.

- [ ] **Step 1: Add `ChildKiller` to the portable_pty imports**

In `src/pty.rs`, find the `use portable_pty::{...}` line inside `run_pty_session` and add `ChildKiller`:

```rust
use portable_pty::{native_pty_system, CommandBuilder, PtySize, ChildKiller};
```

- [ ] **Step 2: Get the kill handle after spawning the child**

`clone_killer()` is a method on the `Child` trait. In `run_pty_session`, directly after the `emit(SseEvent::connected(), ...)` call, add:

```rust
emit(SseEvent::connected(), &tx, &buf);

let mut child_killer = child.clone_killer();
```

The full block at this point looks like:

```rust
let mut child = pair.slave.spawn_command(cmd).map_err(|e| {
    let tx2 = Arc::clone(&tx);
    let buf2 = Arc::clone(&buf);
    emit(SseEvent::pty_error(), &tx2, &buf2);
    e
})?;

emit(SseEvent::connected(), &tx, &buf);

let mut child_killer = child.clone_killer();

// Stdin thread: raw mode, forward keystrokes to PTY, feed LineEditor
let mut pty_writer = pair.master.take_writer()?;
```

- [ ] **Step 3: Replace the stdin thread body to intercept `0x11`**

Replace the entire `std::thread::spawn(move || -> Result<()> { ... });` block with:

```rust
std::thread::spawn(move || -> Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    let stdin = std::io::stdin();
    let mut line_editor = LineEditor::new();
    let mut raw_buf = [0u8; 256];
    loop {
        let n = stdin.lock().read(&mut raw_buf)?;
        if n == 0 { break; }

        let mut quit = false;
        for &byte in &raw_buf[..n] {
            if byte == 0x11 { // Ctrl+Q
                quit = true;
                break;
            }
            if let Some(text) = line_editor.feed(byte) {
                emit(SseEvent::input(text), &tx_stdin, &buf_stdin);
            }
        }

        if quit {
            emit(SseEvent::user_quit(), &tx_stdin, &buf_stdin);
            let _ = child_killer.kill();
            let _ = crossterm::terminal::disable_raw_mode();
            std::process::exit(0);
        }

        let _ = pty_writer.write_all(&raw_buf[..n]);
    }
    Ok(())
});
```

Note: `child_killer` is moved into the closure. The `quit` flag breaks out of the byte loop so Ctrl+Q bytes are never written to the PTY.

- [ ] **Step 4: Run the full test suite**

```
cargo test
```

Expected: all tests pass. (The Ctrl+Q exit path calls `process::exit(0)` so it is not unit-tested directly — correctness is verified in Step 5 manually.)

- [ ] **Step 5: Manual smoke test**

Build and run:

```
cargo build --release
./target/release/ShellyMcShellface.exe powershell
```

Press Ctrl+Q. Expected:
- Terminal restores to normal (no raw mode artefacts)
- Browser status bar shows "Session ended" (after Task 3)
- Process exits

- [ ] **Step 6: Commit**

```
git add src/pty.rs
git commit -m "feat: intercept Ctrl+Q to kill child and exit cleanly"
```

---

## Task 3: Handle `user_quit` in the browser

**Files:**
- Modify: `src/frontend/app.js`

- [ ] **Step 1: Add `user_quit` case to `handleStatus`**

In `src/frontend/app.js`, find the `function handleStatus(payload)` function:

```js
function handleStatus(payload) {
    if (payload.state === 'connected') {
      setStatus('Connected', 'connected');
    } else if (payload.state === 'pty_exited') {
      if (payload.code && payload.code !== 0) {
        setStatus(`Process exited with error (code ${payload.code})`, 'exited');
      } else {
        setStatus('Process exited', 'exited');
      }
    } else if (payload.state === 'pty_error') {
      setStatus('Process failed to start', 'error');
    }
  }
```

Replace it with:

```js
function handleStatus(payload) {
    if (payload.state === 'connected') {
      setStatus('Connected', 'connected');
    } else if (payload.state === 'pty_exited') {
      if (payload.code && payload.code !== 0) {
        setStatus(`Process exited with error (code ${payload.code})`, 'exited');
      } else {
        setStatus('Process exited', 'exited');
      }
    } else if (payload.state === 'pty_error') {
      setStatus('Process failed to start', 'error');
    } else if (payload.state === 'user_quit') {
      setStatus('Session ended', 'exited');
    }
  }
```

The Reconnect button is shown only in `es.onerror`. Since `user_quit` kills the process, the SSE connection drops and `onerror` fires after we set status. To prevent the Reconnect button appearing after a user quit, add a flag:

- [ ] **Step 2: Add a `userQuit` flag to suppress the Reconnect button**

Near the top of the IIFE, after the `let skipNextOutput = false;` declaration, add:

```js
let userQuit = false;
```

In `handleStatus`, set it on `user_quit`:

```js
} else if (payload.state === 'user_quit') {
  userQuit = true;
  setStatus('Session ended', 'exited');
}
```

In `es.onerror`, check the flag before showing the Reconnect button. Replace the `es.onerror` handler:

```js
es.onerror = () => {
  es.close(); // suppress automatic retry — would duplicate all DOM content
  if (userQuit) return; // process was intentionally ended
  setStatus('Disconnected', 'disconnected');
  reconnectBtn.hidden = false;
  announcer.textContent = 'Connection lost. Reconnect button is now available in the footer.';
};
```

- [ ] **Step 3: Run full test suite**

```
cargo test
node tests/frontend/ansi.test.js
```

Expected: all tests pass.

- [ ] **Step 4: Manual smoke test**

Build and run, then press Ctrl+Q in the terminal. Expected in browser:
- Status bar shows "Session ended"
- Reconnect button does NOT appear

- [ ] **Step 5: Commit**

```
git add src/frontend/app.js
git commit -m "feat: show 'Session ended' on user_quit, suppress reconnect button"
```

---

## Task 4: Auto-port selection

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write the failing tests**

In `src/main.rs`, in the `#[cfg(test)]` block, add these three new tests after the existing ones. Also update the three existing port tests (they will fail once `Args.port` becomes `Option<u16>`):

**New tests to add:**

```rust
#[test]
fn test_find_available_port_returns_bindable_port() {
    let port = find_available_port(10000);
    assert!(port >= 10000);
    assert!(port <= 10099);
    // Verify the returned port is actually bindable
    let result = std::net::TcpListener::bind(("127.0.0.1", port));
    assert!(result.is_ok(), "returned port {} should be bindable", port);
}

#[test]
fn test_find_available_port_skips_occupied_port() {
    // Occupy a port, then verify find_available_port starting there returns a different one
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let occupied = listener.local_addr().unwrap().port();
    let found = find_available_port(occupied);
    assert_ne!(found, occupied, "should skip the occupied port");
    // listener is still held open, so occupied port remains taken
}
```

**Existing tests to update** — change `args.port` assertions from bare `u16` to `Option<u16>`:

```rust
#[test]
fn test_parse_args_extracts_command() {
    let args = parse_args(vec![
        "ShellyMcShellface".into(),
        "echo".into(),
        "hello".into(),
    ]).unwrap();
    assert_eq!(args.command, vec!["echo", "hello"]);
    assert_eq!(args.port, None); // was: 7777
}

#[test]
fn test_parse_args_custom_port() {
    let args = parse_args(vec![
        "ShellyMcShellface".into(),
        "--port".into(),
        "8080".into(),
        "claude".into(),
    ]).unwrap();
    assert_eq!(args.port, Some(8080)); // was: 8080
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
    assert_eq!(args.port, Some(9000)); // was: 9000
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```
cargo test
```

Expected: compile errors — `find_available_port` not defined, `args.port` type mismatch.

- [ ] **Step 3: Change `Args.port` to `Option<u16>` and update `parse_args`**

Replace the `Args` struct and `parse_args` function in `src/main.rs`:

```rust
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
```

- [ ] **Step 4: Add `find_available_port`**

Add this function to `src/main.rs` between `parse_args` and `main`:

```rust
pub fn find_available_port(start: u16) -> u16 {
    for port in start..=start.saturating_add(99) {
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
    eprintln!(
        "No available port in range {}–{}",
        start,
        start.saturating_add(99)
    );
    std::process::exit(1);
}
```

- [ ] **Step 5: Update `main` to resolve the port**

In the `main` function, replace the line `let server_port = args.port;` with:

```rust
let port = match args.port {
    Some(p) => p,
    None => find_available_port(7777),
};
eprintln!("Listening on http://localhost:{}", port);
```

Then update all references to `args.port` and `server_port` in `main` to use `port`:

```rust
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
```

- [ ] **Step 6: Run the full test suite**

```
cargo test
```

Expected: all tests pass, including the two new `find_available_port` tests.

- [ ] **Step 7: Manual smoke test**

Launch two instances without `--port`:

```
./target/release/ShellyMcShellface.exe powershell
# (in a second terminal)
./target/release/ShellyMcShellface.exe powershell
```

Expected:
- First instance prints `Listening on http://localhost:7777` and opens that URL
- Second instance prints `Listening on http://localhost:7778` (or next free port) and opens that URL
- Both sessions run independently in their respective browser tabs

- [ ] **Step 8: Commit**

```
git add src/main.rs
git commit -m "feat: auto-detect next free port from 7777 when --port not specified"
```
