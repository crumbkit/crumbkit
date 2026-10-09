//! Tests for help topics: what `parse_to` prints for `help` and for help
//! pages in a tree that `extend` has given topics.

use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};

use super::Topic;
use crate::help::{Guide, extend};
use crate::{Sink, Spelling, parse_to};

/// A tool with a global flag and one group.
fn tool() -> Command {
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

/// A guide with both topics.
const GUIDE: Guide = Guide::new().topics(&[ORBITS, UNITS]);

/// What `parse_to` printed, and the exit code it returned.
struct Printed {
    code: Option<ExitCode>,
    out: String,
    err: String,
}

/// Runs `parse_to` over `command` with `guide` applied, for `args` that do
/// not parse, and returns what it printed.
fn run(command: Command, guide: &Guide, args: &[&str]) -> Printed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        extend(command, guide),
        Spelling::COLONS,
        args,
    );
    Printed {
        code: result.err(),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Runs `parse_to` over `tool()` with `guide` applied, for `args` that print
/// a page, and returns the page.
fn page(guide: &Guide, args: &[&str]) -> String {
    let printed = run(tool(), guide, args);
    assert_eq!(printed.code, Some(ExitCode::SUCCESS));
    assert_eq!(printed.err, "");
    printed.out
}

#[test]
fn help_with_a_topic_s_name_prints_its_text() {
    assert_eq!(page(&GUIDE, &["tool", "help", "orbits"]), ORBITS.text());
}

#[test]
fn a_topic_s_text_ends_with_one_line_break_when_the_tool_leaves_it_out() {
    assert_eq!(
        page(&GUIDE, &["tool", "help", "units"]),
        "Masses are in Earth masses.\n"
    );
}

#[test]
fn help_without_words_lists_each_topic_with_its_summary_and_the_next_line() {
    let expected = "\
Topics
  orbits  How the tool counts orbits
  units

Next: tool help <TOPIC>
";
    assert_eq!(page(&GUIDE, &["tool", "help"]), expected);
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
Topics: orbits, units. `tool help <TOPIC>` prints one.
Next: tool <COMMAND> -h
";
    let long = page(&GUIDE, &["tool", "--help"]);

    assert_eq!(page(&GUIDE, &["tool", "-h"]), short);
    assert!(
        long.ends_with(
            "\n\nTopics: orbits, units. `tool help <TOPIC>` prints one.\nNext: tool <COMMAND> -h\n"
        ),
        "{long}"
    );
}

#[test]
fn a_group_s_help_does_not_name_the_topics() {
    let group = page(&GUIDE, &["tool", "planets", "-h"]);

    assert!(!group.contains("Topics:"), "{group}");
}

#[test]
fn a_command_wins_over_a_topic_with_the_same_name() {
    const GUIDE: Guide = Guide::new().topics(&[Topic::new("planets", "", "Planets.\n")]);

    assert_eq!(
        page(&GUIDE, &["tool", "help", "planets"]),
        page(&GUIDE, &["tool", "planets", "--help"])
    );
}

#[test]
fn help_ignores_flags_after_a_topic_s_name() {
    assert_eq!(
        page(&GUIDE, &["tool", "help", "orbits", "--json"]),
        ORBITS.text()
    );
}

#[test]
fn a_tree_with_topics_passes_clap_s_debug_assertions() {
    extend(tool(), &GUIDE).debug_assert();
}

#[test]
fn a_repeated_topic_name_is_left_out_and_the_first_topic_prints() {
    const GUIDE: Guide = Guide::new().topics(&[
        ORBITS,
        Topic::new("orbits", "A second summary", "A second text.\n"),
    ]);

    let expected = "\
Topics
  orbits  How the tool counts orbits

Next: tool help <TOPIC>
";
    assert_eq!(page(&GUIDE, &["tool", "help"]), expected);
    assert_eq!(page(&GUIDE, &["tool", "help", "orbits"]), ORBITS.text());
}

#[test]
fn a_root_with_its_own_help_command_gets_no_topics_and_clap_parses_that_command() {
    let tool = tool().subcommand(Command::new("assist").alias("help"));

    let printed = run(tool, &GUIDE, &["tool", "help", "orbits"]);

    assert_eq!(printed.code, Some(ExitCode::from(2)));
    assert_eq!(
        printed.err,
        "error: unexpected argument 'orbits' found\n  help:  run `tool assist -h`\n  code:  crumbkit::cli::usage\n"
    );
}

#[test]
fn a_tool_without_subcommands_and_with_topics_gets_no_next_line() {
    let tool = Command::new("tool").about("Weigh a planet");

    let expected = "\
Weigh a planet

Usage: tool

Options:
  -h, --help  Print help

Topics: orbits, units. `tool help <TOPIC>` prints one.
";
    assert_eq!(run(tool.clone(), &GUIDE, &["tool", "-h"]).out, expected);
    assert_eq!(
        run(tool, &GUIDE, &["tool", "help", "orbits"]).out,
        ORBITS.text()
    );
}
