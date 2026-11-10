//! The skill that a tool embeds, and the frontmatter lines that mark the
//! copy that `install` writes as the tool's own.

use std::fmt::Write as _;

#[cfg(test)]
mod tests;

/// A tool's agent skill: the tool's name, its version and the text of its
/// `SKILL.md`, which the tool embeds with `include_str!`.
///
/// `install` writes the text to `<dir>/<name>/SKILL.md` with two lines
/// added to its frontmatter under `metadata:`, which name the tool and the
/// version in quoted YAML strings:
///
/// ```text
/// metadata:
///   tool: "shelf"
///   version: "0.3.0"
/// ```
///
/// The lines tell `install` that a later copy at that path is its own, so
/// it replaces that copy and leaves every other file alone. An agent still
/// reads `name` and `description` as before, and quoted values keep the
/// YAML valid whatever the version holds, such as build metadata.
///
/// ```
/// use crumbkit_skills::Skill;
///
/// const SKILL: Skill = Skill::new(
///     "shelf",
///     "0.3.0",
///     "---\nname: shelf\ndescription: Browse the catalogue.\n---\n\nRun `shelf books list`.\n",
/// );
/// # let _ = SKILL;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Skill {
    /// The tool's name, which names the skill's folder.
    name: &'static str,
    /// The tool's version, which the installed copy records.
    version: &'static str,
    /// The text of the tool's `SKILL.md`.
    text: &'static str,
}

impl Skill {
    /// Returns the skill of the tool named `name` at `version`, whose
    /// `SKILL.md` is `text`.
    ///
    /// # Panics
    ///
    /// Panics unless `name` is lowercase ASCII letters, digits and dashes
    /// that start with a letter, `version` is one non-empty line of
    /// printable ASCII without a quote or a backslash, and `text` starts
    /// with a frontmatter block between two `---` lines that has no
    /// top-level `metadata:` key, because `install` adds that key. A `const`
    /// that calls it with such arguments fails to compile:
    ///
    /// ```compile_fail
    /// const BAD: crumbkit_skills::Skill =
    ///     crumbkit_skills::Skill::new("shelf", "0.3.0", "No frontmatter.\n");
    /// ```
    #[must_use]
    pub const fn new(name: &'static str, version: &'static str, text: &'static str) -> Self {
        assert!(
            is_name(name),
            "a skill name is lowercase ASCII letters, digits and dashes, starting with a letter"
        );
        assert!(
            is_version(version),
            "a skill version is one non-empty line of printable ASCII without a quote or a backslash"
        );
        assert!(
            frontmatter_end(text.as_bytes()).is_some(),
            "a skill text starts with a frontmatter block between two `---` lines"
        );
        assert!(
            !has_metadata(text.as_bytes()),
            "a skill's frontmatter has no `metadata:` key, because install adds it"
        );
        Self {
            name,
            version,
            text,
        }
    }

    /// Returns the tool's name, such as `shelf`.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the tool's version, such as `0.3.0`.
    #[must_use]
    pub const fn version(&self) -> &'static str {
        self.version
    }

    /// Returns the text of the tool's `SKILL.md`, as the tool embeds it.
    #[must_use]
    pub const fn text(&self) -> &'static str {
        self.text
    }

    /// Returns the text that `install` writes: the skill's text with the
    /// `metadata:` lines that name the tool and the version added at the
    /// end of its frontmatter.
    pub(crate) fn installed(&self) -> String {
        let end = frontmatter_end(self.text.as_bytes()).unwrap_or(0);
        let (head, rest) = self.text.split_at_checked(end).unwrap_or(("", self.text));
        let mut text = String::with_capacity(self.text.len() + 64);
        text.push_str(head);
        // Writing to a `String` cannot fail.
        let _ = write!(
            text,
            "metadata:\n  tool: \"{}\"\n  version: \"{}\"\n",
            self.name, self.version
        );
        text.push_str(rest);
        text
    }

    /// Returns who wrote `text`, an installed `SKILL.md`, by the
    /// `metadata:` lines of its frontmatter.
    ///
    /// RUST-10: it reads only the lines that `install` writes, `metadata:`
    /// at the start of a line and `  tool: "<name>"` and
    /// `  version: "<version>"` indented by two spaces below it, and is no
    /// YAML parser. A copy that a person reformats, such as with other
    /// indentation, counts as written by someone else, and `--force`
    /// replaces it. A YAML parser replaces this once a skill needs to read
    /// other frontmatter.
    pub(crate) fn writer(&self, text: &str) -> Writer {
        let mut lines = text.lines();
        if lines.next() != Some("---") {
            return Writer::Other;
        }
        let mut in_metadata = false;
        let mut tool = None;
        let mut version = None;
        for line in lines {
            if line == "---" {
                break;
            }
            if line == "metadata:" {
                in_metadata = true;
            } else if !line.starts_with(' ') {
                in_metadata = false;
            } else if in_metadata {
                if let Some(value) = quoted(line, "  tool: ") {
                    tool = Some(value);
                } else if let Some(value) = quoted(line, "  version: ") {
                    version = Some(value);
                }
            }
        }
        match (tool, version) {
            (Some(tool), Some(version)) if tool == self.name => Writer::Tool {
                version: version.to_owned(),
            },
            _ => Writer::Other,
        }
    }
}

