// The crate docs are the README. The README links to files in the
// repository, which work on GitHub. The definitions below come first, so
// rustdoc uses them for the same labels (ADR-0013).
//! [schema]: https://github.com/crumbkit/crumbkit/blob/main/crates/cargo-crumbkit/schemas/weigh-v1.json
//! [shelf's weights]: https://github.com/crumbkit/crumbkit/blob/main/crates/cargo-crumbkit/tests/golden/examples/cargo-crumbkit-weigh-bin-shelf.txt
//! [ADR-0033]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0033-weigh.md
//! [shelf's reference]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/docs/commands.md
//! [ADR-0044]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0044-command-reference.md
//! [weigh docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/help/weight.md
//! [command reference docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/help/reference.md
//! [skill check docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/help/skill-check/README.md
//! [ADR-0193]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/skill/0193-skill-check.md
//! [skill draft docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/help/skill-draft/README.md
//! [ADR-0194]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/skill/README.md
//! [the report schema]: https://github.com/crumbkit/crumbkit/blob/main/crates/cargo-crumbkit/skill/audit-report-v1.json
//! [ADR-0199]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/skill/0199-skill-files.md
#![doc = include_str!("../README.md")]

mod cargo;
mod cli;
mod docs;
mod error;
mod tool;
mod tool_skill;
mod weigh;

use std::process::ExitCode;

// These items exist for the binary in `main.rs` and for the crate's own
// tests, so they stay out of the documentation.
#[doc(hidden)]
pub use crate::cli::{ALLOW, Cli, TOOL};
use crate::cli::{CargoCommand, CommandsCommand, CrumbkitCommand, SKILL, ToolSkillCommand};

#[cfg(test)]
mod tests;

// `run` exists for the binary in `main.rs` and for the crate's own tests.
#[doc(hidden)]
/// Parses the process's command line, runs the command and returns the
/// process's exit code.
///
/// cargo runs `cargo crumbkit weigh` as `cargo-crumbkit crumbkit weigh`, so
/// the command tree's root is `cargo` and every command line that help,
/// `Next:` lines and recoveries print starts with `cargo crumbkit`.
#[must_use]
pub fn run() -> ExitCode {
    let (cli, mut output) = match crumbkit::parse::<Cli>(&TOOL) {
        Ok(parsed) => parsed,
        Err(stop) => return stop.exit_code(),
    };
    match cli.cargo {
        CargoCommand::Crumbkit { command } => match command {
            CrumbkitCommand::Weigh(args) => {
                crumbkit::finish(output, weigh::WEIGH, weigh::weigh(&args))
            }
            CrumbkitCommand::Docs(args) => crumbkit::finish(output, docs::DOCS, docs::docs(&args)),
            CrumbkitCommand::ToolSkill { command } => match command {
                ToolSkillCommand::Check(args) => crumbkit::finish(
                    output,
                    tool_skill::TOOL_SKILL_CHECK,
                    tool_skill::check(&args),
                ),
                ToolSkillCommand::Draft(args) => crumbkit::finish(
                    output,
                    tool_skill::TOOL_SKILL_DRAFT,
                    tool_skill::draft(&args),
                ),
            },
            CrumbkitCommand::Skills { command } => crumbkit_skills::run(output, &[SKILL], &command),
            CrumbkitCommand::Commands { command } => match command {
                // `commands list` reads the command tree through
                // `&mut output`, so it runs before `finish`.
                CommandsCommand::List(args) => {
                    let result = crumbkit::commands::list(&mut output, &TOOL, &args);
                    let document = crumbkit::commands::COMMANDS_LIST;
                    crumbkit::finish(output, document, result)
                }
            },
        },
    }
}
