//! The quoting of the words of a command line that crumbkit prints.

#[cfg(test)]
mod tests;

/// Returns `word` as a POSIX shell reads it back as one argument: as it is
/// when it holds only ASCII letters, digits and `-_./:,=+@%`, and otherwise
/// between single quotes, with each `'` in it written as `'\''`. An empty
/// word prints as `''`.
///
/// A leaf's examples, a [`Call`](crate::Call)'s words and the command
/// lines of crumbkit-aliases quote each word with it, so every command
/// line that crumbkit prints quotes in the same way. crumbkit-test reads
/// such a line back when it follows a recovery.
///
/// ```
/// use crumbkit::help::quote;
///
/// assert_eq!(quote("--author"), "--author");
/// assert_eq!(quote("Jane Austen"), "'Jane Austen'");
/// ```
//
// RUST-10: the quoting suits POSIX shells, which crumbkit's supported
// platforms use. It misses other shells, such as PowerShell or cmd.exe,
// which read single quotes and backslashes differently. A rendering for
// each shell replaces it if crumbkit supports Windows.
#[must_use]
pub fn quote(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:,=+@%".contains(c));
    if plain {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}
