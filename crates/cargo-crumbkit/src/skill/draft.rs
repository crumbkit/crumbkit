//! `cargo crumbkit skill draft`, which writes the generated sections of a
//! tool's SKILL.md from its command tree and, given exported usage, its
//! most-used commands (ADR-0194).

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crumbkit::{Document, Listing, Row};
use schemars::JsonSchema;
use serde::Serialize;

use crate::cargo;
use crate::cli::SkillDraftArgs;
use crate::error::SkillError;
use crate::skill::tree::Tree;
use crate::tool::Tool;

mod render;
mod sections;
mod usage;

#[cfg(test)]
mod tests;

use sections::{Rewrite, Unclosed};
use usage::Usage;

/// The document that `cargo crumbkit skill draft` prints: one row for each
/// generated section of the skill file and what the draft did to it. Its
/// schema is `crates/cargo-crumbkit/schemas/skill-draft-v1.json`.
pub(crate) const SKILL_DRAFT: Document<Listing<Drafted, Summary>> = Document::new(
    "crumbkit",
    "skill-draft",
    1,
    "Each generated section of a tool's skill file, and what the draft did to it.",
);

/// The skill file in the binary's package, when `--skill` names none.
const SKILL: &str = "skill/SKILL.md";

/// One generated section of a skill file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Section {
    /// The frontmatter's `description`, from the root's about text.
    Description,
    /// The job table, from the goal headings of the tool's help.
    Jobs,
    /// The most-used commands, from exported usage.
    Usage,
}

impl Section {
    /// Returns the name that the section's markers use, such as `jobs`.
    const fn name(self) -> &'static str {
        match self {
            Self::Description => "description",
            Self::Jobs => "jobs",
            Self::Usage => "usage",
        }
    }
}

/// What the draft did to one section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Status {
    /// `--check` found that a rerun without it would change the section.
    WouldChange,
    /// The draft wrote new text into the section.
    Written,
    /// The section already held the text that the draft would write.
    Unchanged,
    /// The draft had no usage to rank, so the section keeps its text.
    Kept,
    /// The file has no markers for the section, so the draft leaves it out.
    Unmarked,
}

/// One generated section of the skill file and what the draft did to it.
#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub(crate) struct Drafted {
    /// The section's name: description, jobs or usage.
    section: Section,
    /// What the draft did: written, unchanged, would-change under --check, kept without usage, or unmarked when the file has no markers for the section.
    status: Status,
}

impl Row for Drafted {
    const COLUMNS: &'static [&'static str] = &["section", "status"];

    fn is_problem(&self) -> bool {
        self.status == Status::WouldChange
    }
}

/// The figures of one draft: the file, the tool and the usage it read.
#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub(crate) struct Summary {
    /// The path of the skill file.
    file: String,
    /// The name that the tool's root command runs as.
    tool: String,
    /// Whether the draft only checked the file, under --check, and wrote nothing.
    check: bool,
    /// The usage directory that --usage named, or null without --usage.
    usage: Option<String>,
    /// The runs of the tool that the usage directory holds, each span id once.
    runs: u64,
    /// The runs that the ranking ignored, because they name another tool or a command that the tool no longer has.
    ignored: u64,
    /// The non-empty lines of the usage files that are not an exported usage record.
    unreadable: u64,
}

/// Prints one sentence, such as `Drafted skill/SKILL.md for shelf from 12
/// runs, and ignored 1 run of a command that shelf no longer has or of
/// another tool.`
impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let verb = if self.check { "Checked" } else { "Drafted" };
        write!(f, "{verb} {} for {}", self.file, self.tool)?;
        match &self.usage {
            None => write!(f, " without usage, so its usage section keeps its text.")?,
            Some(dir) if self.runs == 0 => write!(
                f,
                ", and {dir} holds no runs of {}, so its usage section keeps its text.",
                self.tool
            )?,
            Some(_) => write!(
                f,
                " from {} runs, and ignored {} that name a command {} no longer has or another tool.",
                self.runs, self.ignored, self.tool
            )?,
        }
        if self.unreadable > 0 {
            write!(
                f,
                " {} lines of the usage files are not exported usage records.",
                self.unreadable
            )?;
        }
        Ok(())
    }
}

/// The text of the file after the draft, with the row of each section and
/// the figures of the usage.
struct Draft {
    /// The file's text after the draft.
    text: String,
    /// The row of each section.
    rows: Vec<Drafted>,
    /// The runs of the tool in the usage.
    runs: u64,
    /// The runs that the ranking ignored.
    ignored: u64,
}

