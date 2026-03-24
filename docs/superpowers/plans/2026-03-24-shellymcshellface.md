# ShellyMcShellface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI binary that wraps any command in a PTY and streams all output to an accessible browser page via SSE, replacing the terminal display entirely.

**Architecture:** A single Rust binary runs two concurrent tasks: a PTY task (spawns the target command, forwards raw stdin via crossterm, reads stdout) and an axum HTTP task (serves static frontend files and a `/events` SSE endpoint with full event replay). A vanilla JS frontend groups PTY output under collapsible `<details>` headings and maintains a hidden live region for screen reader announcements.

**Tech Stack:** Rust + Tokio, `portable-pty` (PTY / Windows ConPTY), `axum` 0.8 (HTTP + SSE), `crossterm` 0.28 (raw stdin), `tokio-stream` (BroadcastStream for SSE), `serde_json` (event payloads), `webbrowser` (auto-open); vanilla HTML/CSS/JS frontend embedded via `include_str!`; Node.js for ANSI parser unit tests.

---

## File Structure

```
ShellyMcShellface/
├── Cargo.toml
├── src/
│   ├── main.rs             - CLI entry point, arg parsing, task orchestration
│   ├── types.rs            - SseEvent enum, StatusState enum, serialisation
│   ├── line_editor.rs      - LineEditor: keystroke buffer, flush on Enter
│   ├── ansi.rs             - Strip non-SGR ANSI escape sequences
│   ├── event_buffer.rs     - Thread-safe append-only Vec<SseEvent>
│   ├── server.rs           - axum routes, SSE handler, static file serving
│   ├── pty.rs              - PTY spawn, raw stdin forwarding, stdout processing
│   └── frontend/
│       ├── index.html      - Page structure: details/summary groups, footer status
│       ├── app.js          - SSE connection, grouping logic, scroll, announcer
│       └── ansi.js         - SGR sequence → {text, style} array; Node-testable
└── tests/
    └── frontend/
        └── ansi.test.js    - Node.js unit tests for ansi.js
```

---

## Colour Palette

Background: `#1a1a1a`. All foreground colours verified ≥ 7:1 contrast ratio (WCAG AAA).

| ANSI code | Colour name     | Hex       | Approx contrast |
|-----------|-----------------|-----------|-----------------|
| 30 / 0    | Black (remapped)| `#b0b0b0` | 8.7:1           |
| 31 / 1    | Red             | `#ff8585` | 7.4:1           |
| 32 / 2    | Green           | `#7dff7d` | 13.7:1          |
| 33 / 3    | Yellow          | `#ffff85` | 16.5:1          |
| 34 / 4    | Blue            | `#b0b0ff` | 8.7:1           |
| 35 / 5    | Magenta         | `#ff85ff` | 8.3:1           |
| 36 / 6    | Cyan            | `#85ffff` | 14.7:1          |
| 37 / 7    | White           | `#e8e8e8` | 14.2:1          |
| 90 / 8    | Bright black    | `#c8c8c8` | 11.3:1          |
| 91 / 9    | Bright red      | `#ffaaaa` | 10.2:1          |
| 92 / 10   | Bright green    | `#aaffaa` | 12.9:1          |
| 93 / 11   | Bright yellow   | `#ffffaa` | 15.3:1          |
| 94 / 12   | Bright blue     | `#c8c8ff` | 12.5:1          |
| 95 / 13   | Bright magenta  | `#ffaaff` | 10.2:1          |
| 96 / 14   | Bright cyan     | `#aaffff` | 13.5:1          |
| 97 / 15   | Bright white    | `#ffffff` | 17.4:1          |

Default foreground (no colour code): `#e8e8e8`.

---

## Task 1: Project scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/types.rs` (empty stub)
- Create: `src/line_editor.rs` (empty stub)
- Create: `src/ansi.rs` (empty stub)
- Create: `src/event_buffer.rs` (empty stub)
- Create: `src/server.rs` (empty stub)
- Create: `src/pty.rs` (empty stub)
- Create: `src/frontend/index.html` (empty stub)
- Create: `src/frontend/app.js` (empty stub)
- Create: `src/frontend/ansi.js` (empty stub)

- [x] **Step 1: Create Cargo.toml**

```toml
[package]
name = "shellymcshellface"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "ShellyMcShellface"
path = "src/main.rs"

[dependencies]
portable-pty = "0.8"
axum = { version = "0.8", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
tokio-stream = { version = "0.1", features = ["sync"] }
anyhow = "1"
webbrowser = "1"
crossterm = "0.28"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [x] **Step 2: Create stub src/main.rs**

```rust
mod ansi;
mod event_buffer;
mod line_editor;
mod pty;
mod server;
mod types;

fn main() {
    println!("stub");
}
```

- [x] **Step 3: Create empty stub files**

Create each of these with just a comment:
- `src/types.rs`: `// types`
- `src/line_editor.rs`: `// line_editor`
- `src/ansi.rs`: `// ansi`
- `src/event_buffer.rs`: `// event_buffer`
- `src/server.rs`: `// server`
- `src/pty.rs`: `// pty`
- `src/frontend/index.html`: `<!-- index -->`
- `src/frontend/app.js`: `// app`
- `src/frontend/ansi.js`: `// ansi`
- `tests/frontend/ansi.test.js`: `// tests`

- [x] **Step 4: Verify it compiles**

```
cargo build
```
Expected: compiles with no errors (warnings OK).

- [x] **Step 5: Commit**

```bash
git init
git add Cargo.toml src/ tests/
git commit -m "feat: initial project scaffold"
```

---

## Task 2: Types

**Files:**
- Modify: `src/types.rs`

- [x] **Step 1: Write the failing test**

