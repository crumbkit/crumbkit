//! Tests for help topics: what `parse_to` prints for `help` and for help
//! pages with a tool record that has topics.

use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};

use super::Topic;
use crate::help::{Exit, Help, exit};
use crate::tool::Failures;
use crate::twins::assert_one_word_apart;
use crate::{Sink, Tool, parse_to};

/// A tool with a global flag and one group.
fn tree() -> Command {
    Command::new("tool")
        .about("Explore a small solar system")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print JSON"),
        )
        .subcommand(
            Command::new("planets")
                .about("Work with planets")
                .subcommand_required(true)
                .subcommand(Command::new("list").about("List the planets")),
        )
}

/// A topic whose text ends with a line break.
const ORBITS: Topic = Topic::new(
    "orbits",
    "How the tool counts orbits",
    "An orbit is one trip around the Sun.\nThe tool counts orbits in Earth days.\n",
);

/// A topic whose text has no final line break and whose summary is empty.
const UNITS: Topic = Topic::new("units", "", "Masses are in Earth masses.");

/// A tool record with both topics.
const TOOL: Tool = Tool::new().help(Help::new().topics(&[ORBITS, UNITS]));

/// What `parse_to` printed, and the exit code it returned.
struct Printed {
    code: Option<ExitCode>,
    out: String,
    err: String,
}

