//! The `skills check` command, which compares each installed copy of a
//! tool's skill with the skill that the binary embeds (ADR-0192).
//!
//! The tool owns the command. It adds a `check` subcommand that takes
//! [`SkillsCheck`] to its `skills` group, and its handler passes the result
//! of [`check`] to [`finish`](crumbkit::finish) with the document
//! [`SKILLS_CHECK`]:
//!
//! ```ignore
//! SkillsCommand::Check(args) => {
//!     let result = Ok::<_, crumbkit::Error>(crumbkit_skills::check(&SKILL, &args));
//!     crumbkit::finish(run, crumbkit_skills::SKILLS_CHECK, result)
//! }
//! ```

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::Args;
use crumbkit::{Document, Listing, Row};
use schemars::JsonSchema;
use serde::Serialize;

use crate::skill::{Skill, Writer};
use crate::targets::{self, Place, Target, shown, shown_all};

#[cfg(test)]
mod tests;

/// The document that `skills check` prints: one row for each skills
/// directory, of which the text lists only those whose copy is missing,
/// differs or cannot be read. Its schema is
/// `crates/crumbkit-skills/schemas/skills-check-v1.json`.
pub const SKILLS_CHECK: Document<Listing<CheckRow>> = Document::new(
    "crumbkit",
    "skills-check",
    1,
    "Whether each skills directory holds the skill that the binary embeds: current, linked, missing, differs or unreadable.",
)
.when_empty("no agent skills directory exists; name one with --dir")
.only_problems()
.built_in();

/// The arguments of `<tool> skills check`: which skills directory to check.
#[derive(Args, Clone, Debug, Default, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct SkillsCheck {
    /// Check this skills directory in place of the agent skills directories
    #[arg(long, value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    dir: Option<PathBuf>,
}

/// Whether a skills directory holds the binary's skill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Checked {
    /// The directory holds no copy of the skill.
    Missing,
    /// The directory holds a copy that differs from the binary's.
    Differs,
    /// The copy cannot be read.
    Unreadable,
    /// The skill's folder is a symbolic link to a folder that install did
    /// not write, which install skips.
    Linked,
    /// The copy is the binary's.
    Current,
}

/// One skills directory, or several that resolve to the same place, and
/// whether it holds the binary's skill.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CheckRow {
    /// Whether the copy is the binary's: missing, differs, unreadable, linked or current.
    state: Checked,
    /// The skills directories that share this copy, as named. Directories that resolve to one place through a symbolic link share one copy.
    dirs: Vec<String>,
    /// Why the copy is missing a file, differs, cannot be read or is linked, such as `written by shelf 0.2.0`, `REFERENCE.md is missing` or `linked to /src/shelf/skill`, or null.
    reason: Option<String>,
    /// The SKILL.md file that was checked, at its real path.
    path: String,
}

impl Row for CheckRow {
    const COLUMNS: &'static [&'static str] = &["state", "dirs", "reason"];
    const LONG_COLUMNS: &'static [&'static str] = &["path"];

    // A copy that is missing, differs or cannot be read makes check exit
    // with 1, and the document lists only those rows in text. A linked
    // folder does not, because install skips it and its owner chose it.
    fn is_problem(&self) -> bool {
        !matches!(self.state, Checked::Current | Checked::Linked)
    }
}

/// Compares the copy of `skill` in each agent skills directory that exists,
/// or in the directory that `--dir` names, with the text that `install`
/// writes, and returns one row for each, for the tool to pass to
/// [`finish`](crumbkit::finish) with [`SKILLS_CHECK`].
///
/// It checks the directories that [`install`](crate::install) writes to,
/// merged by real path in the same way, and follows a symbolic link at
/// `<dir>/<name>`. A copy is current when its text is what this binary's
/// install writes, including the `metadata:` lines that name the tool and
/// its version, so a copy from another version differs. A current
/// `SKILL.md` whose folder lacks one of the skill's other files, which
/// [`Skill::files`] names, is missing, with the reason naming the file, and
/// one whose other file holds other text differs. A directory
/// without a copy is missing, and a copy that cannot be read, or a file
/// where the skill's folder belongs, is unreadable. Those rows come first,
/// the text lists only them, and the command exits with 1 when there is
/// one. A skill folder that links to a folder this tool's install did not
/// write, which install skips, is `linked`, with the reason
/// `linked to <path>`, and is no problem. With no home directory, or when
/// none of the agent skills directories exists, it checks nothing, and the
/// empty note asks for `--dir`.
#[must_use]
pub fn check(skill: &Skill, args: &SkillsCheck) -> Listing<CheckRow> {
    check_in(skill, targets::agent_dirs_from_env(), args)
}

