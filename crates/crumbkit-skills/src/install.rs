//! The `skills install` command, which writes each of a tool's skills into
//! each agent skills directory (ADR-0192, ADR-0201).
//!
//! The tool owns the command. It adds a `skills` group with an `install`
//! subcommand that takes [`SkillsInstall`], and its handler passes the
//! result of [`install`] to [`finish`](crumbkit::finish) with the document
//! [`SKILLS_INSTALL`]:
//!
//! ```ignore
//! SkillsCommand::Install(args) => {
//!     let result = Ok::<_, crumbkit::Error>(crumbkit_skills::install(&[SKILL], &args));
//!     crumbkit::finish(run, crumbkit_skills::SKILLS_INSTALL, result)
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

/// The document that `skills install` prints: one row for each skill in
/// each skills directory that install acted on. Its schema is
/// `crates/crumbkit-skills/schemas/skills-install-v1.json`.
pub const SKILLS_INSTALL: Document<Listing<InstallRow>> = Document::new(
    "crumbkit",
    "skills-install",
    1,
    "What install did with each skill in each skills directory: written, unchanged, skipped or failed.",
)
.when_empty("no agent skills directory exists; name one with --dir")
.built_in();

/// The arguments of `<tool> skills install`: where to install, and whether
/// to replace what install did not write.
#[derive(Args, Clone, Debug, Default, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct SkillsInstall {
    /// Install into this skills directory in place of the agent skills directories
    #[arg(long, value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    dir: Option<PathBuf>,

    /// Replace a file or a link that install did not write with the skill's own folder
    #[arg(long)]
    force: bool,
}

/// What install did with one skill in one skills directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Installed {
    /// Install could not read or write the directory.
    Failed,
    /// Install left a file or a link that it did not write.
    Skipped,
    /// Install wrote the skill.
    Written,
    /// The directory already held this version of the skill.
    Unchanged,
}

/// One skill in one skills directory, or in several that resolve to the
/// same place, and what install did with it there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct InstallRow {
    /// What install did: failed, skipped, written or unchanged.
    state: Installed,
    /// The skill's name, which names its folder in each skills directory, such as `shelf`.
    skill: String,
    /// The skills directories that share this copy, as named. Directories that resolve to one place through a symbolic link share one copy.
    dirs: Vec<String>,
    /// Why install skipped or failed, or what it replaced under --force, such as `linked to /src/shelf/skill`, or null.
    reason: Option<String>,
    /// The SKILL.md file that install wrote or left, or the skill's other file whose write failed, at its real path.
    path: String,
}

impl Row for InstallRow {
    const COLUMNS: &'static [&'static str] = &["state", "skill", "dirs", "reason"];
    const LONG_COLUMNS: &'static [&'static str] = &["path"];

    // A failed row makes install exit with 1. A skipped one does
    // not, because the person or program that owns that file chose it.
    fn is_problem(&self) -> bool {
        self.state == Installed::Failed
    }
}

/// Writes each skill of `skills` to `<dir>/<name>/SKILL.md`, with the
/// skill's other files beside it, in each agent skills directory that
/// exists, or in the directory that `--dir` names, and returns one row for
/// each skill in each directory, for the tool to pass to
/// [`finish`](crumbkit::finish) with [`SKILLS_INSTALL`].
///
/// `<name>` is the skill's name, which [`Skill::name`] returns, and each
/// row names it under `skill`. The names in `skills` differ, because two
/// skills with one name would write one folder twice. Install plans and
/// writes each skill's folder on its own, so a skill that is skipped or
/// fails leaves the tool's other skills as they are.
///
/// The agent skills directories are `~/.claude/skills`, `~/.agents/skills`
/// and `$CODEX_HOME/skills`, where `CODEX_HOME` defaults to `~/.codex`. The
/// home directory is `HOME`, or `USERPROFILE` on Windows when `HOME` is
/// unset. With no home directory, or when none of the agent skills
/// directories exists, it installs nothing, and the empty note asks for
/// `--dir`. A `--dir` that does not name a directory gives a `failed` row
/// for each skill.
///
/// Directories that resolve to the same real path, such as a
/// `~/.claude/skills` that links to `~/.agents/skills`, get one write and
/// one row for each skill, which names each directory. When `<dir>/<name>` is a symbolic link, install
/// follows it: a folder that holds a copy that this tool's install wrote is
/// updated once, and a link anywhere else is skipped with the reason
/// `linked to <path>`. A `SKILL.md` that install did not write is skipped
/// too. `--force` replaces such a link, file or `SKILL.md` with the skill's
/// own folder and file. Install recognises its own copy by the
/// `metadata:` lines that [`Skill`] describes.
///
/// The skill's other files, which [`Skill::files`] names, belong to
/// install when the folder's `SKILL.md` does. A folder without a
/// `SKILL.md` that holds a file at one of their names with other text is
/// skipped, and `--force` replaces that file. A file that already holds the
/// skill's text counts as install's own, so the next install finishes a
/// copy that an interrupted one left. Install writes the other files first and
/// `SKILL.md` last, so an agent never loads a skill whose files are
/// missing, and a row is `unchanged` only when every file already matches.
///
/// Each write goes to a temporary file in the same folder, which install
/// then renames over the file, so an agent never reads half a file. A
/// directory that install cannot read or write gives a `failed` row with
/// the reason, and the command exits with 1. Rows that failed come first.
#[must_use]
pub fn install(skills: &[Skill], args: &SkillsInstall) -> Listing<InstallRow> {
    install_in(skills, targets::agent_dirs_from_env(), args)
}

