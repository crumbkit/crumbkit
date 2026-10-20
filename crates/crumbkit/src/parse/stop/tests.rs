//! Tests for what a `Stop` reports of the walk of the words: the path to
//! the deepest command, the flags typed and an option's value, after help,
//! the version, a topic and a refused view flag.

use std::ffi::OsStr;

use clap::{Arg, Command};

use super::Stop;
use crate::help::Guide;
use crate::parse::{Unparsed, try_parse_to};
use crate::twins::assert_one_word_apart;
use crate::{DryRun, Output, Sink, ViewFlags};

/// A tool with a version, a global `--format` option and a `planets` group
/// whose `show` takes names and a `--sort` key.
fn tool() -> Command {
    Command::new("tool")
        .version("1.2.3")
        .arg(Arg::new("format").long("format").global(true))
        .subcommand(
            Command::new("planets").subcommand(
                Command::new("show")
                    .arg(Arg::new("names").required(true).num_args(1..))
                    .arg(Arg::new("sort").long("sort")),
            ),
        )
}

/// Runs `try_parse_to` over `tool()` with `args` into memory and returns
/// the `Stop` of the run. A run that parses or is refused fails the test.
#[track_caller]
fn stopped(args: &[&str]) -> Stop {
    let mut sink = Sink::new(Vec::new(), Vec::new());
    match try_parse_to::<(), _, _, _, _>(&mut sink, tool(), &Guide::new(), args) {
        Err(Unparsed::Stopped(stop)) => stop,
        Err(Unparsed::Refused(refused)) => unreachable!("{args:?} was refused: {refused:?}"),
        Ok(_) => unreachable!("{args:?} parsed"),
    }
}

#[test]
fn a_help_page_below_a_group_reports_the_path_to_that_group() {
    let stop = stopped(&["tool", "planets", "--help"]);

    assert_eq!(stop.path(), ["planets"]);
}

#[test]
fn the_word_help_before_a_command_reports_the_path_to_that_command() {
    let stop = stopped(&["tool", "help", "planets", "show"]);

    assert_eq!(stop.path(), ["planets", "show"]);
}

#[test]
fn the_version_reports_an_empty_path() {
    let stop = stopped(&["tool", "--version"]);

    assert_eq!(stop.path(), Vec::<String>::new());
}

#[test]
fn a_help_page_reports_the_help_id_and_the_options_typed_as_its_flags() {
    let stop = stopped(&["tool", "--format", "json", "planets", "show", "--help"]);

    assert_eq!(stop.flags(), ["format", "help"]);
}

#[test]
fn a_help_page_reports_a_global_option_s_value_typed_before_the_subcommand() {
    let stop = stopped(&["tool", "--format", "json", "planets", "show", "--help"]);

    assert_eq!(stop.value("format"), Some(OsStr::new("json")));
}

#[test]
fn a_help_page_reports_a_global_option_s_value_typed_inline_after_the_subcommand() {
    let stop = stopped(&["tool", "planets", "show", "--format=json", "--help"]);

    assert_eq!(stop.value("format"), Some(OsStr::new("json")));
}

#[test]
fn a_help_page_reports_no_value_for_an_option_that_the_words_leave_out() {
    let stop = stopped(&["tool", "planets", "show", "--sort", "name", "--help"]);

    assert_eq!(stop.value("format"), None);
    assert_eq!(stop.value("sort"), Some(OsStr::new("name")));
}

#[test]
fn a_topic_page_reports_an_empty_path_and_reads_the_words_after_the_program_name() {
    let stop = stopped(&["tool", "--format", "json", "help", "exit-codes"]);

    assert_eq!(stop.printed(), crate::Printed::Topic);
    assert_eq!(stop.path(), Vec::<String>::new());
    assert_eq!(stop.flags(), ["format"]);
    assert_eq!(stop.value("format"), Some(OsStr::new("json")));
}

/// A tool with a global `--format` option whose `list` flattens the view
/// flags.
#[derive(clap::Parser)]
#[command(name = "tool")]
struct Viewed {
    #[arg(long, global = true)]
    format: Option<String>,
    #[command(subcommand)]
    command: Listed,
}

/// The commands of [`Viewed`].
#[derive(clap::Subcommand)]
enum Listed {
    List {
        #[command(flatten)]
        view: ViewFlags,
    },
}

/// Runs `try_parse_to` over [`Viewed`] with `args` into memory and returns
/// whether it parsed, or the `Refused` of the run printed to memory and
/// turned into its `Stop`.
fn viewed(args: &[&str]) -> Result<(), Stop> {
    let command = <Viewed as clap::CommandFactory>::command();
    let mut sink = Sink::new(Vec::new(), Vec::new());
    match try_parse_to::<(), _, _, _, _>(&mut sink, command, &Guide::new(), args) {
        Ok(_) => Ok(()),
        Err(unparsed) => Err(unparsed.print_to(&mut sink)),
    }
}

#[test]
fn the_stop_of_a_refused_view_flag_reports_the_path_and_an_option_s_value() {
    let refused = ["tool", "--format", "json", "list", "--full"];
    let admitted = ["tool", "--format", "json", "list", "--full", "--json"];
    assert_one_word_apart(&refused, &admitted);
    assert!(viewed(&admitted).is_ok(), "{admitted:?} was refused");

    let Err(stop) = viewed(&refused) else {
        panic!("{refused:?} parsed");
    };

    assert_eq!(stop.printed(), crate::Printed::Refusal);
    assert_eq!(stop.path(), ["list"]);
    assert_eq!(stop.flags(), ["format", "full"]);
    assert_eq!(stop.value("format"), Some(OsStr::new("json")));
}

/// Builds an `Output` over [`Viewed`] with the view flags `view` and
/// `args`, printing to memory.
fn built(view: ViewFlags, args: &[&str]) -> Result<Output, Stop> {
    let command = <Viewed as clap::CommandFactory>::command();
    let mut sink = Sink::new(Vec::new(), Vec::new());
    Output::new_to(
        &mut sink,
        view,
        DryRun::default(),
        command,
        &Guide::new(),
        args,
    )
}

#[test]
fn the_stop_of_output_new_s_refused_view_flags_reports_the_path_and_an_option_s_value() {
    let args = ["tool", "--format", "json", "list", "--full"];
    assert!(built(ViewFlags::new().full(true).json(true), &args).is_ok());

    let Err(stop) = built(ViewFlags::new().full(true), &args) else {
        panic!("--full without --json was accepted");
    };

    assert_eq!(stop.path(), ["list"]);
    assert_eq!(stop.value("format"), Some(OsStr::new("json")));
}
