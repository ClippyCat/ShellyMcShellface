# ShellyMcShellface — Design Spec

**Date:** 2026-03-24
**Status:** Approved

## Problem

PowerShell stops responding with large terminal output, and the terminal lacks semantic structure, making output difficult to read with assistive technology. The goal is a browser-based display that replaces terminal output entirely, while keeping input in the terminal.

## Overview

`ShellyMcShellface` is a Rust CLI wrapper. You run `ShellyMcShellface <command>` instead of `<command>` directly. It spawns the command in a PTY, forwards your keystrokes to it, and streams all output to a local browser page via SSE. The terminal never displays any output. The browser is the sole display.

## Architecture

### Rust Server

Single binary, three concurrent Tokio tasks:

**PTY task**
- Spawns the target command in a PTY using `portable-pty` (supports Windows ConPTY)
- Reads the user's stdin in raw mode and forwards every keystroke to the PTY stdin
- Maintains a line-editor buffer server-side: printable characters are appended, backspace (`\x7f`) removes the last character, `\r`/`\n` flushes the buffer as the resolved input text. Control characters other than backspace and newline are forwarded to the PTY but not added to the line-editor buffer
- On Enter, emits an `input` SSE event carrying the resolved text. If the resolved text is empty or whitespace-only, the text is replaced with the literal string `(empty command)`
- Splits PTY stdout on `\n` server-side; each `\n`-terminated segment is emitted as one `output` SSE event
- ANSI SGR codes (colour, bold, reset) are passed through to the client; cursor-movement sequences (`\x1b[A`, `\x1b[2J`, etc.) are stripped server-side before sending

**HTTP task**
- Listens on `localhost:7777` by default; configurable via `--port` flag
- Serves static frontend files and a `/events` SSE endpoint using `axum`
- Maintains an append-only in-memory event buffer containing all `input`, `output`, and `status` events in their original interleaved chronological order
- New SSE connections receive a full replay of the event buffer before live events begin, so tabs opened mid-session reconstruct the complete grouped structure correctly
- Opens the browser automatically on launch using the `webbrowser` crate
- On PTY spawn failure or port collision, prints the error to stderr and exits with a non-zero code

**Key crates:** `portable-pty`, `axum`, `tokio`, `anyhow`, `webbrowser`

### Usage

```
ShellyMcShellface <command> [--port <port>]
```

Examples:
```
ShellyMcShellface claude
ShellyMcShellface ssh user@server
ShellyMcShellface --port 8080 claude
```

### SSE Event Types

| Type | Payload | Description |
|------|---------|-------------|
| `input` | `{ text: string }` | Resolved input text on Enter; `"(empty command)"` if empty or whitespace-only |
| `output` | `{ text: string }` | One `\n`-terminated segment of PTY output, SGR codes preserved, cursor-movement codes stripped |
| `status` | `{ state: "connected" \| "pty_exited" \| "pty_error", code?: number }` | Emitted on SSE connect, on PTY process exit (with exit code), and on PTY spawn error. Included in the replay buffer in chronological order |

## Browser Frontend

A single static HTML page served by the Rust server.

### Page Structure

```
<html lang="en">
<body>
  <main>
    <h1>Terminal</h1>
    <section aria-label="Terminal session">

      <!-- startup group (output before first input, always present) -->
      <details open>
        <summary><h2>Session start</h2></summary>
        <p>output line</p>
        ...
      </details>

      <!-- completed groups (collapsed) -->
      <details>
        <summary><h2>input text</h2></summary>
        <p>output line</p>
        ...
      </details>

      <!-- latest group (always open) -->
      <details open>
        <summary><h2>input text</h2></summary>
        <p>output line</p>
        ...
      </details>

    </section>
  </main>

  <!-- visually hidden live region for screen reader announcements only -->
  <div aria-live="polite" class="sr-only" id="announcer"></div>

  <footer>
    <div role="status" id="connection-status">Connected</div>
  </footer>
</body>
</html>
```

### Grouping Logic

