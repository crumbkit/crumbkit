//! Tests for `reach`: which lazy groups it expands, and the tree it leaves.

use clap::{Arg, ArgAction, Command, CommandFactory, Parser, Subcommand};

use crate::{Lazy, reach};

#[derive(Debug, Parser)]
#[command(name = "tool")]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Top,
}

#[derive(Debug, Subcommand)]
enum Top {
    Books,
    /// Work with the skill.
    Skills {
        #[command(subcommand)]
        command: Lazy<Skills>,
    },
    Tools {
        #[command(subcommand)]
        command: Tools,
    },
}

/// The commands of the skill.
#[derive(Debug, Subcommand)]
enum Skills {
    /// Install the skill.
    Install,
}

#[derive(Debug, Subcommand)]
enum Tools {
    Inner {
        #[command(subcommand)]
        command: Lazy<Skills>,
    },
}

fn names(command: &Command) -> Vec<&str> {
    command.get_subcommands().map(Command::get_name).collect()
}

#[test]
fn reach_expands_a_lazy_group_only_when_its_word_is_typed() {
    let mut command = Cli::command();
    reach(&mut command, &["books"]);
    assert_eq!(
        names(command.find_subcommand("skills").unwrap()),
        Vec::<&str>::new()
    );

    reach(&mut command, &["skills", "install"]);
    assert_eq!(
        names(command.find_subcommand("skills").unwrap()),
        ["install"]
    );
}

#[test]
fn a_lazy_group_s_leaf_takes_the_root_s_global_flags() {
    let mut command = Cli::command();
    reach(&mut command, &["skills", "install"]);
    let leaf = command
        .find_subcommand("skills")
        .and_then(|group| group.find_subcommand("install"))
        .unwrap();
    assert!(leaf.get_arguments().any(|arg| arg.get_id() == "json"));
    let matches = command
        .try_get_matches_from(["tool", "skills", "install", "--json"])
        .unwrap();
    assert!(matches.get_flag("json"));
}

#[test]
fn reach_never_adds_clap_s_help_subcommand() {
    let mut command = Cli::command();
    reach(&mut command, &["skills", "install"]);
    assert!(command.find_subcommand("help").is_none());
    assert!(
        command
            .find_subcommand("skills")
            .unwrap()
            .find_subcommand("help")
            .is_none()
    );
}

#[test]
fn a_lazy_group_two_levels_below_the_root_expands_when_its_word_is_typed() {
    let mut command = Cli::command();
    reach(&mut command, &["tools", "inner", "install", "--json"]);
    let inner = command
        .find_subcommand("tools")
        .and_then(|tools| tools.find_subcommand("inner"))
        .unwrap();
    assert_eq!(names(inner), ["install"]);
    let leaf = inner.find_subcommand("install").unwrap();
    assert!(leaf.get_arguments().any(|arg| arg.get_id() == "json"));
}

#[test]
fn reach_reads_a_group_word_after_a_flag_that_takes_a_value() {
    let mut command = Command::new("tool")
        .arg(
            Arg::new("dir")
                .short('C')
                .global(true)
                .action(ArgAction::Set),
        )
        .subcommand(
            Command::new("skills")
                .subcommand_value_name("COMMAND")
                .defer(|command| command.subcommand(Command::new("install"))),
        );
    reach(&mut command, &["-C", "skills", "skills", "install"]);
    assert_eq!(
        names(command.find_subcommand("skills").unwrap()),
        ["install"]
    );
}

#[test]
fn reach_leaves_a_command_that_has_no_lazy_group_unbuilt() {
    let mut command = Cli::command();
    reach(&mut command, &["books"]);
    // A tree that reach built would hold the help flag it adds in a build.
    assert!(command.get_arguments().all(|arg| arg.get_id() != "help"));
}

/// What `try_parse_to` prints for `args` against `command`.
fn printed(command: Command, args: &[&str]) -> (String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = crate::parse::try_parse_to::<()>(
        &mut crate::Sink::new(&mut out, &mut err),
        command,
        &crate::Tool::new(),
        args,
    );
    if let Err(crate::Unparsed::Refused(refused)) = result {
        let _ = refused.print_to(&mut crate::Sink::new(&mut out, &mut err));
    }
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

// The same tool as `Cli`, with each group declared without `Lazy`.
#[derive(Debug, Parser)]
#[command(name = "tool")]
struct EagerCli {
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: EagerTop,
}

#[derive(Debug, Subcommand)]
enum EagerTop {
    Books,
    /// Work with the skill.
    Skills {
        #[command(subcommand)]
        command: Skills,
    },
    Tools {
        #[command(subcommand)]
        command: EagerTools,
    },
}

#[derive(Debug, Subcommand)]
enum EagerTools {
    Inner {
        #[command(subcommand)]
        command: Skills,
    },
}

fn eager() -> Command {
    EagerCli::command()
}

#[test]
fn a_lazy_group_prints_the_same_help_and_refusals_as_an_eager_one() {
    for args in [
        &["tool", "skills", "--help"][..],
        &["tool", "skills", "install", "--help"],
        &["tool", "skills", "-h"],
        &["tool", "help", "skills"],
        &["tool", "help", "skills", "install"],
        &["tool", "skills", "help", "install"],
        &["tool", "skills", "nope"],
        &["tool", "skills", "install", "--bogus"],
        &["tool", "tools", "inner", "install", "--help"],
        &["tool", "help", "tools", "inner", "install"],
        &["tool", "-h"],
    ] {
        assert_eq!(
            printed(Cli::command(), args),
            printed(eager(), args),
            "{args:?}"
        );
    }
}