Add to `src/types.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_event_serialises_to_sse_parts() {
        let ev = SseEvent::output("hello\n");
        let (event_type, data) = ev.to_sse_parts();
        assert_eq!(event_type, "output");
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(json["text"], "hello\n");
    }

    #[test]
    fn test_input_event_serialises_to_sse_parts() {
        let ev = SseEvent::input("ls -la");
        let (event_type, data) = ev.to_sse_parts();
        assert_eq!(event_type, "input");
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(json["text"], "ls -la");
    }

    #[test]
    fn test_status_connected_serialises() {
        let ev = SseEvent::connected();
        let (event_type, data) = ev.to_sse_parts();
        assert_eq!(event_type, "status");
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(json["state"], "connected");
    }

    #[test]
    fn test_status_pty_exited_with_code() {
        let ev = SseEvent::pty_exited(Some(1));
        let (_, data) = ev.to_sse_parts();
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(json["state"], "pty_exited");
        assert_eq!(json["code"], 1);
    }

    #[test]
    fn test_status_pty_exited_code_zero_has_no_code_field() {
        let ev = SseEvent::pty_exited(Some(0));
        let (_, data) = ev.to_sse_parts();
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(json["code"], serde_json::Value::Null);
    }
}
```

- [x] **Step 2: Run tests to verify they fail**

```
cargo test -p shellymcshellface types
```
Expected: compile error — `SseEvent` not defined.

- [x] **Step 3: Implement types.rs**

```rust
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusState {
    Connected,
    PtyExited,
    PtyError,
}

#[derive(Clone, Debug)]
pub enum SseEvent {
    Input { text: String },
    Output { text: String },
    Status { state: StatusState, code: Option<i32> },
}

impl SseEvent {
    pub fn output(text: impl Into<String>) -> Self {
        SseEvent::Output { text: text.into() }
    }

    pub fn input(text: impl Into<String>) -> Self {
        SseEvent::Input { text: text.into() }
    }

    pub fn connected() -> Self {
        SseEvent::Status { state: StatusState::Connected, code: None }
    }

    pub fn pty_exited(code: Option<i32>) -> Self {
        // Only include non-zero codes in the payload
        let emit_code = code.filter(|&c| c != 0);
        SseEvent::Status { state: StatusState::PtyExited, code: emit_code }
    }

    pub fn pty_error() -> Self {
        SseEvent::Status { state: StatusState::PtyError, code: None }
    }

    /// Returns (event_type, json_data_string) for SSE wire format.
    pub fn to_sse_parts(&self) -> (String, String) {
        match self {
            SseEvent::Output { text } => {
                let data = serde_json::json!({ "text": text }).to_string();
                ("output".into(), data)
            }
            SseEvent::Input { text } => {
                let data = serde_json::json!({ "text": text }).to_string();
                ("input".into(), data)
            }
            SseEvent::Status { state, code } => {
                let data = serde_json::json!({ "state": state, "code": code }).to_string();
                ("status".into(), data)
            }
        }
    }
}

#[cfg(test)]
mod tests { /* ... paste tests from Step 1 above ... */ }
```

- [x] **Step 4: Run tests to verify they pass**

```
cargo test -p shellymcshellface types
```
Expected: 5 tests pass.

- [x] **Step 5: Commit**

```bash
git add src/types.rs
git commit -m "feat: add SseEvent types with SSE serialisation"
```

---

## Task 3: LineEditor

**Files:**
- Modify: `src/line_editor.rs`

- [x] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_printable_chars_accumulate_in_buffer() {
        let mut ed = LineEditor::new();
        ed.feed(b'h'); ed.feed(b'i');
        assert_eq!(ed.buffer(), "hi");
    }

    #[test]
    fn test_backspace_removes_last_char() {
        let mut ed = LineEditor::new();
        ed.feed(b'h'); ed.feed(b'i'); ed.feed(0x7f);
        assert_eq!(ed.buffer(), "h");
    }

    #[test]
    fn test_backspace_on_empty_buffer_is_no_op() {
        let mut ed = LineEditor::new();
        ed.feed(0x7f);
        assert_eq!(ed.buffer(), "");
    }

    #[test]
    fn test_enter_cr_flushes_and_returns_text() {
        let mut ed = LineEditor::new();
        ed.feed(b'l'); ed.feed(b's');
        let result = ed.feed(b'\r');
        assert_eq!(result, Some("ls".to_string()));
        assert_eq!(ed.buffer(), "");
    }

    #[test]
    fn test_enter_lf_flushes_and_returns_text() {
        let mut ed = LineEditor::new();
        ed.feed(b'p'); ed.feed(b'w'); ed.feed(b'd');
        let result = ed.feed(b'\n');
        assert_eq!(result, Some("pwd".to_string()));
    }

    #[test]
    fn test_empty_enter_returns_empty_command_literal() {
        let mut ed = LineEditor::new();
        let result = ed.feed(b'\r');
        assert_eq!(result, Some("(empty command)".to_string()));
    }

    #[test]
    fn test_whitespace_only_enter_returns_empty_command_literal() {
        let mut ed = LineEditor::new();
        ed.feed(b' '); ed.feed(b' ');
        let result = ed.feed(b'\r');
        assert_eq!(result, Some("(empty command)".to_string()));
    }

    #[test]
    fn test_control_chars_not_added_to_buffer() {
        let mut ed = LineEditor::new();
        ed.feed(b'a');
        ed.feed(0x03); // Ctrl+C
        ed.feed(0x01); // Ctrl+A
        assert_eq!(ed.buffer(), "a");
    }

    #[test]
    fn test_control_char_does_not_flush() {
        let mut ed = LineEditor::new();
        ed.feed(b'a');
        let result = ed.feed(0x03);
        assert_eq!(result, None);
    }
}
```

- [x] **Step 2: Run to verify failure**

```
cargo test -p shellymcshellface line_editor
```
Expected: compile error — `LineEditor` not defined.

- [x] **Step 3: Implement LineEditor**

```rust
pub struct LineEditor {
    buf: String,
}

impl LineEditor {
    pub fn new() -> Self {
        LineEditor { buf: String::new() }
    }

    pub fn buffer(&self) -> &str {
        &self.buf
    }

