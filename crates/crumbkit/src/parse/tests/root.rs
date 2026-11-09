//! Tests for a tool record whose root is a subcommand, as for a tool that
//! another program runs with a fixed first word, such as a cargo subcommand.

use crate::parse::parse_tree_to;
use std::process::ExitCode;

use super::{Parsed, tree};
use crate::help::Help;
use crate::parse::Stop;
use crate::twins::assert_one_word_apart;
use crate::{Sink, Tool};

/// A tool record that roots the tool at its `planets` group, as cargo roots a
/// cargo subcommand at the word after `cargo`.
const TOOL: Tool = Tool::new().help(Help::new().root(&["planets"]));

/// Runs `parse_to` over `tree()` with `TOOL` and `args` into memory, for a
/// command line that does not parse.
fn parse(args: &[&str]) -> Parsed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_tree_to::<()>(&mut Sink::new(&mut out, &mut err), tree(), &TOOL, args);
    let stop = result.err();
    Parsed {
        code: stop.as_ref().map(Stop::exit_code),
        json: stop.as_ref().map(|stop| stop.output().is_json()),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

#[test]
fn help_after_the_tool_s_root_prints_a_topic() {
    let parsed = parse(&["tool", "planets", "help", "exit-codes"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    assert!(parsed.out.starts_with("Exit codes\n"), "{}", parsed.out);
    assert_eq!(parsed.err, "");
}

#[test]
fn help_after_the_tool_s_root_prints_the_root_s_long_help() {
    let parsed = parse(&["tool", "planets", "help"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    assert_eq!(parsed.out, parse(&["tool", "planets", "--help"]).out);
    assert!(
        parsed
            .out
            .contains("\nTopics: exit-codes. `tool planets help <TOPIC>` prints one.\n"),
        "{}",
        parsed.out
    );
}

#[test]
fn help_after_the_tool_s_root_with_a_command_prints_that_command_s_long_help() {
    let parsed = parse(&["tool", "planets", "help", "show"]);

    assert_eq!(
        parsed.out,
        parse(&["tool", "planets", "show", "--help"]).out
    );
}

#[test]
fn the_topics_line_sits_on_the_help_of_the_tool_s_root_and_names_its_command_line() {
    let root = parse(&["tool", "planets", "-h"]).out;
    let above = parse(&["tool", "-h"]).out;

    assert!(
        root.contains("\nTopics: exit-codes. `tool planets help <TOPIC>` prints one.\n"),
        "{root}"
    );
    assert!(!above.contains("Topics:"), "{above}");
}

#[test]
fn help_above_the_tool_s_root_prints_no_topic() {
    let refused = ["tool", "help", "exit-codes"];
    let admitted = ["tool", "planets", "help", "exit-codes"];
    assert_one_word_apart(&refused, &admitted);
    assert_eq!(parse(&admitted).code, Some(ExitCode::SUCCESS));
    let parsed = parse(&refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.out, "");
}
