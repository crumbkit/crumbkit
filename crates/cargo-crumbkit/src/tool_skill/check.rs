//! `cargo crumbkit tool-skill check`, which checks every command line in a
//! tool's SKILL.md against the tool's real command tree (ADR-0193).

use std::fmt;
use std::fs;
use std::path::Path;

use crumbkit::{Document, Listing, Row};
use schemars::JsonSchema;
use serde::Serialize;

use crate::cargo;
use crate::cli::ToolSkillCheckArgs;
use crate::error::SkillError;
use crate::tool::Tool;
use crate::tool_skill::tree::Tree;

// `tool-skill draft` reads fences with the same rules, so it skips markers in
// a fenced example.
pub(super) mod markdown;
mod verify;
mod words;

#[cfg(test)]
mod tests;

/// The document that `cargo crumbkit tool-skill check` prints: one row for each
/// command line of the tool in the skill file, the failing ones first. Its
/// schema is `crates/cargo-crumbkit/schemas/tool-skill-check-v1.json`.
pub(crate) const TOOL_SKILL_CHECK: Document<Listing<Checked, Summary>> = Document::new(
    "crumbkit",
    "tool-skill-check",
    1,
    "Each command line of a tool in its skill file, and why it fails.",
)
.only_problems();

/// The skill file in the binary's package, when `--skill` names none.
const SKILL: &str = "skill/SKILL.md";

/// The prompt that a shell example may put before a command line, which
/// the check drops.
const PROMPT: &str = "$ ";

/// One command line of the tool in the skill file, and the reason that it
/// fails.
#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub(crate) struct Checked {
    /// The path of the skill file.
    file: String,
    /// The line of the skill file that holds the command line, counted from 1.
    line: u64,
    /// The command line as the skill file writes it.
    span: String,
    /// Why the command line does not match the tool's commands, or null when it matches.
    reason: Option<String>,
}

impl Row for Checked {
    const COLUMNS: &'static [&'static str] = &["file", "line", "span", "reason"];
    const PROBLEM: Option<&'static str> = Some("reason");
    const LINE: Option<&'static str> = Some("<FILE>:<LINE>: <SPAN>: <REASON>");

    fn text_line(&self) -> Option<String> {
        let reason = self.reason.as_deref().unwrap_or("ok");
        Some(format!(
            "{}:{}: {}: {reason}",
            self.file, self.line, self.span
        ))
    }
}

/// The figures of one check: the file, the tool, and how many command
/// lines it checked and how many failed.
#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub(crate) struct Summary {
    /// The path of the skill file.
    file: String,
    /// The name that the tool's root command runs as, which starts each checked command line.
    tool: String,
    /// The command lines of the tool in the skill file.
    checked: u64,
    /// The command lines that do not match the tool's commands.
    failed: u64,
}

/// Prints one sentence, such as `Checked 12 shelf command lines in
/// skill/SKILL.md, and 1 failed.`
impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Checked {} {} command lines in {}, and {} failed.",
            self.checked, self.tool, self.file, self.failed
        )
    }
}

/// Builds the binary that `--bin` names, as `weigh` does, reads its
/// `commands list --all --json --full` document, and checks every command
/// line of the tool in the skill file that `--skill` names, or in
/// `skill/SKILL.md` in the binary's package.
///
/// # Errors
///
/// Returns the [`SkillError`] of a build that fails, of a binary that
/// cannot run or lists no commands, or of a skill file that cannot be read.
pub(crate) fn check(args: &ToolSkillCheckArgs) -> Result<Listing<Checked, Summary>, SkillError> {
    let built = cargo::build(&cargo::program(), &args.bin)?;
    let file = args
        .skill
        .clone()
        .unwrap_or_else(|| built.package.join(SKILL));
    let text = fs::read_to_string(&file).map_err(|error| SkillError::Unreadable {
        path: file.clone(),
        error,
    })?;
    let tool = Tool::new(&built.executable, &std::env::temp_dir())?;
    let document = tool.document()?;
    let tree = Tree::read(&document).ok_or_else(|| tool.no_commands_list())?;
    Ok(listing(&file, &text, &tree, &args.bin))
}

/// Returns the rows and summary of the command lines of `tree`'s tool, whose
/// binary is `bin`, in `text`, the skill file at `file`.
///
/// A command line is an inline code span, or a line of a fenced block, whose
/// first word is the name of the tool's root command, after a `$ ` prompt if
/// it has one. When the root's name is not the binary's, the tool runs under
/// another program, as cargo-crumbkit runs as `cargo crumbkit`, and that
/// program has commands of its own, such as `cargo test`. A command line
/// then also needs a second word that names a subcommand of the root.
fn listing(file: &Path, text: &str, tree: &Tree, bin: &str) -> Listing<Checked, Summary> {
    let name = tree.name();
    let ours = |second: Option<&str>| {
        name == bin
            || second.is_some_and(|second| tree.command(&format!("{name} {second}")).is_some())
    };
    let file = file.display().to_string();
    let mut rows: Vec<Checked> = markdown::code(text)
        .into_iter()
        .filter_map(|code| {
            let span = code.text.strip_prefix(PROMPT).unwrap_or(&code.text);
            let reason = match words::split(span) {
                Ok(words)
                    if words.first().is_some_and(|first| first == name)
                        && ours(words.get(1).map(String::as_str)) =>
                {
                    verify::verify(tree, &words).err()
                }
                Err(reason) => {
                    let mut plain = span.split_whitespace();
                    if plain.next() != Some(name) || !ours(plain.next()) {
                        return None;
                    }
                    Some(reason.to_owned())
                }
                Ok(_) => return None,
            };
            Some(Checked {
                file: file.clone(),
                // A `usize` always fits in a `u64` on the 32-bit and 64-bit
                // platforms that Rust supports, so the conversion never
                // saturates.
                line: u64::try_from(code.line).unwrap_or(u64::MAX),
                span: span.to_owned(),
                reason,
            })
        })
        .collect();
    // The failing rows come first (OUTPUT-3), each group in file order.
    rows.sort_by_key(|row| row.reason.is_none());
    let count = |n: usize| u64::try_from(n).unwrap_or(u64::MAX);
    let summary = Summary {
        file,
        tool: name.to_owned(),
        checked: count(rows.len()),
        failed: count(rows.iter().filter(|row| row.reason.is_some()).count()),
    };
    Listing::from_all(rows, None)
        .described("command lines")
        .summary(summary)
}
