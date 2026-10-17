//! Tests for picking a missing argument: what `parse_to` asks the guide's
//! picker, what it adds to the command line, and when it refuses instead.

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use clap::{Arg, ArgAction, ArgMatches, Command, FromArgMatches};

use crate::help::{Guide, Leaf};
use crate::pick::{Answer, Choice, Choices, Picker, Question};
use crate::twins::assert_one_word_apart;
use crate::{Error, Sink, Spelling, Stop, parse_to};

/// A tool whose `show` takes a required `--list` and an optional `--sort`,
/// whose `get` takes a required positional `NAME` with possible values,
/// whose `take` takes one or more positional `ID`s, and whose `find` takes
/// a required `--pattern` without any choices.
fn tool() -> Command {
    Command::new("tool")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("crumbkit_no_input")
                .long("no-input")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("show")
                .arg(
                    Arg::new("list")
                        .long("list")
                        .value_name("LIST")
                        .required(true)
                        .help("The list to show"),
                )
                .arg(Arg::new("sort").long("sort")),
        )
        .subcommand(
            Command::new("get").arg(
                Arg::new("name")
                    .value_name("NAME")
                    .required(true)
                    .value_parser(["mars", "venus"]),
            ),
        )
        .subcommand(
            Command::new("take").arg(
                Arg::new("ids")
                    .value_name("ID")
                    .required(true)
                    .num_args(1..),
            ),
        )
        .subcommand(Command::new("find").arg(Arg::new("pattern").long("pattern").required(true)))
}

/// A picker that writes the question on one line and reads the answer
/// from one line of input: positions separated by spaces, or nothing,
/// which cancels.
struct Scripted;

impl Picker for Scripted {
    fn pick(
        &self,
        question: &Question<'_>,
        input: &mut dyn BufRead,
        output: &mut dyn Write,
    ) -> io::Result<Answer> {
        let labels: Vec<&str> = question.choices().iter().map(Choice::label).collect();
        writeln!(
            output,
            "pick {} for {}: {}",
            question.name(),
            question.command(),
            labels.join(", ")
        )?;
        let mut line = String::new();
        input.read_line(&mut line)?;
        let positions: Vec<usize> = line
            .split_whitespace()
            .map(|word| word.parse().unwrap_or(usize::MAX))
            .collect();
        if positions.is_empty() {
            Ok(Answer::Cancelled)
        } else {
            Ok(Answer::Picked(positions))
        }
    }
}

static SCRIPTED: Scripted = Scripted;

#[expect(
    clippy::unnecessary_wraps,
    reason = "a choices source returns a `Result`"
)]
fn lists() -> Result<Vec<Choice>, Error> {
    Ok(vec![
        Choice::new("to read".to_owned(), "to read  3 books".to_owned()),
        Choice::new("-done".to_owned(), "-done  1 book".to_owned()),
    ])
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "a choices source returns a `Result`"
)]
fn ids() -> Result<Vec<Choice>, Error> {
    Ok(["1", "2", "3"]
        .iter()
        .map(|id| Choice::new((*id).to_owned(), format!("book {id}")))
        .collect())
}

/// A failure of the source of `--pattern`.
#[derive(Debug)]
struct Offline;

impl std::fmt::Display for Offline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the catalogue is offline")
    }
}

impl std::error::Error for Offline {}

static OFFLINE: crate::Code = crate::Code::new("tool", "catalogue", "offline");

impl crate::Failure for Offline {
    fn code(&self) -> &'static crate::Code {
        &OFFLINE
    }

    fn recovery(&self) -> crate::Recovery {
        crate::Recovery::Retry
    }
}

fn offline() -> Result<Vec<Choice>, Error> {
    Err(Offline.into())
}

const GUIDE: Guide = Guide::new().picker(&SCRIPTED).leaves(&[
    Leaf::new(&["show"]).choices(&[Choices::new("list", lists)]),
    // A source for `name`, which has possible values, is never called.
    Leaf::new(&["get"]).choices(&[Choices::new("name", offline)]),
    Leaf::new(&["take"]).choices(&[Choices::new("ids", ids)]),
]);

