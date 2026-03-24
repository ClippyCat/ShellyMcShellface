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
