//! clap's styles on the text form of a refusal, which a sink whose error
//! writer wants styles receives (ADR-0131).

#[cfg(feature = "color")]
use std::fmt::Write as _;

use clap::builder::StyledStr;
use clap::builder::styling::Style;

#[cfg(test)]
mod tests;

/// Returns `plain`, the text form of a refusal that crumbkit composed, with
/// clap's styles: the `error:` label in `error`, the style of clap's own
/// `error:` label, and each piece that clap styles in `clap`, its rendered
/// error, in clap's style for it. A piece of clap's message, such as a
/// refused value or the flag it was given to, is styled in the `error:`
/// line, and a piece of clap's usage is styled in the `usage:` lines.
/// Removing the escape codes from the result gives `plain` back.
///
/// Without crumbkit's `color` feature, clap offers no escape codes, and the
/// text stays plain.
pub(crate) fn text(plain: &str, error: &Style, clap: Option<&StyledStr>) -> String {
    #[cfg(feature = "color")]
    {
        with_styles(plain, error, clap)
    }
    #[cfg(not(feature = "color"))]
    {
        let _ = (error, clap);
        plain.to_owned()
    }
}

/// The label that starts the first line of a refusal's text form.
#[cfg(feature = "color")]
const ERROR: &str = "error:";

/// The start of each line of a refusal's text form that holds one of
/// clap's usage lines.
#[cfg(feature = "color")]
const USAGE: &str = "  usage: ";

/// Styles `plain` as [`text`] describes. The first line takes the pieces
/// of clap's message after its `error:` label, and the block of `usage:`
/// lines takes the pieces of clap's usage.
#[cfg(feature = "color")]
fn with_styles(plain: &str, error: &Style, clap: Option<&StyledStr>) -> String {
    let rendered = clap.map(|clap| clap.ansi().to_string()).unwrap_or_default();
    let pieces = pieces(&rendered);
    let of = |part: Part| pieces.iter().filter(move |piece| piece.part == part);
    let mut text = String::with_capacity(plain.len() * 2);
    let mut rest = plain;
    if let Some(after) = plain.strip_prefix(ERROR) {
        let end = after.find('\n').map_or(after.len(), |end| end + 1);
        // Writing to a `String` never fails.
        let _ = write!(text, "{error}{ERROR}{error:#}");
        apply(&mut text, &after[..end], of(Part::Message));
        rest = &after[end..];
    }
    let start = rest.find(USAGE).unwrap_or(rest.len());
    let block = rest[start..]
        .split_inclusive('\n')
        .take_while(|line| line.starts_with(USAGE))
        .map(str::len)
        .sum::<usize>();
    text.push_str(&rest[..start]);
    apply(&mut text, &rest[start..start + block], of(Part::Usage));
    text.push_str(&rest[start + block..]);
    text
}

/// Appends `region` to `text` with each of `pieces` that `region` holds
/// wrapped in that piece's escape codes. Each piece is looked for after the
/// one before it, in clap's order, and a piece that the rest of `region`
/// does not hold, such as a name that crumbkit replaced, stays unstyled.
//
// RUST-10: this finds clap's styled pieces in crumbkit's text by their
// characters, in order, so a piece whose characters also occur earlier,
// after the piece before it, styles that earlier occurrence. The plain
// bytes stay the same either way. Building the message from the error's
// kind and context, with each piece's style, replaces this once a tool
// sees a misplaced style.
#[cfg(feature = "color")]
fn apply<'a>(text: &mut String, region: &str, pieces: impl Iterator<Item = &'a Piece>) {
    let mut rest = region;
    for piece in pieces {
        let Some(at) = rest.find(piece.text.as_str()) else {
            continue;
        };
        text.push_str(&rest[..at]);
        text.push_str(&piece.open);
        text.push_str(&piece.text);
        text.push_str(&piece.close);
        rest = &rest[at + piece.text.len()..];
    }
    text.push_str(rest);
}

/// The part of clap's rendered error that a styled piece belongs to.
#[cfg(feature = "color")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    /// The message above the usage, which crumbkit prints after `error:`.
    Message,
    /// The usage lines, which crumbkit prints after `usage:`.
    Usage,
    /// A label, such as `error:`, `tip:` or `Usage:`, which crumbkit
    /// replaces or drops, and the closing `For more information` line,
    /// which crumbkit drops.
    Other,
}

/// One piece of clap's rendered error that clap styled.
#[cfg(feature = "color")]
#[derive(Debug)]
struct Piece {
    /// The piece's characters without escape codes.
    text: String,
    /// The escape codes that start the piece's style.
    open: String,
    /// The escape code that ends the piece's style.
    close: String,
    /// The part of the error that the piece belongs to.
    part: Part,
}

/// Returns the pieces that clap styled in `rendered`, its error with ANSI
/// escape codes, in order. Each piece starts at a run of escape codes that
/// set a style and ends at clap's reset code, `ESC [0m`.
//
// RUST-10: this reads only the SGR sequences, `ESC [` up to a final byte,
// that clap's styles write, and treats each piece as one style that the
// reset ends. A crate such as anstyle-parse reads every sequence and
// replaces this if clap starts writing other sequences, such as links.
#[cfg(feature = "color")]
fn pieces(rendered: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut plain = String::with_capacity(rendered.len());
    let mut open = String::new();
    let mut current: Option<(String, usize)> = None;
    let mut rest = rendered;
    while let Some(start) = rest.find("\u{1b}[") {
        let text = &rest[..start];
        plain.push_str(text);
        if let Some((piece, _)) = current.as_mut() {
            piece.push_str(text);
        }
        let after = &rest[start + 2..];
        let end = after
            .find(|c: char| ('@'..='~').contains(&c))
            .map_or(after.len(), |end| end + 1);
        let code = &rest[start..start + 2 + end];
        rest = &after[end..];
        if code == "\u{1b}[0m" {
            if let Some((text, at)) = current.take()
                && !text.is_empty()
            {
                let part = part_at(&plain, at, &text);
                pieces.push(Piece {
                    text,
                    open: std::mem::take(&mut open),
                    close: code.to_owned(),
                    part,
                });
            }
            open.clear();
        } else {
            if current.as_ref().is_some_and(|(text, _)| !text.is_empty()) {
                // A style that starts before the last one was reset ends it.
                open.clear();
            }
            open.push_str(code);
            current = Some((String::new(), plain.len()));
        }
    }
    pieces
}

/// Returns the part of clap's rendered error that a piece `text` at byte
/// `at` of `plain`, the error's text so far, belongs to. The usage starts
/// at the line that starts with `Usage:` and ends at the next blank line,
/// as crumbkit reads clap's usage lines for a refusal.
#[cfg(feature = "color")]
fn part_at(plain: &str, at: usize, text: &str) -> Part {
    if matches!(text, "error:" | "tip:" | "Usage:") {
        return Part::Other;
    }
    let mut part = Part::Message;
    let mut offset = 0;
    for line in plain.split_inclusive('\n') {
        if offset > at {
            break;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("Usage:") {
            part = Part::Usage;
        } else if trimmed.starts_with("For more information")
            || (part == Part::Usage && trimmed.is_empty())
        {
            part = Part::Other;
        }
        offset += line.len();
    }
    part
}
