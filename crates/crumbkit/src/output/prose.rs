//! How `finish` prints the text of a prose result.

use std::fmt::{self, Write as _};
use std::io;

/// Returns the text that a prose result prints: `prose` with each control
/// character other than a line feed or a tab as its `char::escape_default`
/// form, such as `\r` or `\u{1b}`, and then a line feed when the text does
/// not end with one. An empty text stays empty. `finish` builds it once, so
/// it can measure the text and then write it.
///
/// The table escapes a line feed as well, because a row stays on one line,
/// and prose keeps its line feeds and tabs because they lay out its text.
pub(super) fn text(prose: &dyn fmt::Display) -> io::Result<String> {
    let mut text = String::new();
    // Writing into a `String` fails only when the `Display` impl itself
    // returns an error, which a correct impl never does.
    write!(text, "{prose}").map_err(io::Error::other)?;
    if text.is_empty() {
        return Ok(text);
    }
    let mut escaped = String::with_capacity(text.len() + 1);
    for character in text.chars() {
        if character.is_control() && character != '\n' && character != '\t' {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }
    if !escaped.ends_with('\n') {
        escaped.push('\n');
    }
    Ok(escaped)
}