- On load, a "Session start" group is created immediately as an open `<details>` with `<summary><h2>Session start</h2></summary>`. All output before the first input lands here.
- When an `input` event arrives: the current open group is closed (remove `open` attribute); a new `<details open>` group is created with the input text as its summary heading.
- Each `output` event appends a `<p>` to the current open group.
- All groups except the latest are collapsed (`open` attribute absent). The latest group always has `open`.
- When a group is collapsed due to a new `input` event, focus is not programmatically moved. The user's focus is in the real terminal, not the browser, so no focus disruption occurs.

### Screen Reader Announcements

The visible `<section>` is not a live region. A separate visually-hidden `<div aria-live="polite" id="announcer">` receives text-only copies of new output lines. Each new output line is appended as a new `<p>` child element to the announcer — old children are never removed during the session, preserving `aria-atomic` semantics for `aria-live="polite"` (the screen reader announces only the newly added node, not the full region). `aria-atomic` is left at its default (`false`).

To prevent unbounded DOM growth, the announcer is trimmed to its last 50 child elements before each append. This does not affect announced content as the user has already heard those announcements; the visible section holds the permanent record.

`input` events are not announced via the live region. The user typed the input themselves and does not need it read back.

**Known limitation:** `aria-live="polite"` announcements may be dropped or collapsed during rapid PTY output bursts — this is expected screen reader behaviour, not a bug. The `<section>` with its heading structure is the primary reading and navigation interface; the live region is best-effort monitoring for incremental output.

```css
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
```

### Focus and Keyboard Navigation

Naturally focusable elements in tab order:
1. Each `<summary>` element inside `<details>` (the `<summary>` receives focus, not the `<details>` itself)
2. The connection status in `<footer>`

Headings are not in the tab sequence. Screen reader users navigate by heading (`h` key in most readers) using the `<h2>` inside each `<summary>`. This gives two navigation modes: tab through groups (focus order via `<summary>`), or heading-jump through groups (virtual cursor via `<h2>`).

The `<section tabindex="-1">` attribute is present solely to enable programmatic `scrollTop` manipulation across browsers. It does not add the section to the user's tab sequence.

### Auto-Scroll

New output appends to the current open group. The `<section>` element's `scrollTop` is updated to its `scrollHeight` after each append using an instant jump (no smooth scrolling), so `prefers-reduced-motion` is not a concern. This scrolls the container rather than the document, which does not move the screen reader virtual cursor (the virtual cursor follows the live region, not scroll position).

The `<section>` must have `overflow: auto` (or `overflow: scroll`) in CSS for `scrollTop` manipulation to take effect. It must also have `tabindex="-1"` to ensure programmatic scroll works correctly across browsers.

### ANSI Rendering

The client receives PTY output with SGR escape codes intact and cursor-movement codes already stripped server-side. A small JS parser converts SGR sequences to `<span>` elements with inline CSS for colour and weight. Unrecognised SGR codes are stripped silently.

**Colour palette:** The page background is `#1a1a1a` (near-black). ANSI 16-colour values are remapped to AAA-compliant (≥7:1 contrast against `#1a1a1a`) equivalents. The standard ANSI palette is not used directly. The exact colour table is defined in the implementation plan.

### Status Display

The `<div role="status" id="connection-status">` element is updated on each `status` SSE event. Because `status` events are included in the replay buffer, a tab opened after PTY exit will correctly display the final state rather than "Connected."

- `connected` → "Connected"
- `pty_exited` with code 0 → "Process exited"
- `pty_exited` with non-zero code → "Process exited with error (code N)"
- `pty_error` → "Process failed to start"

On SSE disconnect (network drop), the client calls `eventSource.close()` to suppress the `EventSource` API's automatic retry, displays "Disconnected — reload to reconnect" in the status region, and stops processing events. Without closing the `EventSource`, automatic reconnection would replay the full event buffer and duplicate all DOM content.

## What This Is Not

- Not a full terminal emulator — input always comes from the real terminal
- Not Claude-specific — works with any command including SSH sessions
- No Claude semantic conversation view — output is displayed as terminal output with ANSI rendering