/// Runs `parse_to` over `command` with `tool`, for `args` that do not
/// parse, and returns what it printed.
fn run(command: Command, tool: &Tool, args: &[&str]) -> Printed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result =
        parse_to::<(), _, _, _, _>(&mut Sink::new(&mut out, &mut err), command, tool, args);
    Printed {
        code: result.err().map(ExitCode::from),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Runs `parse_to` over `tree()` with `tool`, for `args` that print a page,
/// and returns the page.
fn page(tool: &Tool, args: &[&str]) -> String {
    let printed = run(tree(), tool, args);
    assert_eq!(printed.code, Some(ExitCode::SUCCESS));
    assert_eq!(printed.err, "");
    printed.out
}

#[test]
fn help_with_a_topic_s_name_prints_its_text() {
    assert_eq!(page(&TOOL, &["tool", "help", "orbits"]), ORBITS.text());
}

#[test]
fn a_topic_s_text_ends_with_one_line_break_when_the_tool_leaves_it_out() {
    assert_eq!(
        page(&TOOL, &["tool", "help", "units"]),
        "Masses are in Earth masses.\n"
    );
}

#[test]
fn help_without_words_prints_what_the_root_s_long_help_prints() {
    let long = page(&TOOL, &["tool", "--help"]);

    assert_eq!(page(&TOOL, &["tool", "help"]), long);
    assert!(
        long.contains("\nTopics: orbits, units, exit-codes. `tool help <TOPIC>` prints one.\n"),
        "{long}"
    );
}

#[test]
fn the_root_s_short_and_long_help_name_the_topics_before_the_next_line() {
    let short = "\
Explore a small solar system

Usage: tool [OPTIONS] <COMMAND>

Commands:
  planets  Work with planets

Options:
  -h, --help  Print help (see more with '--help')

Global flags: --json. `tool --help` describes them.
Topics: orbits, units, exit-codes. `tool help <TOPIC>` prints one.
Next: tool <COMMAND> -h
";
    let long = page(&TOOL, &["tool", "--help"]);

    assert_eq!(page(&TOOL, &["tool", "-h"]), short);
    assert!(
        long.ends_with(
            "\n\nTopics: orbits, units, exit-codes. `tool help <TOPIC>` prints one.\nNext: tool <COMMAND> -h\n"
        ),
        "{long}"
    );
}

#[test]
fn a_group_s_help_does_not_name_the_topics() {
    let group = page(&TOOL, &["tool", "planets", "-h"]);

    assert!(!group.contains("Topics:"), "{group}");
}

#[test]
fn a_command_wins_over_a_topic_with_the_same_name() {
    const TOOL: Tool =
        Tool::new().help(Help::new().topics(&[Topic::new("planets", "", "Planets.\n")]));

    assert_eq!(
        page(&TOOL, &["tool", "help", "planets"]),
        page(&TOOL, &["tool", "planets", "--help"])
    );
}

#[test]
fn help_ignores_flags_after_a_topic_s_name() {
    assert_eq!(
        page(&TOOL, &["tool", "help", "orbits", "--json"]),
        ORBITS.text()
    );
}

#[test]
fn a_repeated_topic_name_is_named_once_and_the_first_topic_prints() {
    const TOOL: Tool = Tool::new().help(Help::new().topics(&[
        ORBITS,
        Topic::new("orbits", "A second summary", "A second text.\n"),
    ]));

    let short = page(&TOOL, &["tool", "-h"]);

    assert!(
        short.contains("\nTopics: orbits, exit-codes. `tool help <TOPIC>` prints one.\n"),
        "{short}"
    );
    assert_eq!(page(&TOOL, &["tool", "help", "orbits"]), ORBITS.text());
}

#[test]
fn a_root_with_its_own_help_command_gets_no_topics_and_clap_parses_that_command() {
    let tool = tree().subcommand(Command::new("assist").alias("help"));

    assert_one_word_apart(&["tool", "help", "orbits"], &["tool", "help"]);
    assert_eq!(run(tool.clone(), &TOOL, &["tool", "help"]).code, None);
    let printed = run(tool, &TOOL, &["tool", "help", "orbits"]);

    assert_eq!(printed.code, Some(ExitCode::from(2)));
    assert_eq!(
        printed.err,
        "error: unexpected argument 'orbits' found\n  usage: tool assist [OPTIONS]\n  help:  run `tool assist -h`\n  code:  tool::cli::usage\n"
    );
}

#[test]
fn a_root_with_its_own_help_command_names_no_topics_in_its_help() {
    let tool = tree().subcommand(Command::new("assist").alias("help"));

    let short = run(tool, &TOOL, &["tool", "-h"]).out;

    assert!(!short.contains("Topics:"), "{short}");
}

#[test]
fn a_tool_without_subcommands_and_with_topics_gets_no_next_line() {
    let tool = Command::new("tool").about("Weigh a planet");

    let expected = "\
Weigh a planet

Usage: tool

Options:
  -h, --help  Print help

Topics: orbits, units, exit-codes. `tool help <TOPIC>` prints one.
";
    assert_eq!(run(tool.clone(), &TOOL, &["tool", "-h"]).out, expected);
    assert_eq!(
        run(tool, &TOOL, &["tool", "help", "orbits"]).out,
        ORBITS.text()
    );
}

#[test]
fn help_without_words_prints_the_long_help_of_a_root_without_subcommands() {
    let tool = Command::new("tool").about("Weigh a planet");

    assert_eq!(
        run(tool.clone(), &TOOL, &["tool", "help"]).out,
        run(tool, &TOOL, &["tool", "--help"]).out
    );
}

#[test]
fn help_exit_codes_prints_crumbkit_s_codes_and_the_tool_s_codes() {
    const TOOL: Tool = Tool::new()
        .failures(Failures::new().exits(&[Exit::new(124, "The telescope ran out of time.")]));

    assert_eq!(
        page(&TOOL, &["tool", "help", "exit-codes"]),
        exit::text("tool", &TOOL)
    );
    assert!(page(&TOOL, &["tool", "help", "exit-codes"]).contains("  124  The telescope"));
}

#[test]
fn a_tool_without_topics_names_the_exit_codes_topic_in_its_help() {
    let short = page(&Tool::new(), &["tool", "-h"]);

    assert_eq!(
        page(&Tool::new(), &["tool", "help", "exit-codes"]),
        exit::text("tool", &Tool::new())
    );
    assert!(
        short.contains("\nTopics: exit-codes. `tool help <TOPIC>` prints one.\n"),
        "{short}"
    );
}

#[test]
fn crumbkit_s_exit_codes_topic_prints_in_place_of_a_tool_topic_with_its_name() {
    const TOOL: Tool =
        Tool::new().help(Help::new().topics(&[Topic::new("exit-codes", "Mine", "Mine.\n")]));

    let short = page(&TOOL, &["tool", "-h"]);

    assert!(
        short.contains("\nTopics: exit-codes. `tool help <TOPIC>` prints one.\n"),
        "{short}"
    );
    assert_eq!(
        page(&TOOL, &["tool", "help", "exit-codes"]),
        exit::text("tool", &TOOL)
    );
}

#[test]
fn a_root_without_subcommands_or_topics_names_no_topics_in_its_help() {
    let tool = Command::new("tool").about("Weigh a planet");

    let short = run(tool, &Tool::new(), &["tool", "-h"]).out;

    assert!(!short.contains("Topics:"), "{short}");
}
