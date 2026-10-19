//! Tests for `try_parse_to`: a refused command line comes back unprinted
//! with the words that the caller typed and parsed, and prints what
//! `parse_to` prints when the tool prints it.

use std::ffi::OsString;

use clap::{Arg, ArgAction, Command};

use super::{Refused, Unparsed};
use crate::codes::CLI_USAGE;
use crate::events::testing::watch;
use crate::help::{Exit, Guide};
use crate::parse::{parse_to, try_parse_to};
use crate::twins::assert_one_word_apart;
use crate::{Sink, ViewFlags};

/// A tool with a global `--json` flag and a `planets` group whose `show`
/// takes names and a `--sort` key, and a `moon_phases` command.
fn tool() -> Command {
    Command::new("tool")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("planets")
                .subcommand_required(true)
                .subcommand(
                    Command::new("show")
                        .arg(Arg::new("names").required(true).num_args(1..))
                        .arg(Arg::new("sort").long("sort")),
                ),
        )
        .subcommand(Command::new("moon_phases"))
}

/// Runs `try_parse_to` over `command` with `guide` and `args`, checks that
/// it printed nothing, and returns its refusal. A run that parses or stops
/// fails the test.
#[track_caller]
fn refused_by(command: Command, guide: &Guide, args: &[&str]) -> Refused {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let unparsed =
        try_parse_to::<(), _, _, _, _>(&mut Sink::new(&mut out, &mut err), command, guide, args);
    assert_eq!((out, err), (Vec::new(), Vec::new()));
    match unparsed {
        Err(Unparsed::Refused(refused)) => refused,
        Err(Unparsed::Stopped(stop)) => unreachable!("{args:?} stopped with {}", stop.number()),
        Ok(_) => unreachable!("{args:?} parsed"),
    }
}

/// Runs `try_parse_to` over `tool()` without a guide, as [`refused_by`].
#[track_caller]
fn refused(args: &[&str]) -> Refused {
    refused_by(tool(), &Guide::new(), args)
}