/// Builds the binary that `--bin` names, as `weigh` does, reads its
/// `commands list --all --json --full` document, and rewrites the generated
/// sections of the skill file that `--skill` names, or of `skill/SKILL.md`
/// in the binary's package. A missing file gets a skeleton. Under `--check`
/// it writes nothing, and each section that a rerun would change reports a
/// problem.
///
/// # Errors
///
/// Returns the [`SkillError`] of a build that fails, of a binary that
/// cannot run or lists no commands, of a skill file or usage file that
/// cannot be read, of a section marker without its end marker, or of a
/// skill file that cannot be written.
pub(crate) fn draft(args: &SkillDraftArgs) -> Result<Listing<Drafted, Summary>, SkillError> {
    let built = cargo::build(&cargo::program(), &args.bin)?;
    let file = args
        .skill
        .clone()
        .unwrap_or_else(|| built.package.join(SKILL));
    let existing = match fs::read_to_string(&file) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(SkillError::Unreadable { path: file, error }),
    };
    let usage = match &args.usage {
        Some(dir) => {
            usage::read(dir).map_err(|(path, error)| SkillError::UsageUnreadable { path, error })?
        }
        None => None,
    };
    let tool = Tool::new(&built.executable, &std::env::temp_dir())?;
    let document = tool.document()?;
    let tree = Tree::read(&document).ok_or_else(|| tool.no_commands_list())?;
    let draft = drafted(
        existing.as_deref(),
        &tree,
        usage.as_ref(),
        args.top,
        args.check,
    )
    .map_err(|(section, Unclosed { line })| SkillError::Unclosed {
        path: file.clone(),
        section: section.name(),
        line,
    })?;
    if !args.check && existing.as_deref() != Some(draft.text.as_str()) {
        write(&file, &draft.text)?;
    }
    let summary = Summary {
        file: file.display().to_string(),
        tool: tree.name().to_owned(),
        check: args.check,
        usage: args.usage.as_ref().map(|dir| dir.display().to_string()),
        runs: draft.runs,
        ignored: draft.ignored,
        unreadable: usage.as_ref().map_or(0, |usage| usage.unreadable),
    };
    Ok(Listing::from_all(draft.rows, None)
        .described("sections")
        .summary(summary))
}

/// Returns the draft of `existing`, the skill file's text or `None` when
/// the file is missing, for `tree`, with the `top` most-used commands of
/// `usage`. Under `check` a section that would change reports
/// [`Status::WouldChange`] in place of [`Status::Written`].
///
/// The usage section is drafted when the usage holds at least one ranked
/// command. Otherwise it keeps its text, so a draft without `--usage`
/// leaves the section that an earlier draft with it wrote.
///
/// # Errors
///
/// Returns the section and the line of a start marker without its end
/// marker.
fn drafted(
    existing: Option<&str>,
    tree: &Tree,
    usage: Option<&Usage>,
    top: usize,
    check: bool,
) -> Result<Draft, (Section, Unclosed)> {
    let name = tree.name();
    let all = usage.map_or(&[][..], |usage| usage.runs.as_slice());
    let ours: Vec<usage::Run> = all.iter().filter(|run| run.tool == name).cloned().collect();
    let mut ignored = count(all.len() - ours.len());
    let mut ranked = Vec::new();
    for command in usage::rank(&ours) {
        let path = format!("{name} {}", command.command);
        if tree.command(&path).is_none() {
            ignored += command.runs;
        } else if !tree.has_subcommands(&path) && ranked.len() < top {
            ranked.push(command);
        }
    }
    let description = render::description(tree);
    let jobs = render::jobs(tree);
    let most_used = (!ranked.is_empty()).then(|| render::most_used(tree, &ranked));
    let changed = if check {
        Status::WouldChange
    } else {
        Status::Written
    };
    let mut rows = Vec::new();
    let text = match existing {
        None => {
            let usage = if most_used.is_some() {
                changed
            } else {
                Status::Kept
            };
            for (section, status) in [
                (Section::Description, changed),
                (Section::Jobs, changed),
                (Section::Usage, usage),
            ] {
                rows.push(Drafted { section, status });
            }
            render::skeleton(
                tree,
                &description,
                &jobs,
                most_used.as_deref().unwrap_or_default(),
            )
        }
        Some(existing) => {
            let mut text = existing.to_owned();
            for (section, body) in [
                (Section::Description, Some(description)),
                (Section::Jobs, Some(jobs)),
                (Section::Usage, most_used),
            ] {
                let status = match body {
                    None => Status::Kept,
                    Some(body) => match sections::rewrite(&text, section.name(), &body)
                        .map_err(|unclosed| (section, unclosed))?
                    {
                        Rewrite::Unmarked => Status::Unmarked,
                        Rewrite::Unchanged => Status::Unchanged,
                        Rewrite::Changed(rewritten) => {
                            text = rewritten;
                            changed
                        }
                    },
                };
                rows.push(Drafted { section, status });
            }
            text
        }
    };
    // The sections that would change come first (OUTPUT-3), each group in
    // file order.
    rows.sort_by_key(|row| row.status != Status::WouldChange);
    Ok(Draft {
        text,
        rows,
        runs: count(ours.len()),
        ignored,
    })
}

/// Returns `n` as a `u64`. A `usize` always fits in a `u64` on the 32-bit
/// and 64-bit platforms that Rust supports, so the conversion never
/// saturates.
fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Writes `text` to `file`, after creating the directory that holds it.
///
/// # Errors
///
/// Returns [`SkillError::WriteFailed`] when the directory or the file
/// cannot be written.
fn write(file: &Path, text: &str) -> Result<(), SkillError> {
    let failed = |error| SkillError::WriteFailed {
        path: file.to_path_buf(),
        error,
    };
    if let Some(dir) = file.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        #[expect(
            clippy::disallowed_methods,
            reason = "skill draft owns the skill file, and creates its directory"
        )]
        fs::create_dir_all(dir).map_err(failed)?;
    }
    #[expect(
        clippy::disallowed_methods,
        reason = "skill draft owns the skill file, and writes it"
    )]
    fs::write(file, text).map_err(failed)
}
