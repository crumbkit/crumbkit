//! The skill that a tool embeds, and the frontmatter lines that mark the
//! copy that `install` writes as the tool's own.

use std::fmt::Write as _;

#[cfg(test)]
mod tests;

/// One of a tool's agent skills: the tool's name, its version, the text of
/// the skill's `SKILL.md` and any files that the skill reads beside it, all
/// of which the tool embeds with `include_str!`.
///
/// The skill's name is the `name` of its frontmatter, which is what an
/// agent reads, so a tool with several skills gives each `SKILL.md` its own
/// `name` and `description`. `install` writes the text to
/// `<dir>/<name>/SKILL.md` with two lines added to its frontmatter under
/// `metadata:`, which name the tool and the version in quoted YAML strings:
///
/// ```text
/// metadata:
///   tool: "shelf"
///   version: "0.3.0"
/// ```
///
/// The lines tell `install` that a later copy at that path is its own, so
/// it replaces that copy and leaves every other file alone. They name the
/// tool whatever the skill is called, so `install` recognises each skill
/// of its own tool. An agent still
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
/// const LENDING: Skill = Skill::new(
///     "shelf",
///     "0.3.0",
///     "---\nname: shelf-lending\ndescription: Lend a book.\n---\n\nRun `shelf loans add`.\n",
/// );
/// assert_eq!(SKILL.name(), "shelf");
/// assert_eq!(LENDING.name(), "shelf-lending");
/// assert_eq!(LENDING.tool(), "shelf");
/// ```
///
/// A skill that reads other files beside its `SKILL.md`, such as a
/// reference table, names them with [`Skill::files`], and `install` writes
/// them into the same folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Skill {
    /// The tool's name, which the installed copy records.
    tool: &'static str,
    /// The skill's name, which names the skill's folder.
    name: &'static str,
    /// The tool's version, which the installed copy records.
    version: &'static str,
    /// The text of the skill's `SKILL.md`.
    text: &'static str,
    /// The other files of the skill, each as its file name and its text.
    files: &'static [(&'static str, &'static str)],
}

impl Skill {
    /// Returns a skill of the tool named `tool` at `version`, whose
    /// `SKILL.md` is `text`.
    ///
    /// The skill's name is the value of the `name:` line of the frontmatter
    /// of `text`, plain or in double quotes, without the spaces around it.
    /// The folder that `install` writes therefore always has the name that
    /// an agent reads in the file.
    ///
    /// # Panics
    ///
    /// Panics unless `tool` is lowercase ASCII letters, digits and dashes
    /// that start with a letter, `version` is one non-empty line of
    /// printable ASCII without a quote or a backslash, and `text` starts
    /// with a frontmatter block between two `---` lines that has a `name:`
    /// line whose value follows the rule for `tool`, and has no
    /// top-level `metadata:` key, because `install` adds that key. A `const`
    /// that calls it with such arguments fails to compile:
    ///
    /// ```compile_fail
    /// const BAD: crumbkit_skills::Skill =
    ///     crumbkit_skills::Skill::new("shelf", "0.3.0", "No frontmatter.\n");
    /// ```
    ///
    /// ```compile_fail
    /// const NAMELESS: crumbkit_skills::Skill = crumbkit_skills::Skill::new(
    ///     "shelf",
    ///     "0.3.0",
    ///     "---\ndescription: Browse the catalogue.\n---\n",
    /// );
    /// ```
    #[must_use]
    pub const fn new(tool: &'static str, version: &'static str, text: &'static str) -> Self {
        assert!(
            is_name(tool.as_bytes()),
            "a tool name is lowercase ASCII letters, digits and dashes, starting with a letter"
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
        let name = frontmatter_name(text);
        assert!(
            is_name(name.as_bytes()),
            "a skill's frontmatter has a `name:` line whose value is lowercase ASCII letters, digits and dashes, starting with a letter"
        );
        Self {
            tool,
            name,
            version,
            text,
            files: &[],
        }
    }

    /// Returns the skill with `files` beside its `SKILL.md`, each as its
    /// file name and its text, in the order that `install` writes them.
    ///
    /// `install` writes each file into the skill's folder before
    /// `SKILL.md`, and `check` compares each one, so an agent that reads
    /// the skill finds the files of the same version beside it.
    ///
    /// ```
    /// use crumbkit_skills::Skill;
    ///
    /// const SKILL: Skill = Skill::new(
    ///     "shelf",
    ///     "0.3.0",
    ///     "---\nname: shelf\ndescription: Browse the catalogue.\n---\n\nRead `REFERENCE.md`.\n",
    /// )
    /// .files(&[("REFERENCE.md", "# Every shelf command\n")]);
    /// # let _ = SKILL;
    /// ```
    ///
    /// # Panics
    ///
    /// Panics unless each name is ASCII letters, digits, dots, dashes and
    /// underscores that does not start with a dot, no name is `SKILL.md`,
    /// and no name appears twice, with names compared ignoring ASCII case,
    /// because a file system that ignores case holds them as one file. A `const` that calls it with such
    /// arguments fails to compile.
    // RUST-10: a file name holds no `/`, so every file sits in the skill's
    // own folder. A skill with subfolders needs install to create them and
    // check to walk them, which a path type with its own checks would
    // replace this with.
    #[must_use]
    pub const fn files(self, files: &'static [(&'static str, &'static str)]) -> Self {
        let mut index = 0;
        while index < files.len() {
            let name = files[index].0;
            assert!(
                is_file_name(name),
                "a skill file name is ASCII letters, digits, dots, dashes and underscores, not starting with a dot"
            );
            assert!(
                !same_name(name.as_bytes(), b"SKILL.md"),
                "a skill file is not named SKILL.md, which holds the skill's text"
            );
            let mut earlier = 0;
            while earlier < index {
                assert!(
                    !same_name(files[earlier].0.as_bytes(), name.as_bytes()),
                    "a skill names each file once"
                );
                earlier += 1;
            }
            index += 1;
        }
        Self { files, ..self }
    }

    /// Returns the tool's name, such as `shelf`, which is the same for
    /// each of the tool's skills.
    #[must_use]
    pub const fn tool(&self) -> &'static str {
        self.tool
    }

