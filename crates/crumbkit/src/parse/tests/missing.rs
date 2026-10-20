//! Tests for a command line that leaves out a word that the tool needs: a
//! subcommand of a group, or a required argument of a leaf.

use std::process::ExitCode;

use clap::{Arg, Command};

use super::{admits, parse, parse_tree, tool};

/// A tool whose `planets` group holds the visible leaves `show` and `weigh`
/// and the hidden leaf `orbit` between them.
fn tool_with_three_leaves() -> Command {
    Command::new("tool").subcommand(
        Command::new("planets")
            .subcommand_required(true)
            .subcommand(Command::new("show"))
            .subcommand(Command::new("orbit").hide(true))
            .subcommand(Command::new("weigh")),
    )
}

#[test]
fn a_command_line_that_stops_at_a_group_says_that_the_group_needs_a_subcommand() {
    admits(tool(), &["tool", "planets"], &["tool", "planets", "weigh"]);
    let parsed = parse(&["tool", "planets"]);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.out, "");
    let expected = "\
error: the command `tool planets` needs a subcommand: show
  help:  run `tool planets -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
}

#[test]
fn a_group_s_refusal_names_its_visible_subcommands_in_declaration_order() {
    admits(
        tool_with_three_leaves(),
        &["tool", "planets"],
        &["tool", "planets", "weigh"],
    );
    let parsed = parse_tree(tool_with_three_leaves(), &["tool", "planets"]);

    let expected = "\
error: the command `tool planets` needs a subcommand: show, weigh
  help:  run `tool planets -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
}

#[test]
fn a_leaf_that_needs_arguments_names_no_subcommands() {
    let tool = || {
        Command::new("tool").subcommand(
            Command::new("show")
                .arg_required_else_help(true)
                .arg(Arg::new("name")),
        )
    };
    admits(tool(), &["tool", "show"], &["tool", "show", "Mars"]);
    let parsed = parse_tree(tool(), &["tool", "show"]);

    assert!(
        parsed
            .err
            .starts_with("error: the command `tool show` needs arguments\n"),
        "{}",
        parsed.err
    );
}
