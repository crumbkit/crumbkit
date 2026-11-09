//! Tests for what `parse` returns: for a command line that parses, the
//! tool's type and an `Output` that holds the view flags, the command and
//! the words, and for a run that it ended, a `Stop` that says what printed.

use crate::parse::tests::parse_tree_to;
use clap::{Arg, ArgAction, Command};

use super::{Flags, admits, tree};
use crate::view::{Table, View};
use crate::{Printed, Sink, Tool};

#[test]
fn a_command_line_that_parses_returns_the_parsed_type_and_an_output_that_holds_the_command() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_tree_to::<Flags>(
        &mut Sink::new(&mut out, &mut err),
        tree(),
        &Tool::new(),
        ["tool", "--json", "planets", "show", "Mars"],
    );

    let Ok((flags, mut output)) = result else {
        panic!("the command line did not parse");
    };
    assert!(flags.json);
    assert!(output.is_json());
    let words = ["--json", "planets", "show", "Mars"].map(std::ffi::OsString::from);
    assert_eq!(output.words(), words);
    assert_eq!(output.command().get_name(), "tool");
    assert_eq!(out, b"");
    assert_eq!(err, b"");
}

#[test]
fn parse_reads_the_view_flags_that_a_tool_flattens_before_the_tool_s_type_takes_them() {
    /// A tool that flattens crumbkit's view flags.
    #[derive(clap::Parser)]
    #[command(name = "tool")]
    struct Viewed {
        #[command(subcommand)]
        command: Listed,
        #[command(flatten)]
        view: crate::ViewFlags,
    }
    #[derive(clap::Subcommand)]
    enum Listed {
        List,
    }
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = crate::parse_to::<Viewed>(
        &mut Sink::new(&mut out, &mut err),
        &Tool::new(),
        ["tool", "list", "--json", "--fields", "name,moons"],
    );

    let Ok((viewed, output)) = result else {
        panic!(
            "the command line did not parse: {}",
            String::from_utf8_lossy(&err)
        );
    };
    assert!(matches!(viewed.command, Listed::List));
    assert!(output.is_json());
    assert_eq!(output.field_list(), ["name", "moons"]);
}

#[test]
fn a_tool_without_view_flags_parses_with_every_view_flag_off() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_tree_to::<()>(
        &mut Sink::new(&mut out, &mut err),
        Command::new("tool").arg(Arg::new("json").long("json").action(ArgAction::SetTrue)),
        &Tool::new(),
        ["tool", "--json"],
    );

    let Ok(((), output)) = result else {
        panic!("the command line did not parse");
    };
    assert!(!output.is_json());
    assert_eq!(output.view(), &View::Text(Table::Compact));
}

#[test]
fn the_output_of_a_stopped_run_has_the_tool_s_output_budget() {
    const TOOL: Tool =
        Tool::new().output(crate::tool::Output::new().budget(1024, "the test sets it"));
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_tree_to::<Flags>(
        &mut Sink::new(&mut out, &mut err),
        tree(),
        &TOOL,
        ["tool", "--help"],
    );

    let Err(stop) = result else {
        panic!("help did not stop the run");
    };
    assert_eq!(stop.output().budget(), 1024);
}

/// A tool whose `remove` command takes `--dry-run` and `--yes`.
#[derive(clap::Parser)]
#[command(name = "tool")]
struct Removing {
    #[command(subcommand)]
    command: Removes,
}

#[derive(clap::Subcommand)]
enum Removes {
    Remove {
        #[command(flatten)]
        dry_run: crate::DryRun,
        #[command(flatten)]
        yes: crate::Yes,
    },
}

/// Parses `args` as the removing tool, and returns its `Output`.
fn removing(args: &[&str]) -> crate::Output {
    let mut sink = Sink::new(Vec::new(), Vec::new());
    let (_, output) = crate::parse_to::<Removing>(&mut sink, &Tool::new(), args)
        .unwrap_or_else(|_| panic!("the command line did not parse"));
    output
}

#[test]
fn parse_records_dry_run_on_the_deepest_command() {
    assert!(removing(&["tool", "remove", "--dry-run"]).is_dry_run());
    assert!(!removing(&["tool", "remove"]).is_dry_run());
}

#[test]
fn a_dry_run_confirms_without_asking() {
    let mut output = removing(&["tool", "remove", "--dry-run"]);
    let mut err = Vec::new();
    let mut sink = Sink::with_input(Vec::new(), &mut err, &b""[..], false);

    let confirmed = output.confirm_to(&mut sink, crate::Yes::default(), "Remove Pluto?");

    assert!(confirmed.is_ok());
    assert_eq!(err, b"");
}

/// Runs `parse_to` over `command` with `args` into memory, and returns the
/// `Printed` of the `Stop` that ended the run. A run that parses fails the
/// test.
#[track_caller]
fn printed_by(command: Command, args: &[&str]) -> Printed {
    let mut sink = Sink::new(Vec::new(), Vec::new());
    match parse_tree_to::<()>(&mut sink, command, &Tool::new(), args) {
        Err(stop) => stop.printed(),
        Ok(_) => panic!("{args:?} parsed"),
    }
}

#[test]
fn short_help_long_help_and_the_word_help_report_that_help_printed() {
    assert_eq!(printed_by(tree(), &["tool", "-h"]), Printed::Help);
    assert_eq!(
        printed_by(tree(), &["tool", "planets", "show", "--help"]),
        Printed::Help
    );
    assert_eq!(
        printed_by(tree(), &["tool", "help", "planets"]),
        Printed::Help
    );
}

#[test]
fn the_version_reports_that_the_version_printed() {
    assert_eq!(printed_by(tree(), &["tool", "--version"]), Printed::Version);
}

#[test]
fn a_topic_at_the_root_reports_that_a_topic_printed() {
    assert_eq!(
        printed_by(tree(), &["tool", "help", "exit-codes"]),
        Printed::Topic
    );
}

#[test]
fn a_printed_refusal_reports_that_a_refusal_printed() {
    let refused = ["tool", "planets", "show", "Mars", "--srot", "name"];
    admits(
        tree(),
        &refused,
        &["tool", "planets", "show", "Mars", "--sort", "name"],
    );

    assert_eq!(printed_by(tree(), &refused), Printed::Refusal);
}

#[test]
fn a_refused_view_flag_reports_that_a_refusal_printed() {
    /// A tool whose root flattens the view flags.
    #[derive(clap::Parser)]
    #[command(name = "tool")]
    struct Viewed {
        #[command(flatten)]
        view: crate::ViewFlags,
    }
    let command = <Viewed as clap::CommandFactory>::command();
    admits(
        command.clone(),
        &["tool", "--full"],
        &["tool", "--full", "--json"],
    );

    assert_eq!(printed_by(command, &["tool", "--full"]), Printed::Refusal);
}

#[test]
fn the_handle_reports_whether_the_caller_passed_no_input() {
    use clap::{CommandFactory as _, Parser};

    #[derive(Parser)]
    #[command(name = "tool")]
    struct Cli {
        #[command(flatten)]
        no_input: crate::NoInput,
    }
    let no_input = |args: &[&str]| {
        let mut sink = Sink::new(Vec::new(), Vec::new());
        crate::parse_to::<Cli>(&mut sink, &Tool::new(), args)
            .ok()
            .unwrap()
            .1
            .no_input()
    };

    assert!(no_input(&["tool", "--no-input"]));
    assert!(!no_input(&["tool"]));
}