/// Who wrote an installed `SKILL.md`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Writer {
    /// This tool's `install`, at `version`.
    Tool {
        /// The version that the copy records.
        version: String,
    },
    /// Someone else: a person, another tool or another program.
    Other,
}

/// Returns the text between the quotes of `line` after `key`, or `None`
/// when `line` does not start with `key` and a quoted value.
fn quoted<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(key)?.strip_prefix('"')?.strip_suffix('"')
}

/// Reports whether `name` is lowercase ASCII letters, digits and dashes,
/// starting with a letter.
const fn is_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_lowercase() {
        return false;
    }
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-') {
            return false;
        }
        index += 1;
    }
    true
}

/// Reports whether `version` is one non-empty line of printable ASCII
/// without a quote or a backslash, so it needs no escape in a quoted YAML
/// string.
const fn is_version(version: &str) -> bool {
    let bytes = version.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if !byte.is_ascii_graphic() || byte == b'"' || byte == b'\\' {
            return false;
        }
        index += 1;
    }
    true
}

/// Returns the byte offset of the line that closes the frontmatter of
/// `text`, or `None` when `text` does not start with a `---` line or has no
/// closing `---` line.
///
/// RUST-10: it reads line feeds only. A skill's text is the tool's own
/// file, which the tool keeps with Unix line endings.
const fn frontmatter_end(text: &[u8]) -> Option<usize> {
    if !line_is(text, 0, b"---") {
        return None;
    }
    let mut start = next_line(text, 0);
    while start < text.len() {
        if line_is(text, start, b"---") {
            return Some(start);
        }
        start = next_line(text, start);
    }
    None
}

/// Reports whether a line of the frontmatter of `text` starts with
/// `metadata:`.
const fn has_metadata(text: &[u8]) -> bool {
    let Some(end) = frontmatter_end(text) else {
        return false;
    };
    let mut start = next_line(text, 0);
    while start < end {
        if starts_with(text, start, b"metadata:") {
            return true;
        }
        start = next_line(text, start);
    }
    false
}

/// Returns the offset of the line after the one that starts at `start`, or
/// the length of `text` when that line is the last.
const fn next_line(text: &[u8], start: usize) -> usize {
    let mut index = start;
    while index < text.len() && text[index] != b'\n' {
        index += 1;
    }
    if index < text.len() { index + 1 } else { index }
}

/// Reports whether the line that starts at `start` is `line` exactly.
const fn line_is(text: &[u8], start: usize, line: &[u8]) -> bool {
    let end = start + line.len();
    starts_with(text, start, line) && (end == text.len() || text[end] == b'\n')
}

/// Reports whether `text` holds `prefix` at `start`.
const fn starts_with(text: &[u8], start: usize, prefix: &[u8]) -> bool {
    if start + prefix.len() > text.len() {
        return false;
    }
    let mut index = 0;
    while index < prefix.len() {
        if text[start + index] != prefix[index] {
            return false;
        }
        index += 1;
    }
    true
}
