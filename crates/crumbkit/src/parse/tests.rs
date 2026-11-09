//! Tests for `parse_to`: what it returns, and what it prints for help, the
//! version and a refused command line.

use std::ffi::OsString;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};

use super::message::message;
use super::{Stop, Unparsed, try_parse_to};
use crate::twins::assert_one_word_apart;
use crate::{Run, Sink, Tool};

mod aliases;
mod alternatives;
mod dashed;
mod fields;
mod help_flag;
mod help_word;
mod hidden;
mod hint;
mod json;
mod limit;
mod long_note;
mod missing;
mod names;
mod no_result;
mod only_problems;
mod returned;
mod roles;
mod root;
mod styled;
mod usage;
mod views;

/// Parses `args` against `command` as `parse_to` does with `C`'s command.
/// Most of the crate's tests build their tree by hand, so `command` differs
/// from what `C` derives.
pub(crate) fn parse_tree_to<C: clap::FromArgMatches>(
    sink: &mut Sink<impl Write, impl Write, impl BufRead>,
    command: Command,
    tool: &Tool,
    args: impl IntoIterator<Item = impl Into<OsString>>,
) -> Result<(C, Run), Stop> {
    try_parse_to(sink, command, tool, args).map_err(|unparsed| unparsed.print_to(sink))
}

