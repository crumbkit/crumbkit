//! cargo-crumbkit's command-line grammar.

use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand};
use crumbkit::commands::{COMMANDS_LIST, CommandsList};
use crumbkit::help::{Effects, Example, Goal, Group, Help, Leaf};
use crumbkit::{Tool, ViewFlags};
use crumbkit_skills::{
    SKILLS_CHECK, SKILLS_INSTALL, SKILLS_PRINT, Skill, SkillsCheck, SkillsInstall,
};
use crumbkit_test::{Allow, Rule};

use crate::docs::DOCS;
use crate::tool_skill::{TOOL_SKILL_CHECK, TOOL_SKILL_DRAFT};
use crate::weigh::WEIGH;

#[cfg(test)]
mod tests;

/// Cargo subcommands for tools built with crumbkit.
// cargo runs the binary `cargo-crumbkit` with the words `crumbkit weigh`, so
// the root is named `cargo` and `crumbkit` is its one subcommand. Help and
// every printed command line then start with `cargo crumbkit`, which is what
// a person types.
#[derive(Debug, Parser)]
#[command(name = "cargo", bin_name = "cargo")]
pub struct Cli {
    /// The cargo subcommand, which is always `crumbkit`.
    #[command(subcommand)]
    pub(crate) cargo: CargoCommand,

    /// The flags that choose how the result prints.
    #[command(flatten)]
    pub(crate) view: ViewFlags,
}

/// cargo-crumbkit's goal headings and leaves. It gives goal headings to the
/// `crumbkit` group, so `cargo crumbkit -h` lists `weigh` under `Measure a
/// tool's help`, `docs` under `Write a tool's reference`, `tool-skill` and
/// `skills` under `Work with agent skills` and `commands` under `Explore the
/// commands`. The
/// help of each leaf lists its examples, and its `--help` describes the
/// columns of the rows it prints. Its root is
/// `crumbkit`, because cargo always passes that word, so `cargo crumbkit help`
/// prints the long help of `cargo crumbkit`, and `cargo crumbkit help
/// exit-codes` prints a topic.
pub const TOOL: Tool = Tool::new().help(
    Help::new()
        .root(&["crumbkit"])
        .groups(&[Group::new(
            &["crumbkit"],
            &[
                Goal::new("Measure a tool's help", &["weigh"]),
                Goal::new("Write a tool's reference", &["docs"]),
                Goal::new("Work with agent skills", &["tool-skill", "skills"]),
                Goal::new("Explore the commands", &["commands"]),
            ],
        )])
        .leaves(&[
            Leaf::new(&["crumbkit", "weigh"])
                .examples(&[
                    Example::new(&["--bin", "shelf"], "Weigh each path through shelf's help"),
                    Example::new(&["--bin", "shelf", "--json"], "Print the weights as JSON"),
                    Example::new(
                        &["--bin", "shelf", "--usage"],
                        "Add the figures of shelf's own usage log",
                    ),
                ])
                .document(WEIGH),
            Leaf::new(&["crumbkit", "docs"])
                .examples(&[Example::new(
                    &["--bin", "shelf"],
                    "Write shelf's reference to its docs/commands.md",
                )])
                .effects(Effects::additive().idempotent())
                .document(DOCS),
            Leaf::new(&["crumbkit", "tool-skill", "check"])
                .examples(&[Example::new(
                    &["--bin", "shelf", "--skill", "tests/fixtures/shelf-skill.md"],
                    "Check every shelf command line in a skill file",
                )])
                .document(TOOL_SKILL_CHECK),
            Leaf::new(&["crumbkit", "tool-skill", "draft"])
                .examples(&[
                    Example::new(
                        &[
                            "--bin",
                            "shelf",
                            "--skill",
                            "tests/fixtures/shelf-skill-drafted.md",
                            "--usage",
                            "tests/fixtures/shelf-usage-export",
                            "--check",
                        ],
                        "Check that a skill file holds the sections that a draft from usage writes",
                    ),
                    Example::new(
                        &[
                            "--bin",
                            "shelf",
                            "--skill",
                            "tests/fixtures/shelf-skill-drafted.md",
                            "--check",
                            "--json",
                        ],
                        "Check the sections without usage, which keeps the usage section",
                    ),
                ])
                .effects(Effects::additive().idempotent())
                .document(TOOL_SKILL_DRAFT),
            Leaf::new(&["crumbkit", "skills", "install"])
                .examples(&[
                    Example::new(
                        &[],
                        "Install cargo crumbkit's skill into each agent skills directory that exists",
                    ),
                    Example::new(
                        &["--force"],
                        "Install it, replacing a file or a link that install did not write",
                    ),
                ])
                .effects(Effects::additive().idempotent())
                .document(SKILLS_INSTALL),
            Leaf::new(&["crumbkit", "skills", "check"])
                .examples(&[Example::new(
                    &[],
                    "Report each agent skills directory whose copy is missing or differs",
                )])
                .effects(Effects::read_only())
                .document(SKILLS_CHECK),
            Leaf::new(&["crumbkit", "skills", "print"])
                .examples(&[Example::new(&[], "Print cargo crumbkit's skill")])
                .effects(Effects::read_only())
                .document(SKILLS_PRINT),
            Leaf::new(&["crumbkit", "commands", "list"])
                .examples(&[Example::new(&[], "List every command")])
                .document(COMMANDS_LIST),
        ]),
);

