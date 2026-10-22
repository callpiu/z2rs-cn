//! Zelda II dialog text codec.
//!
//! Ported (copied and adapted) from `z2-rando::text`: the same glyph map,
//! controls and box limits, without the ROM table reader/writer (new areas
//! carry their own messages; the engine shows them from a buffer).
//!
//! | text | byte(s) |
//! |---|---|
//! | `0`-`9` | `$D0`-`$D9` |
//! | `A`-`Z` | `$DA`-`$F3` (input is upper-cased) |
//! | space | `$F4` |
//! | `-` | `$F6` |
//! | `.` | `$CF` |
//! | `/` | `$CE` |
//! | `,` | `$9C` |
//! | `!` | `$36` |
//! | `?` | `$34` |
//! | `*` | `$32` |
//! | line break (`\n` or `$`) | `$FD` |
//! | pause (`~`) | `$FE` |
//! | end of message | `$FF` |
//!
//! Any other byte round-trips as a `{XX}` hex escape, and `{XX}` in input
//! emits that byte. A box holds [`MAX_LINES`] lines of [`MAX_LINE_CHARS`]
//! characters; a pause counts as a line break.

use crate::ContentError;

/// Line break control byte.
pub const NEWLINE: u8 = 0xFD;
/// Pause control byte.
pub const PAUSE: u8 = 0xFE;
/// End-of-message byte.
pub const END: u8 = 0xFF;
/// Space glyph.
pub const SPACE: u8 = 0xF4;
/// Widest dialog line the box shows, in characters.
pub const MAX_LINE_CHARS: usize = 11;
/// Lines the dialog box shows at once.
pub const MAX_LINES: usize = 4;

/// Game byte for one printable character, if it has one.
#[must_use]
pub fn char_to_byte(c: char) -> Option<u8> {
    let c = c.to_ascii_uppercase();
    Some(match c {
        '0'..='9' => 0xD0 + (c as u8 - b'0'),
        'A'..='Z' => 0xDA + (c as u8 - b'A'),
        ' ' => SPACE,
        '-' => 0xF6,
        '.' => 0xCF,
        '/' => 0xCE,
        ',' => 0x9C,
        '!' => 0x36,
        '?' => 0x34,
        '*' => 0x32,
        _ => return None,
    })
}

/// Printable character for one game byte (controls and unknown glyphs
/// return `None`).
#[must_use]
pub fn byte_to_char(b: u8) -> Option<char> {
    Some(match b {
        0xD0..=0xD9 => (b'0' + (b - 0xD0)) as char,
        0xDA..=0xF3 => (b'A' + (b - 0xDA)) as char,
        SPACE => ' ',
        0xF6 => '-',
        0xCF => '.',
        0xCE => '/',
        0x9C => ',',
        0x36 => '!',
        0x34 => '?',
        0x32 => '*',
        _ => return None,
    })
}

/// Encode `s` into game bytes, **without** the end byte. `\n` or `$` is a
/// line break, `~` a pause, `{XX}` a raw byte.
pub fn encode(s: &str) -> Result<Vec<u8>, ContentError> {
    let mut out = Vec::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' | '$' => out.push(NEWLINE),
            '~' => out.push(PAUSE),
            '{' => {
                let hex: String = chars.by_ref().take(2).collect();
                if chars.next() != Some('}') || hex.len() != 2 {
                    return Err(ContentError::Text(format!("bad escape in {s:?}")));
                }
                out.push(
                    u8::from_str_radix(&hex, 16)
                        .map_err(|_| ContentError::Text(format!("bad escape {{{hex}}}")))?,
                );
            }
            _ => out.push(char_to_byte(c).ok_or_else(|| {
                ContentError::Text(format!("character {c:?} has no glyph in the dialog font"))
            })?),
        }
    }
    Ok(out)
}

/// [`encode`] plus the end byte.
pub fn encode_message(s: &str) -> Result<Vec<u8>, ContentError> {
    let mut v = encode(s)?;
    v.push(END);
    Ok(v)
}