    /// Feed one byte. Returns Some(resolved_text) on Enter, None otherwise.
    /// Printable ASCII (0x20–0x7e) appends to buffer.
    /// 0x7f (backspace/DEL) removes last char.
    /// 0x0d or 0x0a (CR/LF) flushes buffer.
    /// All other bytes (control chars) are ignored for the buffer but return None.
    pub fn feed(&mut self, byte: u8) -> Option<String> {
        match byte {
            0x7f => {
                self.buf.pop();
                None
            }
            b'\r' | b'\n' => {
                let text = std::mem::take(&mut self.buf);
                let resolved = if text.trim().is_empty() {
                    "(empty command)".to_string()
                } else {
                    text
                };
                Some(resolved)
            }
            0x20..=0x7e => {
                self.buf.push(byte as char);
                None
            }
            _ => None, // control characters: forward to PTY but don't buffer
        }
    }
}

#[cfg(test)]
mod tests { /* paste tests from Step 1 */ }
```

- [x] **Step 4: Run to verify passing**

```
cargo test -p shellymcshellface line_editor
```
Expected: 9 tests pass.

- [x] **Step 5: Commit**

```bash
git add src/line_editor.rs
git commit -m "feat: add LineEditor with backspace and empty-command handling"
```

---

## Task 4: ANSI cursor-movement stripping

**Files:**
- Modify: `src/ansi.rs`

The stripping rule: keep CSI sequences ending in `m` (SGR — colour, bold, reset). Strip all other escape sequences: CSI sequences ending in other letters (`A`, `B`, `C`, `D`, `H`, `J`, `K`, `f`, `l`, `h`, `s`, `u`, etc.), OSC sequences (`\x1b]....\x07`), and bare `\x1b` + single char sequences.

- [x] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text_unchanged() {
        assert_eq!(strip_non_sgr("hello world"), "hello world");
    }

    #[test]
    fn test_sgr_reset_preserved() {
        assert_eq!(strip_non_sgr("\x1b[0m"), "\x1b[0m");
    }

    #[test]
    fn test_sgr_bold_preserved() {
        assert_eq!(strip_non_sgr("\x1b[1m"), "\x1b[1m");
    }

    #[test]
    fn test_sgr_colour_preserved() {
        assert_eq!(strip_non_sgr("\x1b[32m"), "\x1b[32m");
    }

    #[test]
    fn test_sgr_compound_preserved() {
        assert_eq!(strip_non_sgr("\x1b[1;32m"), "\x1b[1;32m");
    }

    #[test]
    fn test_cursor_up_stripped() {
        assert_eq!(strip_non_sgr("a\x1b[Ab"), "ab");
    }

    #[test]
    fn test_cursor_up_with_count_stripped() {
        assert_eq!(strip_non_sgr("a\x1b[3Ab"), "ab");
    }

    #[test]
    fn test_clear_screen_stripped() {
        assert_eq!(strip_non_sgr("a\x1b[2Jb"), "ab");
    }

    #[test]
    fn test_cursor_position_stripped() {
        assert_eq!(strip_non_sgr("a\x1b[1;1Hb"), "ab");
    }

    #[test]
    fn test_osc_title_stripped() {
        assert_eq!(strip_non_sgr("\x1b]0;My Title\x07text"), "text");
    }

    #[test]
    fn test_private_mode_stripped() {
        assert_eq!(strip_non_sgr("\x1b[?25l"), "");
    }

    #[test]
    fn test_text_around_sgr_preserved() {
        assert_eq!(
            strip_non_sgr("before\x1b[1mbold\x1b[0mafter"),
            "before\x1b[1mbold\x1b[0mafter"
        );
    }

    #[test]
    fn test_mixed_sgr_and_cursor_movement() {
        assert_eq!(
            strip_non_sgr("\x1b[1mhello\x1b[A\x1b[0m"),
            "\x1b[1mhello\x1b[0m"
        );
    }
}
```

- [x] **Step 2: Run to verify failure**

```
cargo test -p shellymcshellface ansi
```
Expected: compile error.

- [x] **Step 3: Implement strip_non_sgr**

```rust
/// Strip all ANSI escape sequences except SGR (colour/bold — sequences ending in 'm').
/// SGR sequences are passed through intact.
pub fn strip_non_sgr(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != 0x1b {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        // ESC
        if i + 1 >= bytes.len() {
            i += 1;
            continue;
        }
        match bytes[i + 1] {
            b'[' => {
                // CSI: consume ESC [ <params> <final>
                // <params> = digits, semicolons, '?'
                // <final> = 0x40-0x7e
                let start = i;
                i += 2;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b';' || bytes[i] == b'?') {
                    i += 1;
                }
                if i < bytes.len() {
                    let final_byte = bytes[i];
                    i += 1;
                    if final_byte == b'm' {
                        // SGR — keep the whole sequence
                        out.push_str(&input[start..i]);
                    }
                    // otherwise discard
                }
            }
            b']' => {
                // OSC: ESC ] ... BEL(0x07) or ST(ESC \)
                i += 2;
                while i < bytes.len() {
                    if bytes[i] == 0x07 {
                        i += 1;
                        break;
                    }
                    if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            _ => {
                // ESC + single char — strip both
                i += 2;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests { /* paste tests from Step 1 */ }
```

- [x] **Step 4: Run to verify passing**

```
cargo test -p shellymcshellface ansi
```
Expected: 13 tests pass.

- [x] **Step 5: Commit**

```bash
git add src/ansi.rs
git commit -m "feat: add ANSI cursor-movement stripping, preserve SGR"
```

---

## Task 5: EventBuffer

**Files:**
- Modify: `src/event_buffer.rs`