/// cargo-crumbkit's opt-outs from crumbkit's conventions, which its tests
/// pass to every check that takes them.
pub const ALLOW: &[Allow] = &[
    Allow::new(
        Rule::Examples,
        "crumbkit docs",
        "Each run of the example writes shelf's reference into the workspace, so tests/docs.rs runs docs with a file in a temporary directory instead.",
    ),
    Allow::new(
        Rule::Examples,
        "crumbkit skills install",
        "The example test passes HOME on for cargo's builds, so each run of the example would install into the person's own skills directories.",
    ),
    Allow::new(
        Rule::Examples,
        "crumbkit skills check",
        "The example test passes HOME on for cargo's builds, so the output would depend on the person's own skills directories.",
    ),
    Allow::new(
        Rule::Recoveries,
        "",
        "Each failure of weigh, docs, tool-skill check and tool-skill draft comes from cargo, a build, the tool that it runs, the file system or a broken skill file, so a test provokes one only by building a broken workspace or breaking a file.",
    ),
];

/// The cargo subcommands that cargo-crumbkit serves.
#[derive(Debug, Subcommand)]
pub(crate) enum CargoCommand {
    /// Measure and explore tools built with crumbkit.
    #[command(version)]
    Crumbkit {
        /// What to do.
        #[command(subcommand)]
        command: CrumbkitCommand,
    },
}

/// The commands of `cargo crumbkit`.
#[derive(Debug, Subcommand)]
pub(crate) enum CrumbkitCommand {
    /// Measure the bytes that an agent reads on each path through a tool's help.
    Weigh(WeighArgs),
    /// Write a Markdown reference of every command of a tool to a file.
    Docs(DocsArgs),
    /// Draft and check a tool's agent skill.
    ToolSkill {
        /// What to do with the tool's skill.
        #[command(subcommand)]
        command: ToolSkillCommand,
    },
    /// Install cargo crumbkit's own skill.
    Skills {
        /// What to do with cargo crumbkit's skill.
        #[command(subcommand)]
        command: SkillsCommand,
    },
    /// Explore cargo crumbkit's commands.
    Commands {
        /// What to do with the commands.
        #[command(subcommand)]
        command: CommandsCommand,
    },
}

/// The commands for cargo crumbkit's own commands.
#[derive(Debug, Subcommand)]
pub(crate) enum CommandsCommand {
    /// List a branch of cargo crumbkit's commands with their arguments, examples and columns.
    List(CommandsList),
}

/// The commands for cargo crumbkit's own agent skill, which crumbkit-skills
/// provides.
#[derive(Debug, Subcommand)]
pub(crate) enum SkillsCommand {
    /// Install cargo crumbkit's skill into each agent skills directory that exists.
    Install(SkillsInstall),
    /// Check that each installed copy of cargo crumbkit's skill matches this cargo crumbkit.
    Check(SkillsCheck),
    /// Print cargo crumbkit's skill.
    Print,
}