/// Installs `skills` as [`install`] describes, when the environment names
/// the agent skills directories `agent_dirs`.
fn install_in(
    skills: &[Skill],
    agent_dirs: Option<Vec<PathBuf>>,
    args: &SkillsInstall,
) -> Listing<InstallRow> {
    targets::listing(
        skills,
        args.dir.as_deref(),
        agent_dirs,
        |skill, dir, reason| failed(skill, &[dir.to_owned()], dir, reason),
        |skill, targets| {
            let text = skill.installed();
            merged(skill, targets, args.force)
                .into_iter()
                .map(|(dirs, real, plan)| run(skill, &text, &dirs, &real, plan, args.force))
                .collect()
        },
    )
}

/// What install does with one skill in one skills directory.
#[derive(Debug, PartialEq, Eq)]
enum Plan {
    /// Write `SKILL.md` in `folder`, creating the folder first when it
    /// does not exist.
    Write { folder: PathBuf },
    /// Remove what is at `path`, a link or a file, create a folder there and
    /// write `SKILL.md` in it. `what` names what it replaced.
    Replace { path: PathBuf, what: String },
    /// Leave the directory alone, for `reason`.
    Skip { path: PathBuf, reason: String },
    /// Report that the directory cannot be read, for `reason`.
    Fail { path: PathBuf, reason: String },
}

/// Returns the plan of each target with the target's skills directories and
/// real path, with the targets whose plans write the same folder merged
/// into one, in the order of each plan's first target.
fn merged(skill: &Skill, targets: &[Target], force: bool) -> Vec<(Vec<PathBuf>, PathBuf, Plan)> {
    let mut plans: Vec<(Vec<PathBuf>, PathBuf, Plan)> = Vec::new();
    for target in targets {
        let plan = plan(skill, &target.real, force);
        let same = plans
            .iter_mut()
            .find(|(_, _, earlier)| match (earlier, &plan) {
                (Plan::Write { folder: a, .. }, Plan::Write { folder: b, .. }) => a == b,
                _ => false,
            });
        match same {
            Some((dirs, _, _)) => dirs.extend(target.dirs.iter().cloned()),
            None => plans.push((target.dirs.clone(), target.real.clone(), plan)),
        }
    }
    plans
}

/// Returns what install does in the skills directory at `real`.
fn plan(skill: &Skill, real: &Path, force: bool) -> Plan {
    let foreign = |path: PathBuf, reason: String| {
        if force {
            Plan::Replace { path, what: reason }
        } else {
            Plan::Skip { path, reason }
        }
    };
    match targets::place(real, skill.name()) {
        Place::Absent { folder } | Place::Folder { folder } => Plan::Write { folder },
        Place::Linked { link, to } => {
            let file = to.join("SKILL.md");
            match fs::read_to_string(&file) {
                Ok(text) if matches!(skill.writer(&text), Writer::Tool { .. }) => {
                    Plan::Write { folder: to }
                }
                _ => foreign(link, format!("linked to {}", shown(&to))),
            }
        }
        Place::Broken { link, to } => foreign(link, format!("linked to {}", shown(&to))),
        Place::NotFolder { path } => {
            let reason = format!("{} is not a folder", shown(&path));
            foreign(path, reason)
        }
        Place::Unreadable { path, reason } => Plan::Fail { path, reason },
    }
}

/// How many times install plans a skills directory again when its skill's
/// folder changes between the plan and the write. Each new plan reads the
/// folder afresh, so a second change in that short time fails the row.
const REPLANS: usize = 2;

/// Carries out `plan` for the skills directories `dirs`, whose real path is
/// `real`, writing `text`, and returns its row. `force` replaces a
/// `SKILL.md` that install did not write.
///
/// When the folder of a `Write` plan already exists, install plans the
/// directory again and carries out the new plan when it differs, so a link
/// that appeared after the first plan is followed or skipped as a plan
/// decides, and never written through unseen.
fn run(
    skill: &Skill,
    text: &str,
    dirs: &[PathBuf],
    real: &Path,
    mut plan: Plan,
    force: bool,
) -> InstallRow {
    for _ in 0..=REPLANS {
        let Plan::Write { folder } = &plan else {
            return carry_out(skill, text, dirs, plan, force);
        };
        match fs::create_dir(folder) {
            Ok(()) => return write_file(skill, text, dirs, &folder.join("SKILL.md"), force),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return failed(skill, dirs, &folder.join("SKILL.md"), &error.to_string());
            }
        }
        let again = self::plan(skill, real, force);
        if again == plan {
            return write_file(skill, text, dirs, &folder.join("SKILL.md"), force);
        }
        plan = again;
    }
    failed(
        skill,
        dirs,
        &real.join(skill.name()),
        "the skill's folder kept changing while install wrote it",
    )
}

