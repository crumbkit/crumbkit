//! Tests for `parse_to`: what it returns, and what it prints for help, the
//! version and a refused command line.

use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};

use super::{message, parse_to};
use crate::help::Guide;
use crate::{Sink, Spelling};

/// A tool with two global flags, a version, and a group that holds a visible
/// leaf with an alias and a hidden leaf.
fn tool() -> Command {
    Command::new("tool")
        .version("1.2.3")
        .about("Explore a small solar system")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print JSON"),
        )
        .arg(
            Arg::new("fields")
                .long("fields")
                .value_name("FIELD")
                .global(true)
                .help("Print only these fields"),
        )
        .subcommand(
            Command::new("planets")
                .about("Work with planets")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("show")
                        .about("Show planets by name")
                        .alias("get")
                        .arg(
                            Arg::new("names")
                                .value_name("NAME")
                                .required(true)
                                .num_args(1..)
                                .help("The planets to show"),
                        )
                        .arg(
                            Arg::new("sort")
                                .long("sort")
                                .value_name("KEY")
                                .help("Sort by this key"),
                        ),
                )
                .subcommand(Command::new("weigh").hide(true).about("Weigh a planet")),
        )
}

/// The `--json` flag as a parsed type, so a test can see that `parse_to`
/// returns the tool's own type.
#[derive(clap::Args)]
struct Flags {
    json: bool,
}

/// What `parse_to` printed, and the exit code it returned when it did not
/// return a parsed type.
struct Parsed {
    code: Option<ExitCode>,
    out: String,
    err: String,
}

/// Runs `parse_to` over `tool()` with `args` into memory, for a command line
/// that does not parse.
fn parse(args: &[&str]) -> Parsed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        tool(),
        &Guide::new(),
        Spelling::COLONS,
        args,
    );
    Parsed {
        code: result.err(),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

#[test]
fn a_command_line_that_parses_returns_the_parsed_type_and_the_command() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<Flags, _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        tool(),
        &Guide::new(),
        Spelling::COLONS,
        ["tool", "--json", "planets", "show", "Mars"],
    );

    let Ok((flags, command)) = result else {
        panic!("the command line did not parse");
    };
    assert!(flags.json);
    assert_eq!(command.get_name(), "tool");
    assert_eq!(out, b"");
    assert_eq!(err, b"");
}

#[test]
fn a_refused_command_line_prints_the_usage_failure_on_standard_error_and_exits_with_code_2() {
    let parsed = parse(&["tool", "planets", "show", "Mars", "--srot", "name"]);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.out, "");
    let expected = "\
error: unexpected argument '--srot' found; a similar argument exists: '--sort'
  help:  run `tool planets show -h`
  code:  crumbkit::cli::usage
";
    assert_eq!(parsed.err, expected);
}

#[test]
fn a_refused_command_line_with_json_before_the_double_dash_prints_the_failure_document() {
    let parsed = parse(&[
        "tool", "planets", "show", "Mars", "--srot", "name", "--json",
    ]);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    let expected = concat!(
        r#"{"schema":"crumbkit/failure/v1","code":"crumbkit::cli::usage","#,
        r#""message":"unexpected argument '--srot' found; a similar argument exists: '--sort'","#,
        r#""recovery":{"kind":"run","call":"tool planets show -h"}}"#,
        "\n",
    );
    assert_eq!(parsed.out, expected);
    assert_eq!(parsed.err, "");
}

#[test]
fn the_recovery_names_a_command_typed_by_its_alias_by_its_real_name() {
    let parsed = parse(&["tool", "planets", "get", "--bogus"]);

    assert!(parsed.err.contains("  help:  run `tool planets show -h`\n"));
}

#[test]
fn the_recovery_names_a_hidden_command_that_the_caller_typed() {
    let parsed = parse(&["tool", "planets", "weigh", "--bogus"]);

    assert!(
        parsed
            .err
            .contains("  help:  run `tool planets weigh -h`\n")
    );
}

#[test]
fn the_walk_stops_at_a_double_dash() {
    let parsed = parse(&["tool", "planets", "--", "show", "Mars"]);

    assert!(parsed.err.contains("  help:  run `tool planets -h`\n"));
}

#[test]
fn a_command_line_that_stops_at_a_group_says_that_the_group_needs_a_subcommand() {
    let parsed = parse(&["tool", "planets"]);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.out, "");
    let expected = "\
error: the command `tool planets` needs a subcommand
  help:  run `tool planets -h`
  code:  crumbkit::cli::usage
";
    assert_eq!(parsed.err, expected);
}