/// Returns `words` as the words of a command line.
fn words(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

/// Returns what `refused` prints, on standard output and standard error.
fn printed(refused: Refused) -> (String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let _ = refused.print_to(&mut Sink::new(&mut out, &mut err));
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// Returns what `parse_to` prints over `command` with `guide` for `args`.
fn parse_printed(command: Command, guide: &Guide, args: &[&str]) -> (String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let _ = parse_to::<(), _, _, _, _>(&mut Sink::new(&mut out, &mut err), command, guide, args);
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

#[test]
fn a_refused_command_line_comes_back_unprinted_with_its_typed_and_parsed_words_and_culprit() {
    let refused = refused(&["tool", "planets", "show", "Mars", "--srot", "name"]);

    assert_eq!(
        refused.typed(),
        words(&["planets", "show", "Mars", "--srot", "name"])
    );
    assert_eq!(refused.parsed(), refused.typed());
    assert_eq!(refused.culprit(), Some("--srot"));
    assert_eq!(
        refused.message(),
        "unexpected argument '--srot' found; a similar argument exists: '--sort'"
    );
    assert_eq!(refused.code(), &CLI_USAGE);
    assert_eq!(refused.number(), 2);
}

#[test]
fn help_with_a_word_that_names_no_command_reports_the_typed_help_and_the_culprit() {
    let refused = refused(&["tool", "help", "nonsense"]);

    assert_eq!(refused.typed(), words(&["help", "nonsense"]));
    assert_eq!(refused.parsed(), words(&["nonsense", "--help"]));
    assert_eq!(refused.culprit(), Some("nonsense"));
}

#[test]
fn an_unknown_subcommand_is_the_culprit() {
    assert_eq!(refused(&["tool", "comets"]).culprit(), Some("comets"));
}

#[test]
fn a_missing_required_argument_has_no_culprit() {
    assert_eq!(refused(&["tool", "planets", "show"]).culprit(), None);
}

#[test]
fn a_dashed_form_is_typed_as_the_caller_wrote_it_and_parsed_as_the_command_s_name() {
    const DASHED: Guide = Guide::new().dashed_forms();

    let refused = refused_by(tool(), &DASHED, &["tool", "moon-phases", "full"]);

    assert_eq!(refused.typed(), words(&["moon-phases", "full"]));
    assert_eq!(refused.parsed(), words(&["moon_phases", "full"]));
    assert_eq!(refused.culprit(), Some("full"));
}

#[test]
fn a_refused_view_flag_names_its_flag_as_the_culprit() {
    /// A tool whose root flattens the view flags.
    #[derive(clap::Parser)]
    #[command(name = "tool")]
    struct Viewed {
        #[command(flatten)]
        view: ViewFlags,
    }

    let command = <Viewed as clap::CommandFactory>::command();
    let refused = refused_by(command, &Guide::new(), &["tool", "--full"]);

    assert_eq!(refused.typed(), words(&["--full"]));
    assert_eq!(refused.culprit(), Some("--full"));
}

#[test]
fn printing_a_refusal_prints_what_parse_to_prints() {
    let args = [
        "tool", "planets", "show", "Mars", "--srot", "name", "--json",
    ];

    assert_eq!(
        printed(refused(&args)),
        parse_printed(tool(), &Guide::new(), &args)
    );
}

#[test]
fn printing_a_refusal_returns_a_stop_with_its_number_and_its_output() {
    const GUIDE: Guide = Guide::new()
        .exits(&[Exit::new(64, "The command line was refused.").failures(&[&CLI_USAGE])]);
    let refused = refused_by(tool(), &GUIDE, &["tool", "comets", "--json"]);
    assert_one_word_apart(
        &["tool", "comets", "--json"],
        &["tool", "planets", "--json"],
    );

    let number = refused.number();
    let json = refused.output().is_json();
    let stop = refused.print_to(&mut Sink::new(Vec::new(), Vec::new()));

    assert_eq!((number, json), (64, true));
    assert_eq!((stop.number(), stop.output().is_json()), (64, true));
}

#[test]
fn the_refused_event_fires_when_the_refusal_prints_and_not_before() {
    let mut kept = None;
    let parsing = watch(|| kept = Some(refused(&["tool", "comets"])));
    let printing = watch(|| {
        if let Some(refused) = kept {
            let _ = printed(refused);
        }
    });

    assert!(
        !parsing.iter().any(|event| event.starts_with("refused")),
        "{parsing:?}"
    );
    assert!(
        printing.iter().any(|event| event.starts_with("refused")),
        "{printing:?}"
    );
}

#[test]
fn a_refused_view_flag_reports_its_exit_when_it_prints_and_not_before() {
    /// A tool whose root flattens the view flags.
    #[derive(clap::Parser)]
    #[command(name = "tool")]
    struct Viewed {
        #[command(flatten)]
        view: ViewFlags,
    }

    let mut kept = None;
    let parsing = watch(|| {
        kept = Some(refused_by(
            <Viewed as clap::CommandFactory>::command(),
            &Guide::new(),
            &["tool", "--full"],
        ));
    });
    let printing = watch(|| {
        if let Some(refused) = kept {
            let _ = printed(refused);
        }
    });

    assert!(
        !parsing.iter().any(|event| event.starts_with("exited")),
        "{parsing:?}"
    );
    assert!(
        printing.iter().any(|event| event.starts_with("exited")),
        "{printing:?}"
    );
}

#[test]
fn a_stop_comes_back_after_help_printed() {
    let mut out = Vec::new();
    let unparsed = try_parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, Vec::new()),
        tool(),
        &Guide::new(),
        ["tool", "help"],
    );

    assert!(
        matches!(&unparsed, Err(Unparsed::Stopped(stop)) if stop.number() == 0),
        "{unparsed:?}"
    );
    assert_ne!(out, Vec::<u8>::new());
}
