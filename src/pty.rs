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

    let mut child = pair.slave.spawn_command(cmd).map_err(|e| {
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
        assert_eq!(buf.snapshot().len(), 0);
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
