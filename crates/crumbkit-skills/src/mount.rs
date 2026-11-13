//! The mount, which gives a tool its `skills` commands in one enum field,
//! one match arm and three leaves (ADR-0216).

use std::io::Write;
use std::process::ExitCode;

use clap::{Args, Subcommand};
use crumbkit::Sink;
use crumbkit::help::{Effects, Example, Leaf};

use crate::check::{SKILLS_CHECK, SkillsCheck, check};
use crate::install::{SKILLS_INSTALL, SkillsInstall, install};
use crate::print::{SKILLS_PRINT, print};
use crate::skill::Skill;

#[cfg(test)]
mod tests;

/// The examples of `skills install`.
const INSTALL_EXAMPLES: &[Example] = &[
    Example::new(
        &[],
        "Install the skills into each agent skills directory that exists",
    ),
    Example::new(
        &["--force"],
        "Install them, replacing a file or a link that install did not write",
    ),
];

/// The example of `skills check`.
const CHECK_EXAMPLES: &[Example] = &[Example::new(
    &[],
    "Report each agent skills directory whose copy is missing or differs",
)];

/// The example of `skills print`.
const PRINT_EXAMPLES: &[Example] = &[Example::new(&[], "Print the skill")];

/// The arguments of `<tool> skills print`, which has none today. The struct
/// is `#[non_exhaustive]`, so an argument can be added without a breaking
/// change.
#[derive(Args, Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct SkillsPrint {}

/// The commands of a tool's `skills` group. A tool mounts it as the
/// `#[command(subcommand)]` field of its `skills` variant and passes it to
/// [`run`] or [`run_to`].
#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SkillsCommand {
    /// Install this tool's skills into each agent skills directory that exists
    Install(SkillsInstall),
    /// Check that each installed copy of this tool's skills matches this build
    Check(SkillsCheck),
    /// Print this tool's skill
    Print(SkillsPrint),
}

/// Runs `command` for `skills`, prints its result and returns the process's
/// exit code. `install` and `check` act on every skill of `skills`, and
/// `print` prints the first, so a tool lists its main skill first. A tool
/// whose other skills print by name keeps its own `print` command
/// (ADR-0201).
///
/// # Panics
///
/// Panics when `skills` is empty and `command` is `print`. The tool's own
/// `check_examples` test runs `skills print` and fails first.
#[must_use]
pub fn run(run: crumbkit::Run, skills: &[Skill], command: &SkillsCommand) -> ExitCode {
    match command {
        SkillsCommand::Install(args) => {
            let result = Ok::<_, crumbkit::Error>(install(skills, args));
            crumbkit::finish(run, SKILLS_INSTALL, result)
        }
        SkillsCommand::Check(args) => {
            let result = Ok::<_, crumbkit::Error>(check(skills, args));
            crumbkit::finish(run, SKILLS_CHECK, result)
        }
        SkillsCommand::Print(_) => {
            let result = Ok::<_, crumbkit::Error>(print(&skills[0]));
            crumbkit::finish(run, SKILLS_PRINT, result)
        }
    }
}

/// Does what [`run`] does through `sink`, in place of the standard streams,
/// and returns the exit code as a number, as [`finish_to`](crumbkit::finish_to)
/// does. A tool uses it to test its `skills` commands against a [`Sink`] in
/// memory.
///
/// # Panics
///
/// Panics when `skills` is empty and `command` is `print`.
#[must_use]
pub fn run_to<O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    run: crumbkit::Run,
    skills: &[Skill],
    command: &SkillsCommand,
) -> u8 {
    match command {
        SkillsCommand::Install(args) => {
            let result = Ok::<_, crumbkit::Error>(install(skills, args));
            crumbkit::finish_to(sink, run, SKILLS_INSTALL, result)
        }
        SkillsCommand::Check(args) => {
            let result = Ok::<_, crumbkit::Error>(check(skills, args));
            crumbkit::finish_to(sink, run, SKILLS_CHECK, result)
        }
        SkillsCommand::Print(_) => {
            let result = Ok::<_, crumbkit::Error>(print(&skills[0]));
            crumbkit::finish_to(sink, run, SKILLS_PRINT, result)
        }
    }
}

/// Returns the leaf of `skills install` at `path`, such as
/// `&["skills", "install"]`, for the tool's [`Help`](crumbkit::help::Help).
/// Chain `.examples(..)` to give it examples of the tool's own.
#[must_use]
pub const fn install_leaf(path: &'static [&'static str]) -> Leaf {
    Leaf::new(path)
        .examples(INSTALL_EXAMPLES)
        .effects(Effects::additive().idempotent())
        .document(SKILLS_INSTALL)
}

/// Returns the leaf of `skills check` at `path`, such as
/// `&["skills", "check"]`.
#[must_use]
pub const fn check_leaf(path: &'static [&'static str]) -> Leaf {
    Leaf::new(path)
        .examples(CHECK_EXAMPLES)
        .effects(Effects::read_only())
        .document(SKILLS_CHECK)
}

/// Returns the leaf of `skills print` at `path`, such as
/// `&["skills", "print"]`.
#[must_use]
pub const fn print_leaf(path: &'static [&'static str]) -> Leaf {
    Leaf::new(path)
        .examples(PRINT_EXAMPLES)
        .effects(Effects::read_only())
        .document(SKILLS_PRINT)
}
