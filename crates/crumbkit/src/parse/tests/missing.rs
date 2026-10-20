//! Tests for a command line that leaves out a word that the tool needs: a
//! subcommand of a group, or a required argument of a leaf.

use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};

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
fn a_command_line_that_stops_at_a_group_prints_the_group_s_help_page_on_standard_error_with_code_2()
{
    admits(tool(), &["tool", "planets"], &["tool", "planets", "weigh"]);
    let parsed = parse(&["tool", "planets"]);
    let page = parse(&["tool", "planets", "-h"]).out;

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.out, "");
    assert!(page.starts_with("Work with planets\n"), "{page}");
    assert_eq!(parsed.err, page);
}

#[test]
fn a_group_that_clap_refuses_without_its_page_prints_its_help_page_too() {
    admits(
        tool_with_three_leaves(),
        &["tool", "planets"],
        &["tool", "planets", "weigh"],
    );
    let parsed = parse_tree(tool_with_three_leaves(), &["tool", "planets"]);
    let page = parse_tree(tool_with_three_leaves(), &["tool", "planets", "-h"]).out;

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.out, "");
    assert!(
        page.starts_with("Usage: tool planets <COMMAND>\n"),
        "{page}"
    );
    assert_eq!(parsed.err, page);
}

#[test]
fn a_command_line_that_stops_at_a_group_with_json_prints_the_failure_document_that_names_its_subcommands()
 {
    let refused = ["tool", "planets", "--json"];
    admits(tool(), &refused, &["tool", "planets", "weigh", "--json"]);
    let parsed = parse(&refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    let expected = concat!(
        r#"{"schema":"tool/failure/v1","code":"tool::cli::usage","#,
        r#""message":"the command `tool planets` needs a subcommand: show","#,
        r#""recovery":{"kind":"run","call":"tool planets -h"}}"#,
        "\n",
    );
    assert_eq!(parsed.out, expected);
    assert_eq!(parsed.err, "");
}

#[test]
fn a_group_s_failure_document_names_its_visible_subcommands_in_declaration_order() {
    let tool = || {
        tool_with_three_leaves().arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue),
        )
    };
    let refused = ["tool", "planets", "--json"];
    admits(tool(), &refused, &["tool", "planets", "weigh", "--json"]);
    let parsed = parse_tree(tool(), &refused);

    assert!(
        parsed
            .out
            .contains(r#""message":"the command `tool planets` needs a subcommand: show, weigh""#),
        "{}",
        parsed.out
    );
}

#[test]
fn a_leaf_that_needs_arguments_keeps_the_refusal_lines_with_its_usage_line() {
    let tool = || {
        Command::new("tool").subcommand(
            Command::new("show")
                .arg_required_else_help(true)
                .arg(Arg::new("name")),
        )
    };
    admits(tool(), &["tool", "show"], &["tool", "show", "Mars"]);
    let parsed = parse_tree(tool(), &["tool", "show"]);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    let expected = "\
error: the command `tool show` needs arguments
  usage: tool show [name]
  help:  run `tool show -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
}

/// A tool whose `copy` leaf takes the required positionals `APP` and `NAME`,
/// and whose `rename` leaf takes a required `--edit <APP>` option and the
/// positional `NAME`. `NAME` is required when `name_required` is true.
fn tool_with_apps(name_required: bool) -> Command {
    let name = || Arg::new("name").value_name("NAME").required(name_required);
    Command::new("tool")
        .subcommand(
            Command::new("copy")
                .arg(Arg::new("app").value_name("APP").required(true))
                .arg(name()),
        )
        .subcommand(
            Command::new("rename")
                .arg(
                    Arg::new("edit")
                        .long("edit")
                        .value_name("APP")
                        .required(true),
                )
                .arg(name()),
        )
}

#[test]
fn a_refusal_separates_two_missing_positionals_with_a_comma() {
    let refused = ["tool", "copy"];
    admits(tool_with_apps(false), &refused, &["tool", "copy", "notes"]);
    let parsed = parse_tree(tool_with_apps(true), &refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    let expected = "\
error: the following required arguments were not provided: <APP>, <NAME>
  usage: tool copy <APP> <NAME>
  help:  run `tool copy -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
}

#[test]
fn a_refusal_separates_a_missing_option_from_a_missing_positional_with_a_comma() {
    let refused = ["tool", "rename"];
    admits(
        tool_with_apps(false),
        &refused,
        &["tool", "rename", "--edit=notes"],
    );
    let parsed = parse_tree(tool_with_apps(true), &refused);

    assert!(
        parsed.err.starts_with(
            "error: the following required arguments were not provided: \
             --edit <APP>, <NAME>\n"
        ),
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
error: the following required arguments were not provided: <SHELL> [possible values: bash, fish, zsh], <DIR>
  usage: tool completions <SHELL> <DIR>
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
