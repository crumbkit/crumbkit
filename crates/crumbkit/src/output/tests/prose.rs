//! Tests for how `finish_to` prints a prose result: its text, its failures
//! and its document.

use std::fmt;
use std::process::ExitCode;

use clap::{CommandFactory as _, Parser, Subcommand};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    Call, Code, Document, Error, Failure, Output, Prose, Recovery, Sink, Spelling, finish_to,
};

const NOT_FOUND: Code = Code::new("solar", "planets", "not_found");

/// The answer of a command that counts planets. Its `Display` prints `text`.
#[derive(Serialize, JsonSchema)]
struct Count {
    /// The number of planets.
    planets: u32,
    /// The text that `Display` prints.
    #[serde(skip)]
    text: &'static str,
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text)
    }
}

/// Returns the count of 8 planets whose text is `text`.
const fn eight(text: &'static str) -> Count {
    Count { planets: 8, text }
}

/// A failure that names a planet the solar system lacks, with the
/// alternatives it offers after its preferred recovery.
#[derive(Debug)]
struct Missing(&'static str, Vec<Recovery>);

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the solar system has no planet {}", self.0)
    }
}

impl std::error::Error for Missing {}

impl Failure for Missing {
    fn code(&self) -> &'static Code {
        &NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&["list"]))
    }

    fn alternatives(&self) -> Vec<Recovery> {
        self.1.clone()
    }
}

/// A tool with a `count` command and a `list` command that a recovery names.
#[derive(Parser)]
#[command(name = "tool")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    output: Output,
}

#[derive(Subcommand)]
enum Command {
    Count,
    List,
}

/// What `finish_to` returned and wrote.
struct Finished {
    code: ExitCode,
    out: String,
    err: String,
}

/// Parses `tool count` followed by `flags`, sets the tool's command as its
/// `main` would, and finishes `prose` into memory.
fn finish_with(flags: &[&str], prose: Prose<Count, Missing>) -> Finished {
    let document: Document<Prose<Count, Missing>> =
        Document::new("solar", "count", 1, "The number of planets.");
    let args = ["tool", "count"].iter().chain(flags);
    let mut output = Tool::try_parse_from(args).unwrap().output;
    output.set_command(Tool::command());
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<_, Error> = Ok(prose);
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
        Spelling::COLONS,
        document,
        result,
    );
    Finished {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

#[test]
fn prose_prints_its_text_and_one_line_feed_and_exits_with_code_0() {
    let finished = finish_with(&[], Prose::new(eight("The solar system has 8 planets.")));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, "The solar system has 8 planets.\n");
    assert_eq!(finished.err, "");
}

#[test]
fn prose_that_ends_in_a_line_feed_gets_no_second_one() {
    let finished = finish_with(&[], Prose::new(eight("Eight planets.\nNo more.\n")));

    assert_eq!(finished.out, "Eight planets.\nNo more.\n");
}

#[test]
fn prose_escapes_each_control_character_other_than_a_line_feed_or_a_tab() {
    let finished = finish_with(
        &[],
        Prose::new(eight("Eight\tplanets\u{1b}[31m.\r\nNo more.")),
    );

    assert_eq!(finished.out, "Eight\tplanets\\u{1b}[31m.\\r\nNo more.\n");
}

#[test]
fn empty_prose_prints_nothing() {
    let finished = finish_with(&[], Prose::new(eight("")));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, "");
}

#[test]
fn json_of_prose_holds_the_value_under_data_and_an_empty_failures_array() {
    let finished = finish_with(&["--json"], Prose::new(eight("Eight planets.")));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/count/v1","data":{"planets":8},"failures":[]}"#,
            "\n"
        )
    );
}

#[test]
fn prose_with_failures_prints_the_text_then_each_failure_on_standard_error_and_exits_with_code_1() {
    let prose = Prose::with_failures(eight("Eight planets."), vec![Missing("Vulcan", Vec::new())]);

    let finished = finish_with(&[], prose);

    let expected = "\
error: the solar system has no planet Vulcan
  help:  run `tool list`
  code:  solar::planets::not_found
";
    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, "Eight planets.\n");
    assert_eq!(finished.err, expected);
}

#[test]
fn json_of_prose_with_failures_holds_them_as_a_batch_does_and_exits_with_code_1() {
    let prose = Prose::with_failures(eight("Eight planets."), vec![Missing("Vulcan", Vec::new())]);

    let finished = finish_with(&["--json"], prose);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/count/v1","data":{"planets":8},"failures":["#,
            r#"{"code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","recovery":{"kind":"run","call":"tool list"}}"#,
            "]}\n"
        )
    );
    assert_eq!(finished.err, "");
}

#[test]
fn fields_on_prose_is_a_refusal_that_exits_with_code_2_before_anything_prints() {
    let finished = finish_with(
        &["--fields", "planets"],
        Prose::new(eight("Eight planets.")),
    );

    let expected = "\
error: --fields selects the columns of a table, and this command prints prose
  help:  use --json to read every field of the result
  code:  crumbkit::cli::usage
";
    assert_eq!(finished.code, ExitCode::from(2));
    assert_eq!(finished.out, "");
    assert_eq!(finished.err, expected);
}

#[test]
fn the_refusal_of_fields_on_prose_names_the_flag_under_fields() {
    let finished = finish_with(
        &["--json", "--fields", "planets"],
        Prose::new(eight("Eight planets.")),
    );

    assert!(
        finished
            .out
            .contains(r#""fields":{"argument":"--fields"},"#),
        "{}",
        finished.out
    );
}

#[test]
fn long_and_full_leave_prose_unchanged() {
    let text = finish_with(&[], Prose::new(eight("Eight planets.")));
    let long = finish_with(&["--long"], Prose::new(eight("Eight planets.")));
    let json = finish_with(&["--json"], Prose::new(eight("Eight planets.")));
    let full = finish_with(&["--json", "--full"], Prose::new(eight("Eight planets.")));

    assert_eq!(long.out, text.out);
    assert_eq!(full.out, json.out);
}

/// Returns a prose result whose one failure offers a hint and a retry as
/// alternatives.
fn with_alternatives() -> Prose<Count, Missing> {
    let alternatives = vec![
        Recovery::Hint("set SOLAR_TOKEN".to_owned()),
        Recovery::Retry,
    ];
    Prose::with_failures(
        eight("Eight planets."),
        vec![Missing("Vulcan", alternatives)],
    )
}

#[test]
fn each_failure_of_prose_prints_its_alternatives_as_help_lines_after_its_recovery() {
    let finished = finish_with(&[], with_alternatives());

    let expected = "\
error: the solar system has no planet Vulcan
  help:  run `tool list`
  help:  set SOLAR_TOKEN
  help:  run the same command again later
  code:  solar::planets::not_found
";
    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, "Eight planets.\n");
    assert_eq!(finished.err, expected);
}

#[test]
fn json_of_prose_holds_the_alternatives_of_each_failure_after_its_recovery() {
    let finished = finish_with(&["--json"], with_alternatives());

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/count/v1","data":{"planets":8},"failures":["#,
            r#"{"code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","recovery":{"kind":"run","call":"tool list"},"alternatives":[{"kind":"hint","text":"set SOLAR_TOKEN"},{"kind":"retry"}]}"#,
            "]}\n"
        )
    );
}