    /// Returns the skill's name, such as `shelf-lending`, which is the
    /// `name` of its frontmatter and names its folder.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the tool's version, such as `0.3.0`.
    #[must_use]
    pub const fn version(&self) -> &'static str {
        self.version
    }

    /// Returns the text of the skill's `SKILL.md`, as the tool embeds it.
    #[must_use]
    pub const fn text(&self) -> &'static str {
        self.text
    }

    /// Returns the other files of the skill, each as its file name and its
    /// text, in the order that [`Skill::files`] gave them. The getter has
    /// the `get_` prefix because the builder takes the plain name.
    #[must_use]
    pub const fn get_files(&self) -> &'static [(&'static str, &'static str)] {
        self.files
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
            self.tool, self.version
        );
        text.push_str(rest);
        text
    }

    /// Returns who wrote `text`, an installed `SKILL.md`, by the
    /// `metadata:` lines of its frontmatter. A copy that names this skill's
    /// tool is the tool's own, whichever of its skills the copy holds.
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
            (Some(tool), Some(version)) if tool == self.tool => Writer::Tool {
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

/// Reports whether `bytes`, the name of a tool or a skill, is lowercase
/// ASCII letters, digits and dashes, starting with a letter.
const fn is_name(bytes: &[u8]) -> bool {
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

/// Reports whether `name` is ASCII letters, digits, dots, dashes and
/// underscores that does not start with a dot, so it names a file in the
/// skill's own folder and never `.` or `..`.
const fn is_file_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes[0] == b'.' {
        return false;
    }
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if !(byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_')) {
            return false;
        }
        index += 1;
    }
    true
}

/// Reports whether `a` and `b` name the same file on a file system that
/// ignores case, as macOS and Windows do by default: the same bytes apart
/// from ASCII case.
const fn same_name(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut index = 0;
    while index < a.len() {
        if !a[index].eq_ignore_ascii_case(&b[index]) {
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

/// Returns the value of the `name:` line of the frontmatter of `text`,
/// without the spaces and tabs around it and without a pair of double
/// quotes around it, or the empty string when the frontmatter has no such
/// line.
///
/// RUST-10: it reads the first line that starts with `name:` at the start
/// of a line, followed by a space, a tab or the end of the line, and is no
/// YAML parser. A name in single quotes, a name with a comment after it or
/// a name on the line below its key is returned as written or not found,
/// and `Skill::new` then refuses it. A YAML parser that a `const fn` can
/// call replaces this once a skill needs to read other frontmatter.
const fn frontmatter_name(text: &str) -> &str {
    let bytes = text.as_bytes();
    let Some(end) = frontmatter_end(bytes) else {
        return "";
    };
    let key = b"name:";
    let mut start = next_line(bytes, 0);
    while start < end {
        let mut from = start + key.len();
        let next = next_line(bytes, start);
        if starts_with(bytes, start, key) && matches!(bytes[from], b'\n' | b' ' | b'\t') {
            // A line before the closing `---` line ends with a line feed.
            let mut to = next - 1;
            while from < to && matches!(bytes[from], b' ' | b'\t') {
                from += 1;
            }
            while from < to && matches!(bytes[to - 1], b' ' | b'\t') {
                to -= 1;
            }
            if to - from >= 2 && bytes[from] == b'"' && bytes[to - 1] == b'"' {
                from += 1;
                to -= 1;
            }
            // Both offsets sit beside an ASCII byte, so they are character
            // boundaries.
            let (head, _) = text.split_at(to);
            let (_, name) = head.split_at(from);
            return name;
        }
        start = next;
    }
    ""
}

/// Reports whether a line of the frontmatter of `text` holds the top-level
/// key `metadata`: `metadata:` followed by the end of the line, a space or
/// a tab, so `metadata: {a: b}` counts and `metadatax:` does not.
const fn has_metadata(text: &[u8]) -> bool {
    let Some(end) = frontmatter_end(text) else {
        return false;
    };
    let mut start = next_line(text, 0);
    while start < end {
        let after = start + b"metadata:".len();
        if starts_with(text, start, b"metadata:")
            && (after == text.len() || matches!(text[after], b'\n' | b' ' | b'\t'))
        {
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
