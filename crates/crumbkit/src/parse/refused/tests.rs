//! Tests for `try_parse_to`: a refused command line comes back unprinted
//! with the words that the caller typed and parsed, and prints what
//! `parse_to` prints when the tool prints it.

use std::ffi::{OsStr, OsString};

use clap::error::{ContextKind, ContextValue, ErrorKind};
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

/// Checks that `admitted`, the twin of the refused `refused`, differs from
/// it by one word and that `try_parse_to` over `command` parses it
/// (TEST-1).
#[track_caller]
fn admits(command: Command, refused: &[&str], admitted: &[&str]) {
    assert_one_word_apart(refused, admitted);
    let mut sink = Sink::new(Vec::new(), Vec::new());
    let parsed = try_parse_to::<(), _, _, _, _>(&mut sink, command, &Guide::new(), admitted);
    assert!(
        parsed.is_ok(),
        "the admitted words {admitted:?} are refused"
    );
}

/// Returns the argument that `refused`'s clap error names as invalid.
fn invalid_arg(refused: &Refused) -> Option<String> {
    match refused.error()?.get(ContextKind::InvalidArg)? {
        ContextValue::String(argument) => Some(argument.clone()),
        _ => None,
    }
}

#[test]
fn a_repeated_argument_s_refusal_exposes_clap_s_error_naming_the_argument() {
    let args = [
        "tool", "planets", "show", "Mars", "--sort", "a", "--sort", "b",
    ];
    admits(
        tool(),
        &args,
        &[
            "tool", "planets", "show", "Mars", "--sort", "a", "Venus", "b",
        ],
    );

    let refused = refused(&args);

    let kind = refused.error().map(clap::Error::kind);
    assert_eq!(kind, Some(ErrorKind::ArgumentConflict));
    assert_eq!(invalid_arg(&refused).as_deref(), Some("--sort <sort>"));
}

#[test]
fn a_conflicting_argument_s_refusal_exposes_clap_s_error_naming_the_argument() {
    let command = Command::new("tool")
        .arg(
            Arg::new("all")
                .long("all")
                .action(ArgAction::SetTrue)
                .conflicts_with("only"),
        )
        .arg(Arg::new("only").long("only"));
    let args = ["tool", "--all", "--only", "Mars"];
    admits(command.clone(), &args, &["tool", "--only", "Mars"]);

    let refused = refused_by(command, &Guide::new(), &args);

    let kind = refused.error().map(clap::Error::kind);
    assert_eq!(kind, Some(ErrorKind::ArgumentConflict));
    assert_eq!(invalid_arg(&refused).as_deref(), Some("--all"));
}

