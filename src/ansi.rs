/// Strip all ANSI escape sequences except SGR (colour/bold — sequences ending in 'm').
/// SGR sequences are passed through intact.
///
/// Output is built into a `Vec<u8>` so that multi-byte UTF-8 sequences in the
/// input are copied byte-for-byte rather than being corrupted by `byte as char`
/// casts.  The result is always valid UTF-8 because:
///   • non-ESC bytes are copied verbatim from the (valid UTF-8) input, and
///   • kept ANSI sequences are ASCII and therefore also valid UTF-8.
pub fn strip_non_sgr(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(input.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != 0x1b {
            out.push(bytes[i]);
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
                // CSI: ESC [ <params> <intermediates> <final>
                // Per ECMA-48:
                //   parameter bytes:    0x30-0x3F  (0-9 : ; < = > ?)
                //   intermediate bytes: 0x20-0x2F  (space ! " # $ % & ' ( ) * + , - . /)
                //   final byte:         0x40-0x7E
                i += 2;
                let param_start = i;
                // consume parameter bytes
                while i < bytes.len() && matches!(bytes[i], 0x30..=0x3F) {
                    i += 1;
                }
                let param_end = i;
                // consume intermediate bytes
                while i < bytes.len() && matches!(bytes[i], 0x20..=0x2F) {
                    i += 1;
                }
                // consume final byte
                if i < bytes.len() && matches!(bytes[i], 0x40..=0x7E) {
                    let final_byte = bytes[i];
                    i += 1;
                    let params = &bytes[param_start..param_end];
                    if final_byte == b'm' {
                        // SGR — keep the whole sequence
                        out.extend_from_slice(&bytes[param_start - 2..i]);
                    } else if final_byte == b'G'
                        && matches!(params, b"" | b"0" | b"1")
                    {
                        // Cursor to column 1 (ESC[G, ESC[0G, ESC[1G) — treat as carriage
                        // return so the JS \r-overwrite pass keeps only the last rewrite of
                        // the line (e.g. tab-completion cycling in PowerShell/bash).
                        out.push(b'\r');
                    } else {
                        // Other non-SGR CSI — emit a private-use placeholder (U+E000) so
                        // the JS layer can collapse runs to a single space gap.
                        out.extend_from_slice("\u{E000}".as_bytes());
                    }
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
    // SAFETY: see doc-comment above.
    String::from_utf8(out).unwrap_or_default()
}

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
    fn test_cursor_to_col1_default_emits_cr() {
        assert_eq!(strip_non_sgr("abc\x1b[Gdef"), "abc\rdef");
    }

    #[test]
    fn test_cursor_to_col1_explicit_emits_cr() {
        assert_eq!(strip_non_sgr("abc\x1b[1Gdef"), "abc\rdef");
    }

    #[test]
    fn test_cursor_to_col1_zero_emits_cr() {
        assert_eq!(strip_non_sgr("abc\x1b[0Gdef"), "abc\rdef");
    }

    #[test]
    fn test_cursor_to_col2_emits_placeholder() {
        assert_eq!(strip_non_sgr("abc\x1b[2Gdef"), "abc\u{E000}def");
    }

    #[test]
    fn test_cursor_up_stripped() {
        assert_eq!(strip_non_sgr("a\x1b[Ab"), "a\u{E000}b");
    }

    #[test]
    fn test_cursor_up_with_count_stripped() {
        assert_eq!(strip_non_sgr("a\x1b[3Ab"), "a\u{E000}b");
    }

    #[test]
    fn test_clear_screen_stripped() {
        assert_eq!(strip_non_sgr("a\x1b[2Jb"), "a\u{E000}b");
    }

    #[test]
    fn test_cursor_position_stripped() {
        assert_eq!(strip_non_sgr("a\x1b[1;1Hb"), "a\u{E000}b");
    }

    #[test]
    fn test_osc_title_stripped() {
        assert_eq!(strip_non_sgr("\x1b]0;My Title\x07text"), "text");
    }

    #[test]
    fn test_private_mode_stripped() {
        assert_eq!(strip_non_sgr("\x1b[?25l"), "\u{E000}");
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
            "\x1b[1mhello\u{E000}\x1b[0m"
        );
    }
}
