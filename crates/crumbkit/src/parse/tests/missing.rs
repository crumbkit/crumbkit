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

/// The shells that `tool completions` writes a script for. `elvish` is a
/// hidden possible value.
#[derive(Clone, clap::ValueEnum)]
enum Shell {
    Bash,
    #[value(hide = true)]
    Elvish,
    Fish,
    Zsh,
}

/// A tool whose `completions` leaf takes a required `SHELL` with possible
/// values, a required `DIR` without them, and an optional `--format` with
/// possible values.
fn tool_with_shells() -> Command {
    Command::new("tool").subcommand(
        Command::new("completions")
            .arg(
                Arg::new("shell")
                    .value_name("SHELL")
                    .required(true)
                    .value_parser(clap::value_parser!(Shell)),
            )
            .arg(Arg::new("dir").value_name("DIR").required(true))
            .arg(
                Arg::new("format")
                    .long("format")
                    .value_name("FORMAT")
                    .value_parser(["plain", "json"]),
            ),
    )
}

#[test]
fn a_missing_argument_with_possible_values_names_its_visible_values_after_its_name() {
    let refused = ["tool", "completions"];
    admits(
        tool_with_shells().mut_subcommand("completions", |leaf| {
            leaf.mut_arg("dir", |dir| dir.required(false))
        }),
        &refused,
        &["tool", "completions", "bash"],
    );
    let parsed = parse_tree(tool_with_shells(), &refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    let expected = "\
error: the following required arguments were not provided: <SHELL> [possible values: bash, fish, zsh] <DIR>
  help:  run `tool completions -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
}

#[test]
fn a_missing_argument_without_possible_values_is_named_alone() {
    let refused = ["tool", "completions", "bash"];
    admits(
        tool_with_shells(),
        &refused,
        &["tool", "completions", "bash", "out"],
    );
    let parsed = parse_tree(tool_with_shells(), &refused);

    assert!(
        parsed
            .err
            .starts_with("error: the following required arguments were not provided: <DIR>\n"),
        "{}",
        parsed.err
    );
}

#[test]
fn a_missing_option_with_possible_values_names_them_after_the_option() {
    let tool = || {
        tool_with_shells().mut_subcommand("completions", |leaf| {
            leaf.mut_arg("format", |format| format.required(true))
        })
    };
    let refused = ["tool", "completions", "bash", "out"];
    admits(
        tool(),
        &refused,
        &["tool", "completions", "bash", "out", "--format=json"],
    );
    let parsed = parse_tree(tool(), &refused);

    assert!(
        parsed.err.starts_with(
            "error: the following required arguments were not provided: \
             --format <FORMAT> [possible values: plain, json]\n"
        ),
        "{}",
        parsed.err
    );
}

#[test]
fn an_invalid_value_names_the_possible_values_in_the_same_words() {
    let refused = ["tool", "completions", "nu", "out"];
    admits(
        tool_with_shells(),
        &refused,
        &["tool", "completions", "bash", "out"],
    );
    let parsed = parse_tree(tool_with_shells(), &refused);

    assert!(
        parsed.err.starts_with(
            "error: invalid value 'nu' for '<SHELL>' [possible values: bash, fish, zsh]\n"
        ),
        "{}",
        parsed.err
    );
}