#[test]
fn short_help_names_the_global_flags_on_one_line_and_exits_with_code_0() {
    let parsed = parse(&["tool", "planets", "show", "-h"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    let expected = "\
Show planets by name

Usage: tool planets show [OPTIONS] <NAME>...

Arguments:
  <NAME>...  The planets to show

Options:
      --sort <KEY>  Sort by this key
  -h, --help        Print help (see more with '--help')

Global flags: --json, --fields <FIELD>. `tool planets show --help` describes them.
";
    assert_eq!(parsed.out, expected);
    assert_eq!(parsed.err, "");
}

#[test]
fn long_help_lists_the_global_flags_in_full_without_the_global_flags_line() {
    let parsed = parse(&["tool", "planets", "show", "--help"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    let expected = "\
Show planets by name

Usage: tool planets show [OPTIONS] <NAME>...

Arguments:
  <NAME>...
          The planets to show

Options:
      --json
          Print JSON

      --sort <KEY>
          Sort by this key

      --fields <FIELD>
          Print only these fields

  -h, --help
          Print help (see a summary with '-h')
";
    assert_eq!(parsed.out, expected);
}

#[test]
fn a_groups_help_ends_with_the_next_line_that_opens_one_of_its_commands() {
    let parsed = parse(&["tool", "planets", "-h"]);

    let expected = "\
Work with planets

Usage: tool planets [OPTIONS] <COMMAND>

Commands:
  show  Show planets by name

Options:
  -h, --help  Print help (see more with '--help')

Global flags: --json, --fields <FIELD>. `tool planets --help` describes them.
Next: tool planets <COMMAND> -h
";
    assert_eq!(parsed.out, expected);
}

#[test]
fn help_with_a_command_path_prints_that_command_s_long_help() {
    let parsed = parse(&["tool", "help", "planets", "show"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    assert_eq!(
        parsed.out,
        parse(&["tool", "planets", "show", "--help"]).out
    );
    assert_eq!(parsed.err, "");
}

#[test]
fn help_without_words_prints_the_root_s_long_help_when_there_are_no_topics() {
    let parsed = parse(&["tool", "help"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    assert_eq!(parsed.out, parse(&["tool", "--help"]).out);
    assert_eq!(parsed.err, "");
}

#[test]
fn help_inside_a_group_prints_the_long_help_of_the_command_after_it() {
    let parsed = parse(&["tool", "planets", "help", "show"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    assert_eq!(
        parsed.out,
        parse(&["tool", "planets", "show", "--help"]).out
    );
    assert_eq!(parsed.err, "");
}

#[test]
fn help_as_the_last_word_after_a_group_prints_that_group_s_long_help() {
    let parsed = parse(&["tool", "planets", "help"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    assert_eq!(parsed.out, parse(&["tool", "planets", "--help"]).out);
    assert_eq!(parsed.err, "");
}

#[test]
fn help_with_an_unknown_word_is_refused_with_the_root_s_short_help_as_the_recovery() {
    let parsed = parse(&["tool", "help", "plants"]);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.out, "");
    let expected = "\
error: unrecognized subcommand 'plants'; a similar subcommand exists: 'planets'
  help:  run `tool -h`
  code:  crumbkit::cli::usage
";
    assert_eq!(parsed.err, expected);
}

/// A word as a parsed type, for a tool whose root takes one positional
/// argument.
#[derive(clap::Args)]
struct Word {
    word: String,
}

#[test]
fn a_root_without_subcommands_reads_help_as_an_ordinary_word() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<Word, _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        Command::new("tool").arg(Arg::new("word").required(true)),
        &Guide::new(),
        Spelling::COLONS,
        ["tool", "help"],
    );

    let Ok((parsed, _)) = result else {
        panic!("the command line did not parse");
    };
    assert_eq!(parsed.word, "help");
}

#[test]
fn the_version_prints_on_standard_output_and_exits_with_code_0() {
    let parsed = parse(&["tool", "--version"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    assert_eq!(parsed.out, "tool 1.2.3\n");
    assert_eq!(parsed.err, "");
}

/// A writer whose every write fails with the given kind of error.
struct Failing(io::ErrorKind);

impl Write for Failing {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(self.0.into())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(self.0.into())
    }
}

#[test]
fn help_exits_with_code_0_when_the_reader_closed_the_pipe() {
    let mut err = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(Failing(io::ErrorKind::BrokenPipe), &mut err),
        tool(),
        &Guide::new(),
        Spelling::COLONS,
        ["tool", "-h"],
    );

    assert_eq!(result.err(), Some(ExitCode::SUCCESS));
    assert_eq!(err, b"");
}

#[test]
fn a_message_without_clap_error_context_is_its_one_line() {
    assert_eq!(
        message("error: unexpected argument found\n"),
        "unexpected argument found"
    );
}

#[test]
fn a_message_joins_the_lines_of_a_paragraph_and_leaves_out_the_usage() {
    let rendered = "\
error: the following required arguments were not provided:
  <NAME>...

Usage: tool planets show <NAME>...

For more information, try '--help'.
";

    assert_eq!(
        message(rendered),
        "the following required arguments were not provided: <NAME>..."
    );
}
