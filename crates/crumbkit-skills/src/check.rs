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
use crate::targets::{self, Chosen, Homes, Place, Target, shown, shown_all};

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
    "Whether each skills directory holds the skill that the binary embeds: current, missing, differs or unreadable.",
)
.when_empty("no agent skills directory exists")
.only_problems();

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
    /// The copy is the binary's.
    Current,
}

/// One skills directory, or several that resolve to the same place, and
/// whether it holds the binary's skill.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CheckRow {
    /// Whether the copy is the binary's: missing, differs, unreadable or current.
    state: Checked,
    /// The skills directories that share this copy, as named. Directories that resolve to one place through a symbolic link share one copy.
    dirs: Vec<String>,
    /// Why the copy differs or cannot be read, such as `written by shelf 0.2.0`, or null.
    reason: Option<String>,
    /// The SKILL.md file that was checked, at its real path.
    path: String,
}

impl Row for CheckRow {
    const COLUMNS: &'static [&'static str] = &["state", "dirs", "reason"];
    const LONG_COLUMNS: &'static [&'static str] = &["path"];

    // A copy that is missing, differs or cannot be read makes check exit
    // with 1, and the document lists only those rows in text.
    fn is_problem(&self) -> bool {
        self.state != Checked::Current
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
/// its version, so a copy from another version differs. A directory
/// without a copy is missing, and a copy that cannot be read, or a file
/// where the skill's folder belongs, is unreadable. Those rows come first,
/// the text lists only them, and the command exits with 1 when there is
/// one. With no home directory and no `--dir`, it checks nothing and its
/// note says so.
#[must_use]
pub fn check(skill: &Skill, args: &SkillsCheck) -> Listing<CheckRow> {
    check_in(skill, &Homes::from_env(), args)
}

/// Checks `skill` as [`check`] describes, when the environment gives
/// `homes`.
fn check_in(skill: &Skill, homes: &Homes, args: &SkillsCheck) -> Listing<CheckRow> {
    let name = skill.name();
    let (targets, note) = match targets::choose(args.dir.as_deref(), homes) {
        Chosen::Targets(targets) => (targets, None),
        Chosen::NamedMissing(dir) => {
            let row = CheckRow {
                state: Checked::Missing,
                dirs: vec![shown(&dir)],
                reason: Some("the directory does not exist".to_owned()),
                path: shown(&dir.join(name).join("SKILL.md")),
            };
            return Listing::from_all(vec![row], None).described("skills directories");
        }
        Chosen::NoAgents(tried) => {
            let tried: Vec<String> = shown_all(&tried);
            let note = format!(
                "None of {} exists, so {name} checked nothing.",
                tried.join(", ")
            );
            (Vec::new(), Some(note))
        }
        Chosen::NoHome => {
            let note =
                format!("The environment names no home directory, so {name} checked nothing.");
            (Vec::new(), Some(note))
        }
    };
    let text = skill.installed();
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
    rows.sort_by_key(|row| !row.is_problem());
    let listing = Listing::from_all(rows, None).described("skills directories");
    match note {
        Some(note) => listing.note(note),
        None => listing,
    }
}

/// Returns the row of `target`, whose copy of `skill` should be `text`.
fn row(skill: &Skill, text: &str, target: &Target) -> CheckRow {
    let row = |state, reason: Option<String>, path: &Path| CheckRow {
        state,
        dirs: shown_all(&target.dirs),
        reason,
        path: shown(path),
    };
    let (folder, linked) = match targets::place(&target.real, skill.name()) {
        Place::Absent { folder } => {
            return row(Checked::Missing, None, &folder.join("SKILL.md"));
        }
        Place::Folder { folder } => (folder, None),
        Place::Linked { to, .. } => (to.clone(), Some(to)),
        Place::Broken { link, to } => {
            let reason = format!("linked to {}, which is not a folder", shown(&to));
            return row(Checked::Missing, Some(reason), &link.join("SKILL.md"));
        }
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
        Ok(old) if old == text => row(Checked::Current, None, &file),
        Ok(old) => {
            let reason = match (skill.writer(&old), linked) {
                (Writer::Tool { version }, _) => format!("written by {} {version}", skill.name()),
                (Writer::Other, Some(to)) => format!(
                    "linked to {}, which {} skills install did not write",
                    shown(&to),
                    skill.name()
                ),
                (Writer::Other, None) => format!(
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
