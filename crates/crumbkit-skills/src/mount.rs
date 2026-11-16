//! The mount, which gives a tool its `skills` commands in one enum field,
//! one match arm and three leaves, with the tool's name in their help
//! (ADR-0216, ADR-0233).

use std::io::Write;
use std::process::ExitCode;

use clap::Args;
use crumbkit::Sink;
use crumbkit::help::{Effects, Example, Leaf};

use crate::check::{SKILLS_CHECK, SkillsCheck, check};
use crate::install::{SKILLS_INSTALL, SkillsInstall, install};
use crate::print::{SKILLS_PRINT, print};
use crate::skill::Skill;

#[cfg(test)]
mod tests;

/// The arguments of `<tool> skills print`, which has none today. The struct
/// is `#[non_exhaustive]`, so an argument can be added without a breaking
/// change.
#[derive(Args, Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct SkillsPrint {}

/// The command of a mounted `skills` group, without the tool's name. The
/// type that [`skills_command!`](crate::skills_command) defines converts into
/// it, and [`run`] and [`run_to`] take it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SkillsAction {
    /// `skills install`, with its arguments.
    Install(SkillsInstall),
    /// `skills check`, with its arguments.
    Check(SkillsCheck),
    /// `skills print`, with its arguments.
    Print(SkillsPrint),
}

/// Defines the clap `Subcommand` enum of a tool's `skills` group, with the
/// tool's name in each about line and each example (ADR-0233). The first
/// argument is the enum's name, with any attributes and visibility before
/// it, and the second is the tool's name as its usage line writes it, such as
/// `"shelf"` or `"cargo crumbkit"`, as a string literal.
///
/// The enum has the variants `Install(SkillsInstall)`, `Check(SkillsCheck)`
/// and `Print(SkillsPrint)`, converts into [`SkillsAction`], and has three
/// `const fn`s, `install_leaf`, `check_leaf` and `print_leaf`, each of which
/// takes the leaf's path, such as `&["skills", "install"]`, and returns the
/// leaf for the tool's [`Help`](crumbkit::help::Help). Chain `.examples(..)`
/// on a leaf to give it examples of the tool's own. The tool needs `clap`
/// with its `derive` feature, because the enum derives `clap::Subcommand`.
///
/// ```
/// crumbkit_skills::skills_command!(
///     /// The commands for shelf's skills.
///     pub ShelfSkills,
///     "shelf"
/// );
///
/// let leaf = ShelfSkills::install_leaf(&["skills", "install"]);
/// assert_eq!(
///     leaf.get_examples()[0].about(),
///     "Install shelf's skills into each agent skills directory that exists",
/// );
/// ```
#[macro_export]
macro_rules! skills_command {
    ($(#[$meta:meta])* $vis:vis $name:ident, $tool:literal $(,)?) => {
        $(#[$meta])*
        #[derive(::clap::Subcommand, Clone, Debug, PartialEq, Eq)]
        $vis enum $name {
            #[doc = concat!("Install ", $tool, "'s skills into each agent skills directory that exists")]
            #[command(about = concat!("Install ", $tool, "'s skills into each agent skills directory that exists"))]
            Install($crate::SkillsInstall),
            #[doc = concat!("Check that each installed copy of ", $tool, "'s skills matches this ", $tool)]
            #[command(about = concat!("Check that each installed copy of ", $tool, "'s skills matches this ", $tool))]
            Check($crate::SkillsCheck),
            #[doc = concat!("Print ", $tool, "'s skill")]
            #[command(about = concat!("Print ", $tool, "'s skill"))]
            Print($crate::SkillsPrint),
        }

        impl ::core::convert::From<$name> for $crate::SkillsAction {
            fn from(command: $name) -> Self {
                match command {
                    $name::Install(args) => Self::Install(args),
                    $name::Check(args) => Self::Check(args),
                    $name::Print(args) => Self::Print(args),
                }
            }
        }

        impl $name {
            /// The examples of `skills install`.
            const INSTALL_EXAMPLES: &'static [$crate::__private::Example] = &[
                $crate::__private::Example::new(
                    &[],
                    concat!("Install ", $tool, "'s skills into each agent skills directory that exists"),
                ),
                $crate::__private::Example::new(
                    &["--force"],
                    concat!("Install ", $tool, "'s skills, replacing a file or a link that install did not write"),
                ),
            ];

            /// The example of `skills check`.
            const CHECK_EXAMPLES: &'static [$crate::__private::Example] =
                &[$crate::__private::Example::new(
                    &[],
                    concat!("Report each agent skills directory where ", $tool, "'s skills are missing or differ"),
                )];

            /// The example of `skills print`.
            const PRINT_EXAMPLES: &'static [$crate::__private::Example] =
                &[$crate::__private::Example::new(&[], concat!("Print ", $tool, "'s skill"))];

            /// Returns the leaf of `skills install` at `path`, such as
            /// `&["skills", "install"]`, for the tool's help.
            #[must_use]
            $vis const fn install_leaf(path: &'static [&'static str]) -> $crate::__private::Leaf {
                $crate::__private::install_leaf(path, Self::INSTALL_EXAMPLES)
            }

            /// Returns the leaf of `skills check` at `path`, such as
            /// `&["skills", "check"]`, for the tool's help.
            #[must_use]
            $vis const fn check_leaf(path: &'static [&'static str]) -> $crate::__private::Leaf {
                $crate::__private::check_leaf(path, Self::CHECK_EXAMPLES)
            }

            /// Returns the leaf of `skills print` at `path`, such as
            /// `&["skills", "print"]`, for the tool's help.
            #[must_use]
            $vis const fn print_leaf(path: &'static [&'static str]) -> $crate::__private::Leaf {
                $crate::__private::print_leaf(path, Self::PRINT_EXAMPLES)
            }
        }
    };
}