#[test]
fn a_refused_view_flag_exposes_no_clap_error() {
    /// A tool whose root flattens the view flags.
    #[derive(clap::Parser)]
    #[command(name = "tool")]
    struct Viewed {
        #[command(flatten)]
        view: ViewFlags,
    }
    let command = <Viewed as clap::CommandFactory>::command();
    admits(
        command.clone(),
        &["tool", "--full"],
        &["tool", "--full", "--json"],
    );

    let refused = refused_by(command, &Guide::new(), &["tool", "--full"]);

    assert_eq!(refused.error().map(clap::Error::kind), None);
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

/// A tool with a global `--format` option and a global `-q` flag, and a
/// `planets` group whose `show` takes names, a `--sort` key and the flags
/// `-v` and `-l`.
fn walked_tool() -> Command {
    /// Returns the flag `-letter` with the id `id`.
    fn flag(id: &'static str, letter: char) -> Arg {
        Arg::new(id).short(letter).action(ArgAction::SetTrue)
    }
    Command::new("tool")
        .arg(Arg::new("format").long("format").global(true))
        .arg(flag("quiet", 'q').global(true))
        .subcommand(
            Command::new("planets").subcommand(
                Command::new("show")
                    .arg(Arg::new("names").required(true).num_args(1..))
                    .arg(Arg::new("sort").long("sort"))
                    .arg(flag("verbose", 'v'))
                    .arg(flag("long", 'l')),
            ),
        )
}

/// Checks that `try_parse_to` over `walked_tool()` parses `admitted`, the
/// twin of `refused`, and returns the refusal of `refused` (TEST-1).
#[track_caller]
fn walked(refused: &[&str], admitted: &[&str]) -> Refused {
    admits(walked_tool(), refused, admitted);
    refused_by(walked_tool(), &Guide::new(), refused)
}

#[test]
fn a_refusal_below_a_group_reports_the_path_to_its_deepest_command() {
    let refused = walked(
        &["tool", "planets", "show"],
        &["tool", "planets", "show", "Mars"],
    );

    assert_eq!(refused.path(), ["planets", "show"]);
}

#[test]
fn a_refusal_at_the_root_reports_an_empty_path() {
    let refused = walked(&["tool", "comets"], &["tool", "planets"]);

    assert_eq!(refused.path(), Vec::<String>::new());
}

#[test]
fn a_refusal_reports_its_flags_in_the_order_typed_with_each_flag_of_a_group() {
    let refused = walked(
        &["tool", "-q", "planets", "show", "--sort", "name", "-vl"],
        &[
            "tool", "-q", "planets", "show", "--sort", "name", "-vl", "Mars",
        ],
    );

    assert_eq!(refused.flags(), ["quiet", "sort", "verbose", "long"]);
}

#[test]
fn a_refusal_whose_help_word_crumbkit_turned_into_a_flag_reports_clap_s_help_id() {
    let refused = walked(
        &["tool", "planets", "help", "nonsense"],
        &["tool", "planets", "show", "nonsense"],
    );

    assert_eq!(refused.parsed(), words(&["planets", "nonsense", "--help"]));
    assert_eq!(refused.flags(), ["help"]);
}

#[test]
fn a_refusal_reports_a_global_option_s_value_typed_before_the_subcommand() {
    let following = walked(
        &["tool", "--format", "json", "planets", "show"],
        &["tool", "--format", "json", "planets", "show", "Mars"],
    );
    let inline = walked(
        &["tool", "--format=json", "planets", "show"],
        &["tool", "--format=json", "planets", "show", "Mars"],
    );

    assert_eq!(following.value("format"), Some(OsStr::new("json")));
    assert_eq!(inline.value("format"), Some(OsStr::new("json")));
}

#[test]
fn a_refusal_reports_a_global_option_s_value_typed_after_the_subcommand() {
    let following = walked(
        &["tool", "planets", "show", "--format", "yaml"],
        &["tool", "planets", "show", "--format", "yaml", "Mars"],
    );
    let inline = walked(
        &["tool", "planets", "show", "--format=yaml"],
        &["tool", "planets", "show", "--format=yaml", "Mars"],
    );

    assert_eq!(following.value("format"), Some(OsStr::new("yaml")));
    assert_eq!(inline.value("format"), Some(OsStr::new("yaml")));
}

#[test]
fn a_refusal_reports_the_last_value_that_the_words_give_an_option() {
    let refused = walked(
        &[
            "tool", "planets", "show", "--sort", "name", "--sort", "mass",
        ],
        &["tool", "planets", "show", "Mars", "name", "--sort", "mass"],
    );

    assert_eq!(refused.value("sort"), Some(OsStr::new("mass")));
}

#[test]
fn a_refusal_reports_no_value_for_an_option_left_out_a_flag_or_an_unknown_id() {
    let refused = walked(
        &["tool", "-q", "planets", "show"],
        &["tool", "-q", "planets", "show", "Mars"],
    );

    assert_eq!(refused.value("format"), None);
    assert_eq!(refused.value("quiet"), None);
    assert_eq!(refused.value("nonsense"), None);
}

#[test]
fn a_refusal_whose_words_typed_json_prints_text_on_standard_error_after_as_json_false() {
    assert_one_word_apart(&["tool", "comets", "--json"], &["tool", "comets"]);

    let (out, err) = printed(refused(&["tool", "comets", "--json"]).as_json(false));

    assert_eq!(
        (out, err),
        parse_printed(tool(), &Guide::new(), &["tool", "comets"])
    );
}

#[test]
fn a_refusal_without_json_prints_the_failure_document_on_standard_output_after_as_json_true() {
    assert_one_word_apart(&["tool", "comets"], &["tool", "comets", "--json"]);

    let (out, err) = printed(refused(&["tool", "comets"]).as_json(true));

    assert_eq!(
        (out, err),
        parse_printed(tool(), &Guide::new(), &["tool", "comets", "--json"])
    );
}

#[test]
fn a_refusal_without_as_json_prints_in_the_form_that_the_words_chose() {
    let (json_out, json_err) = printed(refused(&["tool", "comets", "--json"]));
    let (text_out, text_err) = printed(refused(&["tool", "comets"]));

    assert!(json_out.contains("tool/failure/v1"), "{json_out}");
    assert_eq!((json_err, text_out), (String::new(), String::new()));
    assert!(text_err.contains("comets"), "{text_err}");
}

#[test]
fn as_json_leaves_the_output_that_the_refusal_reports_unchanged() {
    let refused = refused(&["tool", "comets", "--json"]).as_json(false);

    assert!(refused.output().is_json(), "{:?}", refused.output());
}

#[test]
fn the_stop_after_as_json_reports_the_form_that_printed() {
    let sink = &mut Sink::new(Vec::new(), Vec::new());
    let text = refused(&["tool", "comets", "--json"])
        .as_json(false)
        .print_to(sink);
    let json = refused(&["tool", "comets"]).as_json(true).print_to(sink);

    assert_eq!(
        (text.output().is_json(), json.output().is_json()),
        (false, true)
    );
}

#[test]
fn as_json_changes_neither_the_exit_code_nor_the_observer_events() {
    const GUIDE: Guide = Guide::new()
        .exits(&[Exit::new(64, "The command line was refused.").failures(&[&CLI_USAGE])]);
    let args = ["tool", "comets", "--json"];
    let run = |form: Option<bool>| {
        let mut number = 0;
        let events = watch(|| {
            let refused = refused_by(tool(), &GUIDE, &args);
            let refused = match form {
                Some(json) => refused.as_json(json),
                None => refused,
            };
            number = refused
                .print_to(&mut Sink::new(Vec::new(), Vec::new()))
                .number();
        });
        (number, events)
    };

    let (number, events) = run(None);

    assert_eq!(number, 64);
    assert!(
        events.iter().any(|event| event.starts_with("refused")),
        "{events:?}"
    );
    assert_eq!(run(Some(false)), (number, events.clone()));
    assert_eq!(run(Some(true)), (number, events));
}
