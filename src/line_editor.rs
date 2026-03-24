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
            _ => None,
        }
    }
}

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