/// Runs `command` for `skills`, prints its result and returns the process's
/// exit code. `install` and `check` act on every skill of `skills`, and
/// `print` prints the first, so a tool lists its main skill first. When
/// `skills` is empty, `print` prints nothing and succeeds. A tool whose
/// other skills print by name keeps its own `print` command (ADR-0201).
#[must_use]
pub fn run(run: crumbkit::Run, skills: &[Skill], command: impl Into<SkillsAction>) -> ExitCode {
    match command.into() {
        SkillsAction::Install(args) => {
            let result = Ok::<_, crumbkit::Error>(install(skills, &args));
            crumbkit::finish(run, SKILLS_INSTALL, result)
        }
        SkillsAction::Check(args) => {
            let result = Ok::<_, crumbkit::Error>(check(skills, &args));
            crumbkit::finish(run, SKILLS_CHECK, result)
        }
        SkillsAction::Print(_) => match skills {
            [first, ..] => {
                crumbkit::finish(run, SKILLS_PRINT, Ok::<_, crumbkit::Error>(print(first)))
            }
            [] => ExitCode::SUCCESS,
        },
    }
}

/// Does what [`run`] does through `sink`, in place of the standard streams,
/// and returns the exit code as a number, as [`finish_to`](crumbkit::finish_to)
/// does. A tool uses it to test its `skills` commands against a [`Sink`] in
/// memory.
#[must_use]
pub fn run_to<O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    run: crumbkit::Run,
    skills: &[Skill],
    command: impl Into<SkillsAction>,
) -> u8 {
    match command.into() {
        SkillsAction::Install(args) => {
            let result = Ok::<_, crumbkit::Error>(install(skills, &args));
            crumbkit::finish_to(sink, run, SKILLS_INSTALL, result)
        }
        SkillsAction::Check(args) => {
            let result = Ok::<_, crumbkit::Error>(check(skills, &args));
            crumbkit::finish_to(sink, run, SKILLS_CHECK, result)
        }
        SkillsAction::Print(_) => match skills {
            [first, ..] => crumbkit::finish_to(
                sink,
                run,
                SKILLS_PRINT,
                Ok::<_, crumbkit::Error>(print(first)),
            ),
            [] => 0,
        },
    }
}

/// The leaf of `skills install`, which [`skills_command!`](crate::skills_command)
/// calls with the examples that name the tool.
#[must_use]
pub const fn install_leaf(path: &'static [&'static str], examples: &'static [Example]) -> Leaf {
    Leaf::new(path)
        .examples(examples)
        .effects(Effects::additive().idempotent())
        .document(SKILLS_INSTALL)
}

/// The leaf of `skills check`, which [`skills_command!`](crate::skills_command)
/// calls with the examples that name the tool.
#[must_use]
pub const fn check_leaf(path: &'static [&'static str], examples: &'static [Example]) -> Leaf {
    Leaf::new(path)
        .examples(examples)
        .effects(Effects::read_only())
        .document(SKILLS_CHECK)
}

/// The leaf of `skills print`, which [`skills_command!`](crate::skills_command)
/// calls with the examples that name the tool.
#[must_use]
pub const fn print_leaf(path: &'static [&'static str], examples: &'static [Example]) -> Leaf {
    Leaf::new(path)
        .examples(examples)
        .effects(Effects::read_only())
        .document(SKILLS_PRINT)
}