/// Checks `skill` as [`check`] describes, when the environment names the
/// agent skills directories `agent_dirs`.
fn check_in(
    skill: &Skill,
    agent_dirs: Option<Vec<PathBuf>>,
    args: &SkillsCheck,
) -> Listing<CheckRow> {
    let text = skill.installed();
    let refused = |dir: &Path, reason: String| CheckRow {
        state: Checked::Missing,
        dirs: vec![shown(dir)],
        reason: Some(reason),
        path: shown(&dir.join(skill.name()).join("SKILL.md")),
    };
    targets::listing(args.dir.as_deref(), agent_dirs, refused, |targets| {
        let mut rows: Vec<CheckRow> = Vec::new();
        for target in targets {
            let row = row(skill, &text, &target);
            match rows
                .iter_mut()
                .find(|earlier| earlier.path == row.path && earlier.state == row.state)
            {
                Some(earlier) => earlier.dirs.extend(row.dirs),
                None => rows.push(row),
            }
        }
        rows
    })
}

/// Compares each of the other files of `skill` in `folder` with the text
/// that the skill embeds, and returns the state and reason of the first
/// that is missing, differs or cannot be read, or `None` when every file
/// matches.
fn files(skill: &Skill, folder: &Path) -> Option<(Checked, String)> {
    skill
        .get_files()
        .iter()
        .find_map(|(name, body)| match fs::read_to_string(folder.join(name)) {
            Ok(current) if current == *body => None,
            Ok(_) => Some((
                Checked::Differs,
                format!("{name} differs from this binary's"),
            )),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Some((Checked::Missing, format!("{name} is missing")))
            }
            Err(error) => Some((Checked::Unreadable, format!("{name}: {error}"))),
        })
}

/// Returns the row of `target`, whose copy of `skill` should be `text`.
fn row(skill: &Skill, text: &str, target: &Target) -> CheckRow {
    let row = |state, reason: Option<String>, path: &Path| CheckRow {
        state,
        dirs: shown_all(&target.dirs),
        reason,
        path: shown(path),
    };
    let linked = |link: &Path, to: &Path| {
        let reason = format!("linked to {}", shown(to));
        row(Checked::Linked, Some(reason), &link.join("SKILL.md"))
    };
    let folder = match targets::place(&target.real, skill.name()) {
        Place::Absent { folder } => {
            return row(Checked::Missing, None, &folder.join("SKILL.md"));
        }
        Place::Folder { folder } => folder,
        Place::Linked { link, to } => {
            // Install updates a linked folder only when it holds this
            // tool's copy, and skips any other, so check does the same.
            match fs::read_to_string(to.join("SKILL.md")) {
                Ok(old) if matches!(skill.writer(&old), Writer::Tool { .. }) => to,
                _ => return linked(&link, &to),
            }
        }
        Place::Broken { link, to } => return linked(&link, &to),
        Place::NotFolder { path } => {
            let reason = format!("{} is not a folder", shown(&path));
            return row(Checked::Unreadable, Some(reason), &path);
        }
        Place::Unreadable { path, reason } => {
            return row(Checked::Unreadable, Some(reason), &path);
        }
    };
    let file = folder.join("SKILL.md");
    match fs::read_to_string(&file) {
        Ok(old) if old == text => match files(skill, &folder) {
            Some((state, reason)) => row(state, Some(reason), &file),
            None => row(Checked::Current, None, &file),
        },
        Ok(old) => {
            let reason = match skill.writer(&old) {
                Writer::Tool { version } => format!("written by {} {version}", skill.name()),
                Writer::Other => format!(
                    "not written by {} skills install; install --force replaces it",
                    skill.name()
                ),
            };
            row(Checked::Differs, Some(reason), &file)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => row(Checked::Missing, None, &file),
        Err(error) => row(Checked::Unreadable, Some(error.to_string()), &file),
    }
}
