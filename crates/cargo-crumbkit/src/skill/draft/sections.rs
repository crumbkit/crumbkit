//! The generated sections of a skill file: the lines between a pair of
//! markers that `skill draft` rewrites, leaving every other byte alone
//! (ADR-0194).
//!
//! A section named `jobs` starts with a line that holds only
//! `<!-- crumbkit:generated jobs -->` and ends with a line that holds only
//! `<!-- /crumbkit:generated jobs -->`. In YAML frontmatter, where an HTML
//! comment would be a value, the markers are YAML comments:
//! `# crumbkit:generated description` and
//! `# /crumbkit:generated description`. Spaces around a marker are allowed.
//!
//! The YAML form counts only inside the frontmatter, which starts with a
//! `---` first line and ends at the next `---` line, and the HTML form only
//! outside it, because `# ...` in the body is a heading. A marker inside a
//! fenced code block is an example and does not count.

use crate::skill::check::markdown::{self, Fence};

#[cfg(test)]
mod tests;

/// What rewriting one section does to a skill file.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Rewrite {
    /// The file has no markers for the section.
    Unmarked,
    /// Every pair of markers already holds the body.
    Unchanged,
    /// The file with the body between every pair of markers.
    Changed(String),
}

/// A start marker that no end marker with its name follows.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Unclosed {
    /// The line of the start marker, counted from 1.
    pub(crate) line: usize,
}

/// Returns `text` with `body` between every pair of markers of the section
/// called `name`. The lines of the markers and every line outside the pairs
/// keep their bytes, line endings included.
///
/// # Errors
///
/// Returns [`Unclosed`] when a start marker has no end marker after it.
pub(crate) fn rewrite(text: &str, name: &str, body: &str) -> Result<Rewrite, Unclosed> {
    let html = (
        format!("<!-- crumbkit:generated {name} -->"),
        format!("<!-- /crumbkit:generated {name} -->"),
    );
    let yaml = (
        format!("# crumbkit:generated {name}"),
        format!("# /crumbkit:generated {name}"),
    );
    let mut out = String::with_capacity(text.len());
    // The open section: the line of its start marker, the offset where its
    // body starts, and the body with the start marker's line ending.
    let mut open: Option<(usize, usize, String)> = None;
    let mut place = Place::Body;
    let mut marked = false;
    let mut changed = false;
    let mut offset = 0;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let trimmed = line.trim();
        let markers = match place {
            Place::Body => Some(&html),
            Place::Frontmatter => Some(&yaml),
            Place::Fenced(_) => None,
        };
        match &open {
            None => {
                out.push_str(line);
                if markers.is_some_and(|(start, _)| start == trimmed) {
                    let body = if line.ends_with("\r\n") {
                        body.replace('\n', "\r\n")
                    } else {
                        body.to_owned()
                    };
                    open = Some((index + 1, offset + line.len(), body));
                }
            }
            Some((_, from, body)) if markers.is_some_and(|(_, end)| end == trimmed) => {
                changed |= &text[*from..offset] != body;
                marked = true;
                out.push_str(body);
                out.push_str(line);
                open = None;
            }
            Some(_) => {}
        }
        place = place.after(index, line);
        offset += line.len();
    }
    if let Some((line, _, _)) = open {
        return Err(Unclosed { line });
    }
    Ok(match (marked, changed) {
        (false, _) => Rewrite::Unmarked,
        (true, false) => Rewrite::Unchanged,
        (true, true) => Rewrite::Changed(out),
    })
}

/// Where a line of a skill file sits, which decides the markers it may hold.
#[derive(Clone, Copy)]
enum Place {
    /// Inside the YAML frontmatter.
    Frontmatter,
    /// In the Markdown body, outside a fenced code block.
    Body,
    /// Inside a fenced code block that the fence opened.
    Fenced(Fence),
}

impl Place {
    /// Returns where the line after `line`, the line at `index` counted
    /// from 0, sits.
    fn after(self, index: usize, line: &str) -> Self {
        let line = line.trim_end_matches(['\r', '\n']);
        let trimmed = line.trim();
        match self {
            Self::Body if index == 0 && trimmed == "---" => Self::Frontmatter,
            Self::Frontmatter if trimmed == "---" => Self::Body,
            Self::Frontmatter => Self::Frontmatter,
            Self::Body => markdown::opens(line).map_or(Self::Body, Self::Fenced),
            Self::Fenced(fence) if markdown::closes(line, fence) => Self::Body,
            Self::Fenced(fence) => Self::Fenced(fence),
        }
    }
}