- [x] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SseEvent;

    #[test]
    fn test_new_buffer_is_empty() {
        let buf = EventBuffer::new();
        assert_eq!(buf.snapshot().len(), 0);
    }

    #[test]
    fn test_push_and_snapshot_preserves_order() {
        let buf = EventBuffer::new();
        buf.push(SseEvent::connected());
        buf.push(SseEvent::output("line1\n"));
        buf.push(SseEvent::input("ls"));
        buf.push(SseEvent::output("file.txt\n"));
        let snap = buf.snapshot();
        assert_eq!(snap.len(), 4);
        assert!(matches!(snap[0], SseEvent::Status { .. }));
        assert!(matches!(snap[1], SseEvent::Output { .. }));
        assert!(matches!(snap[2], SseEvent::Input { .. }));
        assert!(matches!(snap[3], SseEvent::Output { .. }));
    }

    #[test]
    fn test_snapshot_is_a_clone() {
        let buf = EventBuffer::new();
        buf.push(SseEvent::output("a\n"));
        let snap1 = buf.snapshot();
        buf.push(SseEvent::output("b\n"));
        let snap2 = buf.snapshot();
        assert_eq!(snap1.len(), 1);
        assert_eq!(snap2.len(), 2);
    }
}
```

- [x] **Step 2: Run to verify failure**

```
cargo test -p shellymcshellface event_buffer
```
Expected: compile error.

- [x] **Step 3: Implement EventBuffer**

```rust
use std::sync::Mutex;
use crate::types::SseEvent;

pub struct EventBuffer {
    events: Mutex<Vec<SseEvent>>,
}

impl EventBuffer {
    pub fn new() -> Self {
        EventBuffer { events: Mutex::new(Vec::new()) }
    }

    pub fn push(&self, event: SseEvent) {
        self.events.lock().unwrap().push(event);
    }

