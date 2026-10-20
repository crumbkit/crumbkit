//! Tests for clap's usage line, which the text form of a refusal from clap
//! prints under its `error:` line.

use std::process::ExitCode;

use clap::{Arg, Command};

use super::super::message::usage;
use super::{admits, parse, parse_tree, tool};
use crate::ViewFlags;

#[test]
fn a_missing_argument_s_refusal_prints_clap_s_usage_line_between_the_error_and_the_help_lines() {
    let refused = ["tool", "planets", "show"];
    admits(tool(), &refused, &["tool", "planets", "show", "Mars"]);
    let parsed = parse(&refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.out, "");
    let expected = "\
error: the following required arguments were not provided: <NAME>...
  usage: tool planets show <NAME>...
  help:  run `tool planets show -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
}

#[test]
fn a_refused_view_flag_prints_no_usage_line() {
    /// A tool whose root flattens the view flags.
    #[derive(clap::Parser)]
    #[command(name = "tool")]
    struct Viewed {
        #[command(flatten)]
        view: ViewFlags,
    }
    let command = <Viewed as clap::CommandFactory>::command();
    let refused = ["tool", "--full"];
    admits(command.clone(), &refused, &["tool", "--full", "--json"]);
    let parsed = parse_tree(command, &refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert!(parsed.err.starts_with("error: "), "{}", parsed.err);
    assert!(!parsed.err.contains("usage:"), "{}", parsed.err);
}

/// A tool that takes a required `NAME`, or a `show` subcommand in its place,
/// so clap's usage has two forms.
fn tool_with_two_forms() -> Command {
    Command::new("tool")
        .subcommand_negates_reqs(true)
        .arg(Arg::new("name").value_name("NAME").required(true))
        .subcommand(Command::new("show"))
}

#[test]
fn a_refusal_on_a_command_with_two_usage_forms_prints_a_usage_line_for_each() {
    let refused = ["tool", "--bogus"];
    admits(tool_with_two_forms(), &refused, &["tool", "Mars"]);
    let parsed = parse_tree(tool_with_two_forms(), &refused);

    let expected = "\
error: unexpected argument '--bogus' found; to pass '--bogus' as a value, use '-- --bogus'
  usage: tool <NAME>
  usage: tool [NAME] <COMMAND>
  help:  run `tool -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
}

#[test]
fn the_usage_line_names_the_tool_by_its_file_name_as_help_does_when_the_program_is_a_path() {
    let refused = ["/usr/local/bin/tool", "planets", "show"];
    admits(
        tool(),
        &refused,
        &["/usr/local/bin/tool", "planets", "show", "Mars"],
    );
    let parsed = parse(&refused);
    let page = parse(&["/usr/local/bin/tool", "planets", "show", "-h"]).out;

    assert!(
        parsed
            .err
            .contains("\n  usage: tool planets show <NAME>...\n"),
        "{}",
        parsed.err
    );
    assert!(
        page.contains("\nUsage: tool planets show [OPTIONS] <NAME>...\n"),
        "{page}"
    );
}

#[test]
fn usage_keeps_each_line_from_the_usage_label_to_the_blank_line_without_the_label() {
    let rendered = "\
error: unexpected argument '--bogus' found

Usage: tool <NAME>
       tool <COMMAND>

For more information, try '--help'.
";

    assert_eq!(usage(rendered), ["tool <NAME>", "tool <COMMAND>"]);
}

#[test]
fn usage_is_empty_when_clap_prints_no_usage_line() {
    let rendered = "\
error: invalid value '101' for '--limit <N>': 101 is above the most, 100

For more information, try '--help'.
";

    assert_eq!(usage(rendered), Vec::<String>::new());
}