/// A tool with two global flags, a version, and a group that holds a visible
/// leaf with an alias and a hidden leaf.
fn tree() -> Command {
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

/// What `parse_to` printed, and the exit code it returned and whether its
/// output asked for JSON when it did not return a parsed type.
struct Parsed {
    code: Option<ExitCode>,
    json: Option<bool>,
    out: String,
    err: String,
}

/// Runs `parse_to` over `tree()` with `args` into memory, for a command line
/// that does not parse.
fn parse(args: &[&str]) -> Parsed {
    parse_tree(tree(), args)
}

/// Runs `try_parse_to` over `command` with `args` into memory, for a
/// command line that does not parse, and prints a refusal with
/// `Refused::print_to`, so it prints what `parse_to` prints. It checks that
/// nothing printed before the refusal printed.
fn parse_tree(command: Command, args: &[&str]) -> Parsed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = try_parse_to::<()>(
        &mut Sink::new(&mut out, &mut err),
        command,
        &Tool::new(),
        args,
    );
    let stop = result.err().map(|unparsed| match unparsed {
        Unparsed::Refused(refused) => {
            assert_eq!((&out, &err), (&Vec::new(), &Vec::new()));
            refused.print_to(&mut Sink::new(&mut out, &mut err))
        }
        Unparsed::Stopped(stop) => stop,
    });
    Parsed {
        code: stop.as_ref().map(Stop::exit_code),
        json: stop.as_ref().map(|stop| stop.run().is_json()),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Checks that `admitted`, the twin of the refused `refused`, differs from
/// it by one word and that `parse_to` over `command` accepts it or prints
/// help with code 0 (TEST-1).
#[track_caller]
fn admits(command: Command, refused: &[&str], admitted: &[&str]) {
    assert_one_word_apart(refused, admitted);
    let parsed = parse_tree(command, admitted);
    assert!(
        parsed.code.is_none_or(|code| code == ExitCode::SUCCESS),
        "the admitted words {admitted:?} are refused:\n{}",
        parsed.err
    );
}

#[test]
fn a_refused_command_line_prints_the_usage_failure_on_standard_error_and_exits_with_code_2() {
    let refused = ["tool", "planets", "show", "Mars", "--srot", "name"];
    admits(
        tree(),
        &refused,
        &["tool", "planets", "show", "Mars", "--sort", "name"],
    );
    let parsed = parse(&refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.out, "");
    let expected = "\
error: unexpected argument '--srot' found; a similar argument exists: '--sort'
  usage: tool planets show --sort <KEY> <NAME>...
  help:  run `tool planets show -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
}

#[test]
fn a_refused_command_line_with_json_prints_the_failure_document_and_its_stop_asks_for_json() {
    let refused = [
        "tool", "planets", "show", "Mars", "--srot", "name", "--json",
    ];
    admits(
        tree(),
        &refused,
        &[
            "tool", "planets", "show", "Mars", "--sort", "name", "--json",
        ],
    );
    let parsed = parse(&refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.json, Some(true));
    let expected = concat!(
        r#"{"schema":"tool/failure/v1","code":"tool::cli::usage","#,
        r#""message":"unexpected argument '--srot' found; a similar argument exists: '--sort'","#,
        r#""fields":{"argument":"--srot"},"#,
        r#""recovery":{"kind":"run","call":"tool planets show -h"}}"#,
        "\n",
    );
    assert_eq!(parsed.out, expected);
    assert_eq!(parsed.err, "");
}

#[test]
fn a_value_of_an_option_that_allows_hyphen_values_is_no_request_for_json() {
    let pattern = Arg::new("pattern")
        .long("pattern")
        .allow_hyphen_values(true)
        .global(true);
    let refused = [
        "tool",
        "--pattern",
        "--json",
        "planets",
        "show",
        "Mars",
        "--srot",
    ];
    admits(tree().arg(pattern.clone()), &refused, &refused[..6]);
    let parsed = parse_tree(tree().arg(pattern), &refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.json, Some(false));
    assert_eq!(parsed.out, "");
    assert!(
        parsed
            .err
            .starts_with("error: unexpected argument '--srot'")
    );
}

#[test]
fn the_recovery_names_a_command_typed_by_its_alias_by_its_real_name() {
    let refused = ["tool", "planets", "get", "--bogus"];
    admits(tree(), &refused, &["tool", "planets", "get", "Mars"]);
    let parsed = parse(&refused);

    assert!(parsed.err.contains("  help:  run `tool planets show -h`\n"));
}

#[test]
fn the_recovery_names_a_hidden_command_that_the_caller_typed() {
    let refused = ["tool", "planets", "weigh", "--bogus"];
    admits(tree(), &refused, &refused[..3]);
    let parsed = parse(&refused);

    assert!(
        parsed
            .err
            .contains("  help:  run `tool planets weigh -h`\n")
    );
}

#[test]
fn the_walk_stops_at_a_double_dash() {
    let refused = ["tool", "planets", "--", "show", "Mars"];
    admits(tree(), &refused, &["tool", "planets", "show", "Mars"]);
    let parsed = parse(&refused);

    assert!(parsed.err.contains("  help:  run `tool planets -h`\n"));
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

/// `tree()` with a global flag that the `planets` group defines.
fn tool_with_a_group_global() -> Command {
    tree().mut_subcommand("planets", |planets| {
        planets.arg(
            Arg::new("units")
                .long("units")
                .value_name("UNIT")
                .global(true)
                .help("Print masses in this unit"),
        )
    })
}

#[test]
fn a_group_s_global_flag_is_named_on_the_line_of_the_group_and_each_command_below_it() {
    let root = parse_tree(tool_with_a_group_global(), &["tool", "-h"]).out;
    let group = parse_tree(tool_with_a_group_global(), &["tool", "planets", "-h"]).out;
    let leaf = parse_tree(
        tool_with_a_group_global(),
        &["tool", "planets", "show", "-h"],
    )
    .out;

    assert!(
        root.contains("\nGlobal flags: --json, --fields <FIELD>. `tool --help`"),
        "{root}"
    );
    assert!(
        group.contains(
            "\nGlobal flags: --json, --fields <FIELD>, --units <UNIT>. `tool planets --help`"
        ),
        "{group}"
    );
    assert!(
        leaf.contains(
            "\nGlobal flags: --json, --fields <FIELD>, --units <UNIT>. `tool planets show --help`"
        ),
        "{leaf}"
    );
    assert!(!group.contains("  --units"), "{group}");
    assert!(!leaf.contains("  --units"), "{leaf}");
}

#[test]
fn a_group_s_global_flag_stays_in_the_long_help_of_each_command_below_it() {
    let long = parse_tree(
        tool_with_a_group_global(),
        &["tool", "planets", "show", "--help"],
    )
    .out;

    assert!(long.contains("\n      --units <UNIT>\n"), "{long}");
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
fn a_tool_s_own_after_help_prints_before_crumbkit_s_lines_in_short_and_long_help() {
    let tool = tree().after_help("Report bugs at https://example.org/bugs.");

    let short = parse_tree(tool.clone(), &["tool", "-h"]).out;
    let long = parse_tree(tool, &["tool", "--help"]).out;

    assert!(
        short.ends_with(
            "\
Report bugs at https://example.org/bugs.

Global flags: --json, --fields <FIELD>. `tool --help` describes them.
Topics: exit-codes. `tool help <TOPIC>` prints one.
Next: tool <COMMAND> -h
"
        ),
        "{short}"
    );
    assert!(
        long.ends_with(
            "\
Report bugs at https://example.org/bugs.

Topics: exit-codes. `tool help <TOPIC>` prints one.
Next: tool <COMMAND> -h
"
        ),
        "{long}"
    );
}

#[test]
fn a_tool_s_own_after_long_help_prints_before_crumbkit_s_lines_in_long_help_only() {
    let tool = tree()
        .after_help("Report bugs.")
        .after_long_help("Report bugs at https://example.org/bugs.");

    let short = parse_tree(tool.clone(), &["tool", "-h"]).out;
    let long = parse_tree(tool, &["tool", "--help"]).out;

    assert!(short.contains("\nReport bugs.\n\nGlobal flags:"), "{short}");
    assert!(
        long.ends_with(
            "\nReport bugs at https://example.org/bugs.\n\nTopics: exit-codes. `tool help <TOPIC>` prints one.\nNext: tool <COMMAND> -h\n"
        ),
        "{long}"
    );
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
    let result = parse_tree_to::<()>(
        &mut Sink::new(Failing(io::ErrorKind::BrokenPipe), &mut err),
        tree(),
        &Tool::new(),
        ["tool", "-h"],
    );

    assert_eq!(result.err().map(ExitCode::from), Some(ExitCode::SUCCESS));
    assert_eq!(err, b"");
}

#[test]
fn a_page_that_cannot_be_written_stops_with_exit_1() {
    let mut err = Vec::new();
    let result = parse_tree_to::<()>(
        &mut Sink::new(Failing(io::ErrorKind::Other), &mut err),
        tree(),
        &Tool::new(),
        ["tool", "-h"],
    );

    assert_eq!(result.err().map(|stop| stop.exit()), Some(1));
    assert_eq!(
        String::from_utf8(err).unwrap(),
        "error: cannot write to standard output: other error\n"
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
