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
    let starts = [
        format!("<!-- crumbkit:generated {name} -->"),
        format!("# crumbkit:generated {name}"),
    ];
    let ends = [
        format!("<!-- /crumbkit:generated {name} -->"),
        format!("# /crumbkit:generated {name}"),
    ];
    let mut out = String::with_capacity(text.len());
    let mut open: Option<(usize, usize)> = None;
    let mut marked = false;
    let mut changed = false;
    let mut offset = 0;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let trimmed = line.trim();
        match open {
            None => {
                out.push_str(line);
                if starts.iter().any(|start| start == trimmed) {
                    open = Some((index + 1, offset + line.len()));
                }
            }
            Some((_, from)) if ends.iter().any(|end| end == trimmed) => {
                changed |= &text[from..offset] != body;
                marked = true;
                out.push_str(body);
                out.push_str(line);
                open = None;
            }
            Some(_) => {}
        }
        offset += line.len();
    }
    if let Some((line, _)) = open {
        return Err(Unclosed { line });
    }
    Ok(match (marked, changed) {
        (false, _) => Rewrite::Unmarked,
        (true, false) => Rewrite::Unchanged,
        (true, true) => Rewrite::Changed(out),
    })
}