    /// Returns a snapshot clone of all events in insertion order.
    pub fn snapshot(&self) -> Vec<SseEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests { /* paste tests from Step 1 */ }
```

Also add `#[derive(Clone)]` to `SseEvent` in `src/types.rs` (it already has it).

- [x] **Step 4: Run to verify passing**

```
cargo test -p shellymcshellface event_buffer
```
Expected: 3 tests pass.

- [x] **Step 5: Commit**

```bash
git add src/event_buffer.rs src/types.rs
git commit -m "feat: add thread-safe EventBuffer with snapshot"
```

---

## Task 6: HTTP server and SSE endpoint

**Files:**
- Modify: `src/server.rs`

The server:
- `GET /` → `index.html`
- `GET /app.js` → `app.js`
- `GET /ansi.js` → `ansi.js`
- `GET /events` → SSE stream: replay buffer, then live events from broadcast channel

The SSE handler subscribes to the broadcast channel first, then reads the buffer snapshot. This ensures no events are missed: any event pushed after subscribe (while we read the snapshot) will be in the receiver. The small theoretical window where the same event could appear in both snapshot and receiver is handled by noting the receiver only contains events pushed *after* subscribe, while the snapshot contains events pushed *before* we subscribed (the PTY lock ensures push→broadcast happens atomically from the SSE handler's perspective if we subscribe inside a buffer lock — but for simplicity in this local tool, accept the negligible overlap).

- [x] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SseEvent;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_sse_format_from_event() {
        // Verify that SseEvent converts correctly to axum SSE Event
        let ev = SseEvent::output("hello\n");
        let (event_type, data) = ev.to_sse_parts();
        let sse = axum::response::sse::Event::default()
            .event(event_type)
            .data(data);
        // axum::response::sse::Event doesn't expose fields directly,
        // but construction succeeding is the test.
        // We verify the parts instead:
        let ev2 = SseEvent::output("hello\n");
        let (t, d) = ev2.to_sse_parts();
        assert_eq!(t, "output");
        assert!(d.contains("hello"));
    }

    #[tokio::test]
    async fn test_app_state_holds_shared_refs() {
        let (tx, _rx) = broadcast::channel::<SseEvent>(1024);
        let buf = Arc::new(crate::event_buffer::EventBuffer::new());
        let state = AppState { tx: Arc::new(tx), event_buf: buf };
        state.event_buf.push(SseEvent::connected());
        assert_eq!(state.event_buf.snapshot().len(), 1);
    }
}
```

- [x] **Step 2: Run to verify failure**

```
cargo test -p shellymcshellface server
```
Expected: compile error.

- [x] **Step 3: Implement server.rs**

```rust
use std::sync::Arc;
use axum::{
    Router,
    extract::State,
    response::{Html, sse::{Event, KeepAlive, Sse}},
    routing::get,
};
use tokio::sync::broadcast;
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use crate::{event_buffer::EventBuffer, types::SseEvent};

const INDEX_HTML: &str = include_str!("frontend/index.html");
const APP_JS: &str = include_str!("frontend/app.js");
const ANSI_JS: &str = include_str!("frontend/ansi.js");

#[derive(Clone)]
pub struct AppState {
    pub tx: Arc<broadcast::Sender<SseEvent>>,
    pub event_buf: Arc<EventBuffer>,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/app.js", get(serve_app_js))
        .route("/ansi.js", get(serve_ansi_js))
        .route("/events", get(sse_handler))
        .with_state(state)
}

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn serve_app_js() -> ([(&'static str, &'static str); 1], &'static str) {
    ([("content-type", "application/javascript")], APP_JS)
}

async fn serve_ansi_js() -> ([(&'static str, &'static str); 1], &'static str) {
    ([("content-type", "application/javascript")], ANSI_JS)
}

async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, anyhow::Error>>> {
    // Subscribe before reading buffer to avoid missing events
    let rx = state.tx.subscribe();
    let buffered = state.event_buf.snapshot();

    let buffered_stream = tokio_stream::iter(buffered)
        .map(|ev| Ok(ev));

    let live_stream = BroadcastStream::new(rx)
        .filter_map(|r| r.ok())  // skip Lagged errors
        .map(|ev| Ok(ev));

    let combined = buffered_stream.chain(live_stream)
        .map(|r: Result<SseEvent, anyhow::Error>| {
            r.and_then(|ev| {
                let (event_type, data) = ev.to_sse_parts();
                Ok(Event::default().event(event_type).data(data))
            })
        });

    Sse::new(combined).keep_alive(KeepAlive::default())
}

/// Starts the HTTP server on the given port. Returns only on error.
pub async fn run_server(state: AppState, port: u16) -> anyhow::Result<()> {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let router = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

#[cfg(test)]
mod tests { /* paste tests from Step 1 */ }
```

Add to `Cargo.toml`:
```toml
futures-core = "0.3"
```

- [x] **Step 4: Run to verify passing**

```
cargo test -p shellymcshellface server
```
Expected: 2 tests pass.

- [x] **Step 5: Compile check**

```
cargo build
```
Expected: builds (index.html/app.js/ansi.js are stubs, that's fine).

- [x] **Step 6: Commit**

```bash
git add src/server.rs Cargo.toml
git commit -m "feat: add axum HTTP server with SSE endpoint and buffer replay"
```

---

## Task 7: PTY task

**Files:**
- Modify: `src/pty.rs`

The PTY task:
1. Spawns the target command in a PTY using `portable-pty`
2. Emits `connected` status event (into buffer + broadcast)
3. Spawns a thread to read raw stdin (crossterm raw mode) → forward bytes to PTY stdin → feed `LineEditor` → emit `input` events on Enter
4. Reads PTY stdout in the main thread → buffers partial lines → on `\n`: calls `strip_non_sgr` → emits `output` event
5. When PTY stdout closes: waits for child exit code → emits `pty_exited` or `pty_error` status event
6. On spawn failure: emits `pty_error` status event

This function is synchronous (blocking) — call it inside `tokio::task::spawn_blocking`.

- [x] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{event_buffer::EventBuffer, types::SseEvent};
    use std::sync::Arc;
    use tokio::sync::broadcast;

    fn make_deps() -> (Arc<broadcast::Sender<SseEvent>>, Arc<EventBuffer>) {
        let (tx, _rx) = broadcast::channel(128);
        (Arc::new(tx), Arc::new(EventBuffer::new()))
    }

    #[test]
    fn test_emit_pushes_to_buffer_and_broadcasts() {
        let (tx, buf) = make_deps();
        let mut rx = tx.subscribe();
        emit(SseEvent::connected(), &tx, &buf);
        assert_eq!(buf.snapshot().len(), 1);
        assert!(rx.try_recv().is_ok());
    }

    #[test]
    fn test_process_stdout_chunk_emits_complete_lines() {
        let (tx, buf) = make_deps();
        let mut partial = String::new();
        process_stdout_chunk(b"hello\nworld\n", &mut partial, &tx, &buf);
        let events = buf.snapshot();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_process_stdout_chunk_buffers_partial_line() {
        let (tx, buf) = make_deps();
        let mut partial = String::new();
        process_stdout_chunk(b"hel", &mut partial, &tx, &buf);
        assert_eq!(buf.snapshot().len(), 0); // no complete line yet
        assert_eq!(partial, "hel");
    }

    #[test]
    fn test_process_stdout_chunk_completes_partial() {
        let (tx, buf) = make_deps();
        let mut partial = "hel".to_string();
        process_stdout_chunk(b"lo\n", &mut partial, &tx, &buf);
        assert_eq!(buf.snapshot().len(), 1);
        assert_eq!(partial, "");
    }

    #[test]
    fn test_process_stdout_strips_cursor_movement() {
        let (tx, buf) = make_deps();
        let mut partial = String::new();
        process_stdout_chunk(b"a\x1b[Ab\n", &mut partial, &tx, &buf);
        let events = buf.snapshot();
        if let SseEvent::Output { text } = &events[0] {
            assert_eq!(text, "ab\n");
        } else {
            panic!("expected Output event");
        }
    }
}
```

- [x] **Step 2: Run to verify failure**

```
cargo test -p shellymcshellface pty
```
Expected: compile error.

- [x] **Step 3: Implement pty.rs**

```rust
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::broadcast;
use anyhow::Result;
use crate::{
    ansi::strip_non_sgr,
    event_buffer::EventBuffer,
    line_editor::LineEditor,
    types::SseEvent,
};

/// Push an event to both the buffer and the broadcast channel.
pub fn emit(event: SseEvent, tx: &broadcast::Sender<SseEvent>, buf: &EventBuffer) {
    buf.push(event.clone());
    let _ = tx.send(event); // ignore SendError (no active receivers is fine)
}

/// Process a raw chunk of PTY stdout bytes.
/// Appends to `partial` buffer. Emits one Output event per complete `\n`-terminated line.
pub fn process_stdout_chunk(
    chunk: &[u8],
    partial: &mut String,
    tx: &broadcast::Sender<SseEvent>,
    buf: &EventBuffer,
) {
    // Append bytes to partial, handling non-UTF8 lossily
    partial.push_str(&String::from_utf8_lossy(chunk));

    while let Some(pos) = partial.find('\n') {
        let line: String = partial.drain(..=pos).collect();
        let stripped = strip_non_sgr(&line);
        emit(SseEvent::output(stripped), tx, buf);
    }
}

/// Run the full PTY session. This function blocks until the PTY child exits.
/// Call inside `tokio::task::spawn_blocking`.
pub fn run_pty_session(
    command: Vec<String>,
    tx: Arc<broadcast::Sender<SseEvent>>,
    buf: Arc<EventBuffer>,
) -> Result<()> {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};

    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new(&command[0]);
    for arg in &command[1..] {
        cmd.arg(arg);
    }

    let child = pair.slave.spawn_command(cmd).map_err(|e| {
        let tx2 = Arc::clone(&tx);
        let buf2 = Arc::clone(&buf);
        emit(SseEvent::pty_error(), &tx2, &buf2);
        e
    })?;

    emit(SseEvent::connected(), &tx, &buf);

    // Stdin thread: raw mode, forward keystrokes to PTY, feed LineEditor
    let mut pty_writer = pair.master.take_writer()?;
    let tx_stdin = Arc::clone(&tx);
    let buf_stdin = Arc::clone(&buf);
    std::thread::spawn(move || -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        let stdin = std::io::stdin();
        let mut line_editor = LineEditor::new();
        let mut raw_buf = [0u8; 256];
        loop {
            let n = stdin.lock().read(&mut raw_buf)?;
            if n == 0 { break; }
            let _ = pty_writer.write_all(&raw_buf[..n]);
            for &byte in &raw_buf[..n] {
                if let Some(text) = line_editor.feed(byte) {
                    emit(SseEvent::input(text), &tx_stdin, &buf_stdin);
                }
            }
        }
        Ok(())
    });

    // Stdout reader (main thread of this fn)
    let mut reader = pair.master.try_clone_reader()?;
    let mut read_buf = [0u8; 4096];
    let mut partial = String::new();
    loop {
        match reader.read(&mut read_buf) {
            Ok(0) => break,
            Ok(n) => process_stdout_chunk(&read_buf[..n], &mut partial, &tx, &buf),
            Err(_) => break,
        }
    }

    // Flush any remaining partial line (no trailing newline)
    if !partial.is_empty() {
        let stripped = strip_non_sgr(&partial);
        emit(SseEvent::output(stripped), &tx, &buf);
    }

    // Wait for child and emit final status
    let exit_status = child.wait()?;
    let code = exit_status.exit_code() as i32;
    emit(SseEvent::pty_exited(Some(code)), &tx, &buf);

    // Restore terminal
    let _ = crossterm::terminal::disable_raw_mode();

    Ok(())
}

#[cfg(test)]
mod tests { /* paste tests from Step 1 */ }
```

Note: `portable_pty::ExitStatus::exit_code()` returns `u32`. Cast to `i32` for the payload. On Windows/Unix, typical exit codes fit in `i32`.

- [x] **Step 4: Run unit tests to verify passing**

```
cargo test -p shellymcshellface pty
```
Expected: 5 tests pass (the unit tests don't actually spawn a PTY).

- [x] **Step 5: Build check**

```
cargo build
```
Expected: compiles.

- [x] **Step 6: Commit**

```bash
git add src/pty.rs
git commit -m "feat: add PTY task with stdin forwarding and stdout line splitting"
```

---

## Task 8: main.rs orchestration

**Files:**
- Modify: `src/main.rs`

- [x] **Step 1: Write the failing test**

```rust
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
        assert_eq!(args.port, 7777);
    }

    #[test]
    fn test_parse_args_custom_port() {
        let args = parse_args(vec![
            "ShellyMcShellface".into(),
            "--port".into(),
            "8080".into(),
            "claude".into(),
        ]).unwrap();
        assert_eq!(args.port, 8080);
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
        assert_eq!(args.port, 9000);
    }

    #[test]
    fn test_parse_args_no_command_returns_error() {
        let result = parse_args(vec!["ShellyMcShellface".into()]);
        assert!(result.is_err());
    }
}
```

- [x] **Step 2: Run to verify failure**

```
cargo test -p shellymcshellface main
```
Expected: compile errors.

- [x] **Step 3: Implement main.rs**

```rust
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
    pub port: u16,
}

pub fn parse_args(raw: Vec<String>) -> Result<Args> {
    let mut command = Vec::new();
    let mut port: u16 = 7777;
    let mut i = 1; // skip binary name

    while i < raw.len() {
        if raw[i] == "--port" {
            i += 1;
            port = raw.get(i)
                .context("--port requires a value")?
                .parse::<u16>()
                .context("--port must be a number 1-65535")?;
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

    // Start HTTP server
    let server_port = args.port;
    tokio::spawn(async move {
        if let Err(e) = server::run_server(state, server_port).await {
            eprintln!("Server error: {e}");
        }
    });

    // Brief pause to let the server bind before opening browser
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Open browser
    let url = format!("http://localhost:{}", args.port);
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
mod tests { /* paste tests from Step 1 */ }
```

- [x] **Step 4: Run to verify passing**

```
cargo test -p shellymcshellface main
```
Expected: 4 tests pass.

- [x] **Step 5: Build**

```
cargo build
```
Expected: binary `ShellyMcShellface` produced in `target/debug/`.

- [x] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: add CLI arg parsing and main task orchestration"
```

---

## Task 9: Frontend HTML and CSS

**Files:**
- Modify: `src/frontend/index.html`

- [x] **Step 1: Write index.html**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Terminal</title>
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

    body {
      background: #1a1a1a;
      color: #e8e8e8;
      font-family: "Courier New", Courier, monospace;
      font-size: 1rem;
      line-height: 1.5;
      min-height: 100vh;
      display: flex;
      flex-direction: column;
    }

    main { flex: 1; padding: 1rem; }

    h1 { font-size: 1.25rem; margin-bottom: 1rem; }

    section[aria-label="Terminal session"] {
      overflow: auto;
      max-height: calc(100vh - 6rem);
      outline: none; /* tabindex=-1, no visible focus ring needed */
    }

    details { border-bottom: 1px solid #333; }

    summary {
      cursor: pointer;
      padding: 0.5rem 0;
      list-style: none; /* remove default triangle on some browsers */
    }
    summary::-webkit-details-marker { display: none; }

    summary:focus-visible {
      outline: 3px solid #ffff85;
      outline-offset: 2px;
    }

    summary h2 {
      display: inline;
      font-size: 1rem;
      font-weight: bold;
      color: #aaffff;
    }

    details[open] summary h2::before { content: "▼ "; }
    details:not([open]) summary h2::before { content: "▶ "; }

    details p {
      padding: 0.15rem 1.5rem;
      white-space: pre-wrap;
      word-break: break-all;
    }

    footer {
      padding: 0.5rem 1rem;
      border-top: 1px solid #333;
      font-size: 0.875rem;
    }

    #connection-status { color: #aaffaa; }
    #connection-status[data-state="disconnected"] { color: #ff8585; }
    #connection-status[data-state="error"] { color: #ff8585; }
    #connection-status[data-state="exited"] { color: #e8e8e8; }

    .sr-only {
      position: absolute;
      width: 1px;
      height: 1px;
      padding: 0;
      margin: -1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
      border: 0;
    }
  </style>
</head>
<body>
  <main>
    <h1>Terminal</h1>
    <section aria-label="Terminal session" tabindex="-1" id="session">
    </section>
  </main>

  <div aria-live="polite" class="sr-only" id="announcer"></div>

  <footer>
    <div role="status" id="connection-status" data-state="connecting">Connecting…</div>
  </footer>

  <script src="/ansi.js"></script>
  <script src="/app.js"></script>
</body>
</html>
```

- [x] **Step 2: Build and verify it compiles**

```
cargo build
```
Expected: compiles (index.html is now real content embedded into binary).

- [x] **Step 3: Commit**

```bash
git add src/frontend/index.html
git commit -m "feat: add accessible frontend HTML structure with sr-only, details/summary groups"
```

---

## Task 10: Frontend JS — ANSI SGR parser

**Files:**
- Modify: `src/frontend/ansi.js`
- Modify: `tests/frontend/ansi.test.js`

`parseAnsi(text)` returns an array of `{ text: string, style: string }` objects. `style` is a CSS inline style string (e.g. `"color:#ff8585;font-weight:bold"`). Plain text segments have `style: ""`. This keeps the parser pure and testable outside the browser.

- [x] **Step 1: Write the failing Node tests**

```javascript
// tests/frontend/ansi.test.js
'use strict';
const assert = require('assert');
// Load ansi.js — it conditionally exports via module.exports
const { parseAnsi } = require('../../src/frontend/ansi.js');

function run(name, fn) {
  try { fn(); console.log(`  PASS: ${name}`); }
  catch (e) { console.error(`  FAIL: ${name}\n       ${e.message}`); process.exitCode = 1; }
}

run('plain text returns single span with no style', () => {
  const spans = parseAnsi('hello');
  assert.equal(spans.length, 1);
  assert.equal(spans[0].text, 'hello');
  assert.equal(spans[0].style, '');
});

run('empty string returns empty array', () => {
  const spans = parseAnsi('');
  assert.equal(spans.length, 0);
});

run('reset code \x1b[0m ends current style', () => {
  const spans = parseAnsi('\x1b[1mhi\x1b[0m world');
  // 'hi' is bold, ' world' has no style
  const boldSpan = spans.find(s => s.text === 'hi');
  const plainSpan = spans.find(s => s.text === ' world');
  assert.ok(boldSpan, 'bold span not found');
  assert.ok(plainSpan, 'plain span not found');
  assert.ok(boldSpan.style.includes('font-weight:bold'));
  assert.equal(plainSpan.style, '');
});

run('ANSI red (31) maps to #ff8585', () => {
  const spans = parseAnsi('\x1b[31mred\x1b[0m');
  const span = spans.find(s => s.text === 'red');
  assert.ok(span, 'red span not found');
  assert.ok(span.style.includes('color:#ff8585'), `got: ${span.style}`);
});

run('ANSI green (32) maps to #7dff7d', () => {
  const spans = parseAnsi('\x1b[32mgreen\x1b[0m');
  const span = spans.find(s => s.text === 'green');
  assert.ok(span.style.includes('color:#7dff7d'), `got: ${span.style}`);
});

run('bright blue (94) maps to #c8c8ff', () => {
  const spans = parseAnsi('\x1b[94mblue\x1b[0m');
  const span = spans.find(s => s.text === 'blue');
  assert.ok(span.style.includes('color:#c8c8ff'), `got: ${span.style}`);
});

run('bold (1) sets font-weight:bold', () => {
  const spans = parseAnsi('\x1b[1mbold\x1b[0m');
  const span = spans.find(s => s.text === 'bold');
  assert.ok(span.style.includes('font-weight:bold'));
});

run('compound code 1;32 applies bold and green', () => {
  const spans = parseAnsi('\x1b[1;32mtext\x1b[0m');
  const span = spans.find(s => s.text === 'text');
  assert.ok(span.style.includes('font-weight:bold'), `missing bold: ${span.style}`);
  assert.ok(span.style.includes('color:#7dff7d'), `missing color: ${span.style}`);
});

run('true color 38;2;255;128;0 passes through as rgb()', () => {
  const spans = parseAnsi('\x1b[38;2;255;128;0morange\x1b[0m');
  const span = spans.find(s => s.text === 'orange');
  assert.ok(span.style.includes('color:rgb(255,128,0)'), `got: ${span.style}`);
});

run('unrecognised codes silently stripped', () => {
  // SGR code 999 is not recognised — should be stripped, text still output
  const spans = parseAnsi('\x1b[999mtext\x1b[0m');
  const span = spans.find(s => s.text === 'text');
  assert.ok(span, 'text span not found');
});

run('default fg (39) resets colour', () => {
  const spans = parseAnsi('\x1b[31mred\x1b[39mplain\x1b[0m');
  const plain = spans.find(s => s.text === 'plain');
  assert.ok(plain, 'plain span not found');
  assert.ok(!plain.style.includes('color'), `color not reset: ${plain.style}`);
});
```

- [x] **Step 2: Run to verify failure**

```
node tests/frontend/ansi.test.js
```
Expected: error — `parseAnsi` not exported.

- [x] **Step 3: Implement ansi.js**

```javascript
// src/frontend/ansi.js
'use strict';

const FG_PALETTE = {
  30: '#b0b0b0', 31: '#ff8585', 32: '#7dff7d', 33: '#ffff85',
  34: '#b0b0ff', 35: '#ff85ff', 36: '#85ffff', 37: '#e8e8e8',
  90: '#c8c8c8', 91: '#ffaaaa', 92: '#aaffaa', 93: '#ffffaa',
  94: '#c8c8ff', 95: '#ffaaff', 96: '#aaffff', 97: '#ffffff',
};

/**
 * Apply a list of SGR numeric codes to a mutable state object.
 * state: { color: string|null, bold: boolean }
 */
function applyCodes(codes, state) {
  let i = 0;
  while (i < codes.length) {
    const c = codes[i];
    if (c === 0 || c === '') {
      state.color = null;
      state.bold = false;
    } else if (c === 1) {
      state.bold = true;
    } else if (c === 22) {
      state.bold = false;
    } else if (FG_PALETTE[c] !== undefined) {
      state.color = FG_PALETTE[c];
    } else if (c === 39) {
      state.color = null;
    } else if (c === 38 && codes[i + 1] === 2) {
      const r = codes[i + 2], g = codes[i + 3], b = codes[i + 4];
      state.color = `rgb(${r},${g},${b})`;
      i += 4;
    }
    // all other codes (background, 256-colour, etc.) silently ignored
    i++;
  }
}

function stateToStyle(state) {
  const parts = [];
  if (state.color) parts.push(`color:${state.color}`);
  if (state.bold) parts.push('font-weight:bold');
  return parts.join(';');
}

/**
 * Parse a string containing ANSI SGR escape sequences.
 * Returns an array of { text: string, style: string }.
 * Only SGR sequences (already stripped of cursor-movement server-side) are handled.
 */
function parseAnsi(text) {
  const result = [];
  const state = { color: null, bold: false };
  // Match SGR sequences or plain text segments
  const re = /\x1b\[([\d;]*)m|([^\x1b]+)/g;
  let match;
  while ((match = re.exec(text)) !== null) {
    if (match[2] !== undefined) {
      // Plain text
      const style = stateToStyle(state);
      result.push({ text: match[2], style });
    } else {
      // SGR sequence — update state
      const raw = match[1];
      const codes = raw === '' ? [0] : raw.split(';').map(Number);
      applyCodes(codes, state);
    }
  }
  return result;
}

if (typeof module !== 'undefined') {
  module.exports = { parseAnsi };
}
```

- [x] **Step 4: Run to verify passing**

```
node tests/frontend/ansi.test.js
```
Expected: all 11 tests show `PASS`.

- [x] **Step 5: Commit**

```bash
git add src/frontend/ansi.js tests/frontend/ansi.test.js
git commit -m "feat: add ANSI SGR parser with AAA-compliant 16-colour palette"
```

---

## Task 11: Frontend JS — app logic

**Files:**
- Modify: `src/frontend/app.js`

- [x] **Step 1: Write app.js**

```javascript
// src/frontend/app.js
'use strict';

(function () {
  const session = document.getElementById('session');
  const announcer = document.getElementById('announcer');
  const statusEl = document.getElementById('connection-status');

  // The currently open <details> element receiving output
  let currentGroup = null;

  function createGroup(labelText) {
    const details = document.createElement('details');
    details.setAttribute('open', '');

    const summary = document.createElement('summary');
    const h2 = document.createElement('h2');
    h2.textContent = labelText;
    summary.appendChild(h2);
    details.appendChild(summary);

    return details;
  }

  function closeCurrentGroup() {
    if (currentGroup) {
      currentGroup.removeAttribute('open');
    }
  }

  function openNewGroup(labelText) {
    closeCurrentGroup();
    currentGroup = createGroup(labelText);
    session.appendChild(currentGroup);
  }

  function appendOutputLine(text) {
    if (!currentGroup) {
      openNewGroup('Session start');
    }
    const p = document.createElement('p');
    // Convert ANSI SGR sequences to styled spans
    const spans = parseAnsi(text);
    if (spans.length === 0) {
      p.textContent = text;
    } else {
      for (const { text: t, style } of spans) {
        if (!t) continue;
        if (style) {
          const span = document.createElement('span');
          span.style.cssText = style;
          span.textContent = t;
          p.appendChild(span);
        } else {
          p.appendChild(document.createTextNode(t));
        }
      }
    }
    currentGroup.appendChild(p);
    scrollToBottom();
    announceOutput(text);
  }

  function scrollToBottom() {
    session.scrollTop = session.scrollHeight;
  }

  function announceOutput(text) {
    // Trim announcer to last 50 children to prevent unbounded growth
    while (announcer.childElementCount >= 50) {
      announcer.removeChild(announcer.firstChild);
    }
    const p = document.createElement('p');
    // Plain text only for announcements — no ANSI codes
    p.textContent = text.replace(/\x1b\[[^m]*m/g, '');
    announcer.appendChild(p);
  }

  function handleInput(payload) {
    openNewGroup(payload.text);
  }

  function handleOutput(payload) {
    appendOutputLine(payload.text);
  }

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

  function setStatus(text, state) {
    statusEl.textContent = text;
    statusEl.dataset.state = state;
  }

  // SSE connection
  const es = new EventSource('/events');

  es.addEventListener('input', (e) => {
    try { handleInput(JSON.parse(e.data)); } catch (_) {}
  });

  es.addEventListener('output', (e) => {
    try { handleOutput(JSON.parse(e.data)); } catch (_) {}
  });

  es.addEventListener('status', (e) => {
    try { handleStatus(JSON.parse(e.data)); } catch (_) {}
  });

  es.onerror = () => {
    es.close(); // suppress automatic retry — would duplicate all DOM content
    setStatus('Disconnected — reload to reconnect', 'disconnected');
  };
})();
```

- [x] **Step 2: Build**

```
cargo build
```
Expected: compiles.

- [x] **Step 3: Commit**

```bash
git add src/frontend/app.js
git commit -m "feat: add frontend SSE client with grouping, scroll, and live region"
```

---

## Task 12: End-to-end smoke test

- [ ] **Step 1: Build release binary**

```
cargo build --release
```
Expected: `target/release/ShellyMcShellface` (or `.exe` on Windows).

- [ ] **Step 2: Run with a simple command**

```
./target/release/ShellyMcShellface echo "hello from ShellyMcShellface"
```

Expected:
- Browser opens at `http://localhost:7777`
- Page shows "Terminal" heading
- "Session start" group contains the echo output
- Footer shows "Process exited"
- Status region is announced by screen reader

- [ ] **Step 3: Verify accessible structure in browser DevTools**

Check:
- `<section aria-label="Terminal session">` is present
- Each group is `<details>` with `<summary><h2>…</h2></summary>`
- `#announcer` has `aria-live="polite"` and `class="sr-only"`
- `#connection-status` has `role="status"`

- [ ] **Step 4: Run all tests**

```
cargo test
node tests/frontend/ansi.test.js
```
Expected: all pass.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat: ShellyMcShellface v0.1 — accessible browser terminal display"
```
