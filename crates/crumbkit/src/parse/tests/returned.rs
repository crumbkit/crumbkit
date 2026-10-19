//! Tests for what `parse` returns for a command line that parses: the
//! tool's type and an `Output` that holds the view flags, the command and
//! the words.

use clap::{Arg, ArgAction, Command};

use super::{Flags, tool};
use crate::Sink;
use crate::help::Guide;
use crate::parse::parse_to;
use crate::view::{Table, View};

#[test]
fn a_command_line_that_parses_returns_the_parsed_type_and_an_output_that_holds_the_command() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<Flags, _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        tool(),
        &Guide::new(),
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
    let result = parse_to::<Viewed, _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        <Viewed as clap::CommandFactory>::command(),
        &Guide::new(),
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
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        Command::new("tool").arg(Arg::new("json").long("json").action(ArgAction::SetTrue)),
        &Guide::new(),
        ["tool", "--json"],
    );

    let Ok(((), output)) = result else {
        panic!("the command line did not parse");
    };
    assert!(!output.is_json());
    assert_eq!(output.view(), &View::Text(Table::Compact));
}

#[test]
fn the_output_of_a_stopped_run_has_the_guide_s_output_budget() {
    const GUIDE: Guide = Guide::new().output_budget(1024, "the test sets it");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<Flags, _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        tool(),
        &GUIDE,
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
    let command = <Removing as clap::CommandFactory>::command();
    let (_, output) = parse_to::<Removing, _, _, _, _>(&mut sink, command, &Guide::new(), args)
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
