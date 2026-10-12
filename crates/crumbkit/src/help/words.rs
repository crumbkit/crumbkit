//! The quoting of the words of a command line that crumbkit prints, and the
//! reading of such a line back into its words.

#[cfg(test)]
mod tests;

/// Returns `word` as a POSIX shell reads it back as one argument: as it is
/// when it holds only ASCII letters, digits and `-_./:,=+@%`, and otherwise
/// between single quotes, with each `'` in it written as `'\''`. An empty
/// word prints as `''`.
///
/// A leaf's examples, a [`Call`](crate::Call)'s words and the command
/// lines of crumbkit-aliases quote each word with it, so every command
/// line that crumbkit prints quotes in the same way. [`split`] reads such a
/// line back.
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

/// Returns the words of `line`, a command line whose words [`quote`] wrote
/// and spaces separate, as a POSIX shell reads them, or `None` when `line`
/// holds a quote that does not close or a backslash outside quotes that is
/// not followed by `'`.
///
/// Spaces outside quotes separate words. Text between single quotes is
/// part of the word as it is, and `\'` outside quotes is a `'`. Every other
/// character outside quotes is part of the word, so a placeholder of a
/// call, such as `<ID>...`, is one word. crumbkit-test reads each `run`
/// recovery with it, so it runs the words that the recovery names.
///
/// ```
/// use crumbkit::help::split;
///
/// assert_eq!(
///     split("shelf books list --author 'Jane Austen'"),
///     Some(vec!["shelf".into(), "books".into(), "list".into(), "--author".into(), "Jane Austen".into()])
/// );
/// ```
//
// RUST-10: the reading covers what `quote` writes and nothing more. It
// misses double quotes, backslashes before other characters and expansions,
// which a shell reads and `quote` never writes. A crate such as shlex
// replaces it if crumbkit reads lines that a person typed.
#[must_use]
pub fn split(line: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    // The word being read, or `None` between words.
    let mut word: Option<String> = None;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            ' ' => {
                if let Some(done) = word.take() {
                    words.push(done);
                }
            }
            '\'' => {
                let current = word.get_or_insert_with(String::new);
                loop {
                    match chars.next()? {
                        '\'' => break,
                        inside => current.push(inside),
                    }
                }
            }
            '\\' => {
                if chars.next()? != '\'' {
                    return None;
                }
                word.get_or_insert_with(String::new).push('\'');
            }
            other => word.get_or_insert_with(String::new).push(other),
        }
    }
    words.extend(word);
    Some(words)
}