/// cargo crumbkit's own agent skill, which `cargo crumbkit skills install`
/// writes into the agent skills directories. It tells an agent what each
/// `cargo crumbkit` command is for, and how to audit a tool for places that
/// should use crumbkit, with the table of crumbkit's affordances and the
/// schema of the audit's report beside it.
pub(crate) const SKILL: Skill = Skill::new(
    "cargo-crumbkit",
    env!("CARGO_PKG_VERSION"),
    include_str!("../skill/SKILL.md"),
)
.files(&[
    ("AFFORDANCES.md", include_str!("../skill/AFFORDANCES.md")),
    (
        "audit-report-v1.json",
        include_str!("../skill/audit-report-v1.json"),
    ),
]);

/// The commands for a tool's agent skill.
#[derive(Debug, Subcommand)]
pub(crate) enum ToolSkillCommand {
    /// Check that every command line in a tool's SKILL.md names its real commands and flags.
    Check(ToolSkillCheckArgs),
    /// Write the generated sections of a tool's SKILL.md from its commands and exported usage.
    Draft(ToolSkillDraftArgs),
}

/// The arguments of `cargo crumbkit weigh`.
// `--since` requires the group of `--usage` and `--log`, so it needs one of
// them, and `--log` alone turns the figures on.
#[derive(Debug, Args)]
#[command(group(ArgGroup::new("figures").args(["usage", "log"]).multiple(true)))]
pub(crate) struct WeighArgs {
    /// Build this binary of the current workspace and weigh its help
    #[arg(long, value_name = "NAME")]
    pub(crate) bin: String,

    /// Read the examples' golden files from this directory [default: tests/golden/examples in the binary's package]
    #[arg(long, value_name = "DIR")]
    pub(crate) goldens: Option<PathBuf>,

    /// Add the runs, help pages and output of each command from the tool's usage log
    #[arg(long)]
    usage: bool,

    /// Read the usage figures from this log instead of the tool's own; implies --usage
    #[arg(long, value_name = "FILE")]
    pub(crate) log: Option<PathBuf>,

    /// How far back to read the usage log, such as 12h, 7d or 2w [default: 7d]
    #[arg(long, value_name = "DURATION", requires = "figures")]
    pub(crate) since: Option<String>,
}

impl WeighArgs {
    /// Reports whether `weigh` adds the usage figures, which `--usage` or
    /// `--log` asks for.
    pub(crate) const fn usage(&self) -> bool {
        self.usage || self.log.is_some()
    }
}

/// The arguments of `cargo crumbkit docs`.
#[derive(Debug, Args)]
pub(crate) struct DocsArgs {
    /// Build this binary of the current workspace and describe its commands
    #[arg(long, value_name = "NAME")]
    pub(crate) bin: String,

    /// Write the reference to this file [default: docs/commands.md in the binary's package]
    #[arg(long, value_name = "FILE")]
    pub(crate) out: Option<PathBuf>,
}

/// The arguments of `cargo crumbkit tool-skill check`.
#[derive(Debug, Args)]
pub(crate) struct ToolSkillCheckArgs {
    /// Build this binary of the current workspace and check its skill against its commands
    #[arg(long, value_name = "NAME")]
    pub(crate) bin: String,

    /// Check this Markdown file [default: skill/SKILL.md in the binary's package]
    #[arg(long, value_name = "FILE")]
    pub(crate) skill: Option<PathBuf>,
}

/// The arguments of `cargo crumbkit tool-skill draft`.
#[derive(Debug, Args)]
pub(crate) struct ToolSkillDraftArgs {
    /// Build this binary of the current workspace and draft its skill from its commands
    #[arg(long, value_name = "NAME")]
    pub(crate) bin: String,

    /// Draft this Markdown file [default: skill/SKILL.md in the binary's package]
    #[arg(long, value_name = "FILE")]
    pub(crate) skill: Option<PathBuf>,

    /// Rank the most-used commands from the exported usage files (*.jsonl) under this directory
    #[arg(long, value_name = "DIR")]
    pub(crate) usage: Option<PathBuf>,

    /// List at most this many commands in the most-used section
    #[arg(long, value_name = "N", default_value_t = 10)]
    pub(crate) top: usize,

    /// Write nothing, and exit with 1 when a draft would change a section
    #[arg(long)]
    pub(crate) check: bool,
}