/// Decode game bytes up to (not including) the first end byte. Line breaks
/// become `\n`, pauses `~`, unknown bytes `{XX}`.
#[must_use]
pub fn decode(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            END => break,
            NEWLINE => s.push('\n'),
            PAUSE => s.push('~'),
            _ => match byte_to_char(b) {
                Some(c) => s.push(c),
                None => s.push_str(&format!("{{{b:02X}}}")),
            },
        }
    }
    s
}

/// Check that `s` fits a dialog box: at most `max_lines` lines of at most
/// [`MAX_LINE_CHARS`] characters (a pause is a line break).
pub fn validate_dialog(s: &str, max_lines: usize) -> Result<(), ContentError> {
    let lines: Vec<&str> = s.split(['\n', '$', '~']).collect();
    if lines.len() > max_lines {
        return Err(ContentError::Text(format!(
            "{s:?}: {} lines, the box holds {max_lines}",
            lines.len()
        )));
    }
    for l in lines {
        let width = l.chars().count() - 3 * l.matches('{').count();
        if width > MAX_LINE_CHARS {
            return Err(ContentError::Text(format!(
                "{l:?} is {width} characters, a line holds {MAX_LINE_CHARS}"
            )));
        }
    }
    Ok(())
}

/// Word-wrap plain text into dialog lines of at most [`MAX_LINE_CHARS`]
/// characters joined with `\n`. Returns `None` when a word is too long or
/// the result needs more than `max_lines` lines.
#[must_use]
pub fn wrap(text: &str, max_lines: usize) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        if word.chars().count() > MAX_LINE_CHARS {
            return None;
        }
        match lines.last_mut() {
            Some(l) if l.chars().count() + 1 + word.chars().count() <= MAX_LINE_CHARS => {
                l.push(' ');
                l.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    if lines.is_empty() || lines.len() > max_lines {
        return None;
    }
    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_digits_and_punctuation() {
        assert_eq!(encode("A").unwrap(), vec![0xDA]);
        assert_eq!(encode("z").unwrap(), vec![0xF3]);
        assert_eq!(encode("0 9").unwrap(), vec![0xD0, SPACE, 0xD9]);
        assert_eq!(
            encode("-./,!?*").unwrap(),
            vec![0xF6, 0xCF, 0xCE, 0x9C, 0x36, 0x34, 0x32]
        );
        assert_eq!(
            encode("A\nB$C~D").unwrap(),
            vec![0xDA, NEWLINE, 0xDB, NEWLINE, 0xDC, PAUSE, 0xDD]
        );
        assert_eq!(encode_message("HI").unwrap(), vec![0xE1, 0xE2, END]);
    }

    #[test]
    fn round_trip_including_escapes() {
        let s = "WELL MET,\nTRAVELER!~{CA}{05} 42";
        assert_eq!(decode(&encode(s).unwrap()), s);
        let all: Vec<u8> = (0..=0xFEu8).collect();
        assert_eq!(encode(&decode(&all)).unwrap(), all);
    }

    #[test]
    fn rejects_unknown_characters_and_bad_escapes() {
        assert!(encode("#").is_err());
        assert!(encode("{G1}").is_err());
        assert!(encode("{1").is_err());
    }

    #[test]
    fn dialog_size_limits() {
        assert!(validate_dialog("ABCDEFGHIJK\nB\nC\nD", MAX_LINES).is_ok());
        assert!(validate_dialog("ABCDEFGHIJKL", MAX_LINES).is_err());
        assert!(validate_dialog("A\nB\nC\nD\nE", MAX_LINES).is_err());
        assert!(validate_dialog("A\nB\nC\nD~E", MAX_LINES).is_err());
        assert!(validate_dialog("{CA}BCDEFGHIJK", MAX_LINES).is_ok());
        assert_eq!(
            wrap("THE OLD WELL IS DRY", 4).as_deref(),
            Some("THE OLD\nWELL IS DRY")
        );
        assert_eq!(wrap("ANTIDISESTABLISH", 4), None);
    }
}
