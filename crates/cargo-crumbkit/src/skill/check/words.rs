//! Splits a command line from a skill file into words, as a POSIX shell
//! would for a simple command.

#[cfg(test)]
mod tests;

/// The reason that a command line with an unclosed quote cannot be split.
const UNCLOSED: &str = "the command line has an unclosed quote";

/// Returns the words of the first simple command in `line`, such as
/// `["shelf", "books", "show", "Jane Austen"]` for `shelf books show 'Jane
/// Austen' | head`, or the reason that the line cannot be split.
///
/// Spaces and tabs separate words. Single quotes keep every character,
/// double quotes keep every character except that a backslash escapes `"`,
/// `\`, `$` and a backtick, and a backslash outside quotes keeps the next
/// character. An unquoted `|`, `&` or `;` ends the command, and so does a `#`
/// that starts a word. A word that starts with `<` and a letter runs to the
/// next `>`, so a placeholder such as `<file name>` stays one word, and so
/// does one after `=`, as in `--format=<FORMAT>`. Any other
/// unquoted `<` or `>` starts a redirection and ends the command, and a word
/// of digits just before a `>`, such as the `2` of `2>/dev/null`, is a file
/// descriptor and is dropped.
//
// RUST-10: this reads the quoting and operators that skill files use in
// command examples. It does not expand variables, globs or command
// substitutions such as `$(...)`, which stay in a word as written, and it
// reads only the first command of a line. A line that continues onto the
// next with a trailing backslash is read as two lines. A maintained
// shell-words crate would add a dependency for the same quoting, and still
// not end at operators or keep placeholders whole.
pub(crate) fn split(line: &str) -> Result<Vec<String>, &'static str> {
    let mut words = Vec::new();
    let mut word: Option<String> = None;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => words.extend(word.take()),
            '\'' => {
                let text = word.get_or_insert_with(String::new);
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => text.push(c),
                        None => return Err(UNCLOSED),
                    }
                }
            }
            '"' => {
                let text = word.get_or_insert_with(String::new);
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(c @ ('"' | '\\' | '$' | '`')) => text.push(c),
                            Some(c) => {
                                text.push('\\');
                                text.push(c);
                            }
                            None => return Err(UNCLOSED),
                        },
                        Some(c) => text.push(c),
                        None => return Err(UNCLOSED),
                    }
                }
            }
            '\\' => {
                let text = word.get_or_insert_with(String::new);
                if let Some(c) = chars.next() {
                    text.push(c);
                }
            }
            '|' | '&' | ';' => break,
            '#' if word.is_none() => break,
            '<' if word.as_deref().is_none_or(|text| text.ends_with('='))
                && chars.peek().is_some_and(char::is_ascii_alphabetic) =>
            {
                let text = word.get_or_insert_with(String::new);
                text.push('<');
                for c in chars.by_ref() {
                    text.push(c);
                    if c == '>' {
                        break;
                    }
                }
            }
            '<' | '>' => {
                if word
                    .as_deref()
                    .is_some_and(|text| text.bytes().all(|byte| byte.is_ascii_digit()))
                {
                    word = None;
                }
                break;
            }
            c => word.get_or_insert_with(String::new).push(c),
        }
    }
    words.extend(word);
    Ok(words)
}
