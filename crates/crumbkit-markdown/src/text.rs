//! The `CommonMark` helpers that write the reference: escaped paragraphs,
//! code spans, code fences, heading anchors and sentences.

#[cfg(test)]
mod tests;

/// The characters that start inline markup in `CommonMark`, such as
/// emphasis, links, raw HTML and entities, and that the reference escapes
/// with a backslash in help text. A backtick is not among them, because
/// help text writes code spans with backticks and the reference keeps them.
const INLINE: [char; 9] = ['\\', '*', '_', '[', ']', '<', '>', '&', '~'];

/// The characters that start a block in `CommonMark` when a line starts with
/// them, such as a heading, a list item or a setext underline. `*` and `>`
/// are escaped wherever they appear, so they are not here.
const BLOCK: [char; 4] = ['#', '-', '+', '='];

/// Returns the paragraphs of `text`, help text written as plain text, as
/// `CommonMark` that prints the same words: each paragraph's lines without
/// their indentation, with each markup character escaped and each code
/// span kept. A blank line separates paragraphs, and text with only blank
/// lines has none.
//
// RUST-10: the escaping covers the markup that help text can start by
// accident: inline markup, block markers at the start of a line, a code
// fence, and indentation, which would start a code block. It keeps code
// spans, because crumbkit's help writes them, and it keeps the line breaks
// of the help text, which `CommonMark` joins into one paragraph. A tool whose
// help holds deliberate Markdown, such as lists, loses that layout, and a
// renderer that takes Markdown from the help itself replaces this.
pub(crate) fn paragraphs(text: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            if !lines.is_empty() {
                paragraphs.push(lines.join("\n"));
                lines.clear();
            }
            continue;
        }
        lines.push(escape_line(line));
    }
    if !lines.is_empty() {
        paragraphs.push(lines.join("\n"));
    }
    paragraphs
}

/// Returns `text` as one line of `CommonMark`: its paragraphs joined with
/// spaces, for a place that holds one line, such as a list item that names
/// a child command.
pub(crate) fn one_line(text: &str) -> String {
    paragraphs(text).join(" ").replace('\n', " ")
}

/// Returns `line`, which has no indentation, with a backslash before a block
/// marker at its start and before each inline markup character outside a
/// code span.
fn escape_line(line: &str) -> String {
    let digits = line.len() - line.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let after = line[digits..].chars().next();
    if (1..=9).contains(&digits) && matches!(after, Some('.' | ')')) {
        // An ordered list item starts with up to nine digits and `.` or `)`.
        return format!("{}\\{}", &line[..digits], inline(&line[digits..]));
    }
    if line.starts_with(BLOCK) || line.starts_with("```") {
        return format!("\\{}", inline(line));
    }
    inline(line)
}

/// Returns `text` with a backslash before each character of [`INLINE`]
/// outside a code span. A code span is a run of backticks, the text after
/// it, and the next run of exactly as many backticks, as `CommonMark` defines
/// it. A run of backticks that no such run closes prints as written, so it
/// is kept as well.
fn inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '`' {
            let run = backticks_at(rest);
            let span = closing(&rest[run..], run).map_or(run, |end| run + end + run);
            out.push_str(&rest[..span]);
            rest = &rest[span..];
            continue;
        }
        if INLINE.contains(&c) {
            out.push('\\');
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// Returns the number of backticks at the start of `text`.
fn backticks_at(text: &str) -> usize {
    text.len() - text.trim_start_matches('`').len()
}

/// Returns the byte offset in `text` of the first run of exactly `run`
/// backticks, or `None` when there is none.
fn closing(text: &str, run: usize) -> Option<usize> {
    let mut offset = 0;
    while let Some(start) = text[offset..].find('`') {
        let at = offset + start;
        let length = backticks_at(&text[at..]);
        if length == run {
            return Some(at);
        }
        offset = at + length;
    }
    None
}

/// Returns the length of the longest run of backticks in `text`.
fn longest_run(text: &str) -> usize {
    text.split(|c| c != '`').map(str::len).max().unwrap_or(0)
}

/// Returns `text` as a `CommonMark` code span, which prints it as written:
/// between runs of one more backtick than the longest run inside it, with a
/// space inside each run when the text starts or ends with a backtick.
pub(crate) fn code(text: &str) -> String {
    let ticks = "`".repeat(longest_run(text) + 1);
    if text.starts_with('`') || text.ends_with('`') {
        format!("{ticks} {text} {ticks}")
    } else {
        format!("{ticks}{text}{ticks}")
    }
}

/// Returns the code fence of a block that holds `text`: three backticks, or
/// one more than the longest run of backticks in the text.
pub(crate) fn fence(text: &str) -> String {
    "`".repeat((longest_run(text) + 1).max(3))
}

/// Returns the anchor of a heading whose text is `text`.
//
// RUST-10: `CommonMark` gives headings no ids, so the anchor follows GitHub's
// rule, which GitLab, mdBook and VS Code share for these names: the text in
// lowercase, with letters, digits, `_` and `-` kept, each space as `-`, and
// every other character dropped. GitHub adds `-1` to the second heading
// with an anchor that an earlier heading has, so `render` refuses two
// commands with one anchor. A section heading such as `Exit codes` comes
// before the command headings of its tool's subcommands, so a command whose
// anchor is `exit-codes` would get `exit-codes-1` on GitHub. Explicit HTML
// anchors replace this rule if a renderer with another rule matters.
pub(crate) fn anchor(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

/// Returns `text` ending with a full stop, unless it already ends with
/// `.`, `!`, `?` or `:`, so a description reads as a sentence.
pub(crate) fn sentence(text: &str) -> String {
    let text = text.trim_end();
    if text.ends_with(['.', '!', '?', ':']) {
        text.to_owned()
    } else {
        format!("{text}.")
    }
}