/// Carries out `plan` for the skills directories `dirs`, writing `text`,
/// and returns its row. [`run`] handles a `Write` plan's folder first, so
/// a `Write` here writes into a folder that exists.
fn carry_out(skill: &Skill, text: &str, dirs: &[PathBuf], plan: Plan, force: bool) -> InstallRow {
    match plan {
        Plan::Write { folder } => write_file(skill, text, dirs, &folder.join("SKILL.md"), force),
        Plan::Replace { path, what } => {
            if let Err(error) = replace_with_folder(&path) {
                return failed(skill, dirs, &path, &error.to_string());
            }
            let mut row = write_file(skill, text, dirs, &path.join("SKILL.md"), true);
            if row.state == Installed::Written {
                row.reason = Some(format!("replaced: {what}"));
            }
            row
        }
        Plan::Skip { path, reason } => InstallRow {
            state: Installed::Skipped,
            skill: skill.name().to_owned(),
            dirs: shown_all(dirs),
            reason: Some(reason),
            path: shown(&path.join("SKILL.md")),
        },
        Plan::Fail { path, reason } => failed(skill, dirs, &path, &reason),
    }
}

/// Writes `text` to `file`, a `SKILL.md`, and each of the skill's other
/// files beside it, unless the folder already holds them, and returns the
/// row. A `SKILL.md` that install did not write is skipped unless `force`.
///
/// The other files belong to install when the folder's `SKILL.md` does, so
/// a folder without a `SKILL.md` that holds a file at one of their names
/// with other text is skipped unless `force`, before anything is written
/// there. A file that holds the embedded text is install's own, so an
/// install that stopped before `SKILL.md` is finished by the next one. Install
/// writes `SKILL.md` last, so an agent never loads a skill whose files are
/// missing.
// RUST-10: install leaves a file that an earlier version of the skill
// wrote and this version no longer names. Removing it needs a record of
// the files that each copy holds, such as a list in the frontmatter.
fn write_file(skill: &Skill, text: &str, dirs: &[PathBuf], file: &Path, force: bool) -> InstallRow {
    let row = |state, reason: Option<String>, path: &Path| InstallRow {
        state,
        skill: skill.name().to_owned(),
        dirs: shown_all(dirs),
        reason,
        path: shown(path),
    };
    let old = match fs::read_to_string(file) {
        Ok(old) => Some(old),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return row(Installed::Failed, Some(error.to_string()), file),
    };
    match &old {
        Some(old) if !force && skill.writer(old) == Writer::Other => {
            let reason = format!("not written by {} skills install", skill.tool());
            return row(Installed::Skipped, Some(reason), file);
        }
        None if !force => {
            let folder = file.parent().unwrap_or(file);
            // A file that already holds the embedded text is install's
            // own, left by an install that stopped before SKILL.md.
            let taken = skill.get_files().iter().find(|(name, body)| {
                let path = folder.join(name);
                fs::symlink_metadata(&path).is_ok()
                    && !fs::read_to_string(&path).is_ok_and(|current| current == *body)
            });
            if let Some((name, _)) = taken {
                let reason = format!("{name} not written by {} skills install", skill.tool());
                return row(Installed::Skipped, Some(reason), file);
            }
        }
        _ => {}
    }
    let mut written = false;
    let folder = file.parent().unwrap_or(file);
    for (name, body) in skill.get_files() {
        let path = folder.join(name);
        if fs::read_to_string(&path).is_ok_and(|current| current == *body) {
            continue;
        }
        if let Err(error) = crumbkit::files::write_atomically(&path, body) {
            return row(Installed::Failed, Some(error.to_string()), &path);
        }
        written = true;
    }
    if old.as_deref() != Some(text) {
        if let Err(error) = crumbkit::files::write_atomically(file, text) {
            return row(Installed::Failed, Some(error.to_string()), file);
        }
        written = true;
    }
    let state = if written {
        Installed::Written
    } else {
        Installed::Unchanged
    };
    row(state, None, file)
}

/// Returns the row of `skill` in a directory that install could not read
/// or write.
fn failed(skill: &Skill, dirs: &[PathBuf], path: &Path, reason: &str) -> InstallRow {
    InstallRow {
        state: Installed::Failed,
        skill: skill.name().to_owned(),
        dirs: shown_all(dirs),
        reason: Some(reason.to_owned()),
        path: shown(path),
    }
}

/// Removes the link or file at `path` and creates a folder there.
fn replace_with_folder(path: &Path) -> io::Result<()> {
    #[expect(
        clippy::disallowed_methods,
        reason = "this module owns the file writes of skills install, and --force removes a link or a file that stands where the skill's folder belongs"
    )]
    let removed = fs::remove_file(path);
    // Windows removes a symbolic link to a directory as a directory.
    if removed.is_err() {
        fs::remove_dir(path)?;
    }
    fs::create_dir(path)
}