/// The matches that clap parsed, as the tool's parsed type.
struct Matches(ArgMatches);

impl FromArgMatches for Matches {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        Ok(Self(matches.clone()))
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        self.0 = matches.clone();
        Ok(())
    }
}

/// What `parse_to` returned and printed.
struct Run {
    result: Result<(Matches, crate::Output), Stop>,
    out: String,
    err: String,
}

/// Parses `tool` followed by `words` with `guide`, answering any question
/// with `answer` when `interactive`.
fn run(guide: &Guide, words: &[&str], answer: &str, interactive: bool) -> Run {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::with_input(&mut out, &mut err, answer.as_bytes(), interactive);
    let args = std::iter::once("tool").chain(words.iter().copied());
    let result = parse_to::<Matches, _, _, _, _>(&mut sink, tool(), guide, Spelling::COLONS, args);
    Run {
        result,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Checks that `parse_to` accepts the twin of a refusal: `words` with
/// `answer` when `interactive` (TEST-1). A twin of a refusal that an answer
/// or the terminal causes keeps the words and changes only that.
#[track_caller]
fn admits(words: &[&str], answer: &str, interactive: bool) {
    let admitted = run(&GUIDE, words, answer, interactive);
    assert!(admitted.result.is_ok(), "{}", admitted.err);
}

/// Returns the words of a parsed run.
fn words(run: &Run) -> Vec<String> {
    let (_, parsed) = run.result.as_ref().unwrap();
    parsed
        .words()
        .iter()
        .map(|word| word.to_string_lossy().into_owned())
        .collect()
}

/// Returns the exit code of a stopped run.
fn code(run: &Run) -> ExitCode {
    run.result.as_ref().err().unwrap().code()
}

#[test]
fn a_missing_option_is_picked_and_added_before_any_double_dash() {
    let run = run(&GUIDE, &["show", "--sort", "id"], "0\n", true);
    assert_eq!(words(&run), ["show", "--sort", "id", "--list=to read"]);
    let (matches, _) = run.result.as_ref().unwrap();
    let show = matches.0.subcommand_matches("show").unwrap();
    assert_eq!(
        show.get_one::<String>("list").map(String::as_str),
        Some("to read")
    );
    assert_eq!(
        run.err,
        "pick LIST for tool show: to read  3 books, -done  1 book\n\
         Running: tool show --sort id '--list=to read'\n"
    );
}

#[test]
fn a_missing_positional_with_possible_values_offers_them_without_calling_the_source() {
    let run = run(&GUIDE, &["get"], "1\n", true);
    assert_eq!(words(&run), ["get", "venus"]);
    assert!(run.err.starts_with("pick NAME for tool get: mars, venus\n"));
}

#[test]
fn several_picked_values_of_a_positional_are_added_in_the_order_picked() {
    let run = run(&GUIDE, &["take"], "2 0\n", true);
    assert_eq!(words(&run), ["take", "3", "1"]);
}

#[test]
fn a_picked_positional_that_starts_with_a_hyphen_follows_a_double_dash() {
    const GUIDE: Guide = Guide::new()
        .picker(&SCRIPTED)
        .leaves(&[Leaf::new(&["take"]).choices(&[Choices::new("ids", lists)])]);
    let run = run(&GUIDE, &["take"], "1\n", true);
    assert_eq!(words(&run), ["take", "--", "-done"]);
}

#[test]
fn a_command_line_with_json_still_picks() {
    let run = run(&GUIDE, &["show", "--json"], "0\n", true);
    assert_eq!(words(&run), ["show", "--json", "--list=to read"]);
    assert_eq!(run.out, "");
}

#[test]
fn a_cancelled_pick_prints_the_refusal_that_parse_prints_without_a_picker() {
    admits(&["show"], "0\n", true);
    let picked = run(&GUIDE, &["show"], "\n", true);
    let refused = run(&Guide::new(), &["show"], "", true);
    assert_eq!(code(&picked), ExitCode::from(2));
    let refusal = picked.err.split_once('\n').unwrap().1;
    assert_eq!(refusal, refused.err);
}

#[test]
fn an_answer_outside_the_list_is_refused() {
    admits(&["show"], "0\n", true);
    let run = run(&GUIDE, &["show"], "7\n", true);
    assert_eq!(code(&run), ExitCode::from(2));
}

#[test]
fn an_answer_with_more_values_than_the_argument_takes_is_refused() {
    admits(&["show"], "0\n", true);
    let run = run(&GUIDE, &["show"], "0 1\n", true);
    assert_eq!(code(&run), ExitCode::from(2));
}

#[test]
fn a_sink_where_nobody_can_answer_refuses_without_asking() {
    admits(&["show"], "0\n", true);
    let run = run(&GUIDE, &["show"], "0\n", false);
    assert_eq!(code(&run), ExitCode::from(2));
    assert!(!run.err.contains("pick"));
}

#[test]
fn no_input_refuses_without_asking() {
    assert_one_word_apart(&["show", "--no-input"], &["show"]);
    admits(&["show"], "0\n", true);
    let run = run(&GUIDE, &["show", "--no-input"], "0\n", true);
    assert_eq!(code(&run), ExitCode::from(2));
    assert!(!run.err.contains("pick"));
}

#[test]
fn with_terminal_detection_off_a_missing_argument_is_refused_without_asking_the_picker() {
    const QUIET: Guide = GUIDE.without_terminal_detection("Our CI runs in a pseudo-terminal.");
    assert_one_word_apart(&["show"], &["show", "--list=a"]);
    let admitted = run(&QUIET, &["show", "--list=a"], "", true);
    assert!(admitted.result.is_ok(), "{}", admitted.err);
    let run = run(&QUIET, &["show"], "0\n", true);
    assert_eq!(code(&run), ExitCode::from(2));
    assert!(!run.err.contains("pick"));
}

#[test]
fn an_argument_without_choices_is_refused_without_asking() {
    assert_one_word_apart(&["find"], &["find", "--pattern=mars"]);
    admits(&["find", "--pattern=mars"], "0\n", true);
    let run = run(&GUIDE, &["find"], "0\n", true);
    assert_eq!(code(&run), ExitCode::from(2));
    assert!(!run.err.contains("pick"));
}

#[test]
fn a_failed_source_prints_its_failure_and_exits_with_1() {
    const GUIDE: Guide = Guide::new()
        .picker(&SCRIPTED)
        .leaves(&[Leaf::new(&["find"]).choices(&[Choices::new("pattern", offline)])]);
    let run = run(&GUIDE, &["find"], "0\n", true);
    assert_eq!(code(&run), ExitCode::FAILURE);
    assert!(run.err.contains("the catalogue is offline"), "{}", run.err);
}

#[test]
fn a_parsed_command_line_with_no_input_records_it() {
    let run = run(&GUIDE, &["show", "--list", "a", "--no-input"], "", true);
    let (_, parsed) = run.result.as_ref().unwrap();
    assert!(parsed.no_input());
    let run = self::run(&GUIDE, &["show", "--list", "a"], "", true);
    assert!(!run.result.as_ref().unwrap().1.no_input());
}

#[test]
fn parse_reports_how_many_arguments_a_picker_filled_and_the_exit_of_a_failed_source() {
    const FAILING: Guide = Guide::new()
        .picker(&SCRIPTED)
        .leaves(&[Leaf::new(&["find"]).choices(&[Choices::new("pattern", offline)])]);

    let picked = crate::events::testing::watch(|| drop(run(&GUIDE, &["show"], "0\n", true)));
    let failed = crate::events::testing::watch(|| drop(run(&FAILING, &["find"], "0\n", true)));

    assert_eq!(picked[1..], ["parsed tool show", "picked 1"]);
    assert_eq!(failed[1..], ["failed tool::catalogue::offline", "exited 1"]);
}
