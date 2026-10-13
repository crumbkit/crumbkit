//! The reading of a recovery's command line back into its words.

#[cfg(test)]
mod tests;

/// Returns the words of `line`, a command line whose words spaces separate,
/// as a POSIX shell reads them, or `None` when `line` holds a quote that
/// does not close, a backslash outside quotes that is not followed by `'`,
/// or a `$` or a backtick between double quotes that is not escaped.
///
/// Spaces outside quotes separate words. Text between single quotes is
/// part of the word as it is, and `\'` outside quotes is a `'`. Text between
/// double quotes is part of the word too, except that a backslash before
/// `"`, `\`, `$` or a backtick stands for that character and a backslash
/// before a line break removes both. A `$` or a backtick between double
/// quotes would make a shell expand the text, so `split` refuses it. Every
/// other character outside quotes is part of the word, so a placeholder of
/// a call, such as `<ID>...`, is one word. `crumbkit::help::quote` writes
/// only single quotes, and a tool that writes its hints with double quotes,
/// such as
/// `shelf books list --author "Jane Austen"`, is read the same way.
/// [`check_recoveries`](super::check_recoveries) reads each `run`
/// recovery with it, so it runs the words that the recovery names.
//
// RUST-10: the reading covers what `help::quote` writes and double-quoted
// words. It misses backslashes outside quotes before characters other than
// `'`, and every expansion, which it refuses. A crate such as shlex replaces it
// if crumbkit-test reads lines that a person typed.
#[must_use]
pub(super) fn split(line: &str) -> Option<Vec<String>> {
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
            '"' => {
                let current = word.get_or_insert_with(String::new);
                loop {
                    match chars.next()? {
                        '"' => break,
                        '$' | '`' => return None,
                        '\\' => match chars.next()? {
                            '\n' => {}
                            escaped @ ('"' | '\\' | '$' | '`') => current.push(escaped),
                            other => {
                                current.push('\\');
                                current.push(other);
                            }
                        },
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
