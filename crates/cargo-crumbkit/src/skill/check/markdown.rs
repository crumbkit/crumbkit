//! Finds the code in a skill file: each inline code span and each line of a
//! fenced code block.

#[cfg(test)]
mod tests;

/// One piece of code in a Markdown file.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Code {
    /// The line number of the code, counted from 1.
    pub(crate) line: usize,
    /// The code, without its backticks and without the spaces around a
    /// line of a fenced block.
    pub(crate) text: String,
}

/// An open fenced code block: the character of its fence, a backtick or a
/// tilde, and the length of the fence.
type Fence = (char, usize);

/// Returns every inline code span and every non-empty line of every fenced
/// code block in `text`, in the order of the file.
///
/// A fence is a line that starts, after at most three spaces, with three or
/// more backticks or tildes. The block ends at a line of the same character
/// at least as long and nothing else, or at the end of the file. An inline
/// code span starts with a run of backticks and ends at the next run of the
/// same length, and a span with a space at both ends loses one at each end,
/// as `CommonMark` says.
//
// RUST-10: this reads the two forms of code that a skill file uses for
// command lines. It misses indented code blocks, a fence indented by four
// or more spaces, as in a code block inside an ordered list, a code span
// that runs over a line break, and the CommonMark rules that turn a backtick into text,
// such as a backslash before it or a span inside an HTML block. pulldown-cmark
// reads all of these and replaces this function if a skill file needs them.
pub(crate) fn code(text: &str) -> Vec<Code> {
    let mut found = Vec::new();
    let mut fence: Option<Fence> = None;
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if let Some(open) = fence {
            if closes(line, open) {
                fence = None;
            } else if !line.trim().is_empty() {
                found.push(Code {
                    line: number,
                    text: line.trim().to_owned(),
                });
            }
            continue;
        }
        if let Some(open) = opens(line) {
            fence = Some(open);
            continue;
        }
        found.extend(
            spans(line)
                .into_iter()
                .map(|text| Code { line: number, text }),
        );
    }
    found
}

/// Returns the fence that `line` opens, or `None` when it opens none.
fn opens(line: &str) -> Option<Fence> {
    let (mark, length, rest) = fence_run(line)?;
    // A backtick fence's info string holds no backtick, so a line such as
    // ```` ```a``` ```` is an inline span.
    (mark == '~' || !rest.contains('`')).then_some((mark, length))
}

/// Reports whether `line` closes the block that `open` opened.
fn closes(line: &str, open: Fence) -> bool {
    fence_run(line).is_some_and(|(mark, length, rest)| {
        mark == open.0 && length >= open.1 && rest.trim().is_empty()
    })
}

/// Returns the fence character, the length of the run and the rest of the
/// line, when `line` starts with a run of three or more backticks or tildes
/// after at most three spaces.
fn fence_run(line: &str) -> Option<(char, usize, &str)> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let mark = trimmed.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let length = trimmed.chars().take_while(|&c| c == mark).count();
    (length >= 3).then(|| (mark, length, &trimmed[length..]))
}

/// Returns the text of each inline code span in `line`.
fn spans(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('`') {
        let after = &rest[start..];
        let length = after.chars().take_while(|&c| c == '`').count();
        let body = &after[length..];
        match closing(body, length) {
            Some(end) => {
                found.push(strip(&body[..end]).to_owned());
                rest = &body[end + length..];
            }
            None => rest = body,
        }
    }
    found
}

/// Returns the byte offset in `body` of the first run of exactly `length`
/// backticks, which closes a span that a run of `length` opened.
fn closing(body: &str, length: usize) -> Option<usize> {
    let mut offset = 0;
    while let Some(start) = body[offset..].find('`') {
        let start = offset + start;
        let run = body[start..].chars().take_while(|&c| c == '`').count();
        if run == length {
            return Some(start);
        }
        offset = start + run;
    }
    None
}

/// Returns `text` without one space at each end, when it has a space at both
/// ends and is not only spaces.
fn strip(text: &str) -> &str {
    if text.len() >= 2 && text.starts_with(' ') && text.ends_with(' ') && !text.trim().is_empty() {
        &text[1..text.len() - 1]
    } else {
        text
    }
}
