//! Tests for streams: `stream_to` and the `Streaming` that it opens.

use std::fmt;
use std::io::{self, Write};
use std::ops::ControlFlow;
use std::process::ExitCode;

use clap::{CommandFactory as _, FromArgMatches as _, Parser, Subcommand};
use schemars::JsonSchema;
use serde::Serialize;

use super::stream_to;
use crate::Envelope;
use crate::twins::assert_one_word_apart;
use crate::{
    Call, Code, Document, DryRun, Error, Failure, Output, Recovery, Row, Sink, Spelling, Stream,
};

mod empty;
mod exits;
mod lines;
mod problems;
mod views;
mod whole;

/// A row with two compact columns, one long column and one field that only
/// `--full` shows.
#[derive(Serialize, JsonSchema)]
struct Sighting {
    /// The sighting's number, counting from 1.
    number: u32,
    /// The bird that was seen.
    bird: &'static str,
    /// Where the bird was seen.
    place: &'static str,
    /// The observer's note.
    note: &'static str,
}

impl Row for Sighting {
    const COLUMNS: &'static [&'static str] = &["number", "bird"];
    const LONG_COLUMNS: &'static [&'static str] = &["place"];
}

const SIGHTINGS: Document<Stream<Sighting>> = Document::new(
    "birds",
    "sightings-watch",
    1,
    "Each sighting as it arrives.",
);

fn robin() -> Sighting {
    Sighting {
        number: 1,
        bird: "robin",
        place: "garden",
        note: "singing",
    }
}

fn kingfisher() -> Sighting {
    Sighting {
        number: 12,
        bird: "kingfisher",
        place: "river",
        note: "diving",
    }
}

const NOT_FOUND: Code = Code::new("birds", "sightings", "unreadable");

/// A failure that names a line of the log that could not be read.
#[derive(Debug)]
struct Unreadable(u32);

impl fmt::Display for Unreadable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {} of the log cannot be read", self.0)
    }
}

impl std::error::Error for Unreadable {}

impl Failure for Unreadable {
    fn code(&self) -> &'static Code {
        &NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&["watch"]))
    }
}

/// A tool with one command that streams sightings.
#[derive(Parser)]
#[command(name = "birds")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: crate::ViewFlags,
}

#[derive(Subcommand)]
enum Command {
    Watch {
        #[command(flatten)]
        dry_run: DryRun,
    },
}

/// Parses `birds watch` followed by `flags`, and returns the output as a
/// tool that parses with clap alone builds it. A `--dry-run`
/// in `flags` is recorded in the output, as a handler records it.
fn parse(flags: &[&str]) -> Output {
    let mut command = Tool::command();
    let args = ["birds", "watch"].iter().chain(flags);
    let matches = command.try_get_matches_from_mut(args.clone()).unwrap();
    let tool = Tool::from_arg_matches(&matches).unwrap();
    let mut output = Output::new(tool.view, command, &crate::help::Guide::new(), args);
    let Command::Watch { dry_run } = tool.command;
    let _ = output.dry_run(dry_run);
    output
}

/// What a stream wrote and the exit code it ended with.
struct Streamed {
    code: ExitCode,
    out: String,
    err: String,
}

/// Opens a stream of `document` with `flags`, writes each of `items`,
/// reports `failure` after the first item when it is `Some`, and ends the
/// stream with `end`.
fn run(
    flags: &[&str],
    document: Document<Stream<Sighting>>,
    items: &[Sighting],
    failure: Option<&Unreadable>,
    end: Result<(), Unreadable>,
) -> Streamed {
    let mut output = parse(flags);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let code = match stream_to(&mut sink, &mut output, Spelling::COLONS, document) {
        Ok(mut streaming) => {
            for (index, item) in items.iter().enumerate() {
                assert_eq!(streaming.item(item), ControlFlow::Continue(()));
                if index == 0
                    && let Some(failure) = failure
                {
                    assert_eq!(streaming.failure(failure), ControlFlow::Continue(()));
                }
            }
            streaming.end(end)
        }
        Err(code) => code,
    };
    Streamed {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Streams `items` with `flags` and ends the stream without a failure.
fn run_ok(flags: &[&str], items: &[Sighting]) -> Streamed {
    run(flags, SIGHTINGS, items, None, Ok(()))
}

#[test]
fn text_prints_the_header_and_then_each_item_on_its_own_line() {
    let streamed = run_ok(&[], &[robin(), kingfisher()]);

    assert_eq!(streamed.code, ExitCode::SUCCESS);
    let expected = "\
NUMBER  BIRD
1       robin
12      kingfisher
";
    assert_eq!(streamed.out, expected);
    assert_eq!(streamed.err, "");
}

#[test]
fn a_cell_wider_than_its_header_pushes_the_rest_of_its_line_right() {
    let streamed = run_ok(&["--long"], &[kingfisher()]);

    let expected = "\
NUMBER  BIRD  PLACE
12      kingfisher  river
";
    assert_eq!(streamed.out, expected);
}

#[test]
fn text_prints_the_header_when_the_stream_opens() {
    let mut output = parse(&[]);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);

    let streaming = stream_to(&mut sink, &mut output, Spelling::COLONS, SIGHTINGS).unwrap();
    drop(streaming);

    assert_eq!(String::from_utf8(out).unwrap(), "NUMBER  BIRD\n");
}

#[test]
fn json_prints_each_item_as_a_document_on_its_own_line() {
    let streamed = run_ok(&["--json"], &[robin(), kingfisher()]);

    assert_eq!(streamed.code, ExitCode::SUCCESS);
    let expected = concat!(
        r#"{"schema":"birds/sightings-watch/v1","data":{"number":1,"bird":"robin"}}"#,
        "\n",
        r#"{"schema":"birds/sightings-watch/v1","data":{"number":12,"bird":"kingfisher"}}"#,
        "\n",
    );
    assert_eq!(streamed.out, expected);
    assert_eq!(streamed.err, "");
}

#[test]
fn fields_select_the_fields_of_each_item_in_their_order() {
    let text = run_ok(&["--fields", "place,number"], &[robin()]);
    let json = run_ok(&["--json", "--fields", "place,number"], &[robin()]);

    assert_eq!(text.out, "PLACE  NUMBER\ngarden  1\n");
    assert_eq!(
        json.out,
        concat!(
            r#"{"schema":"birds/sightings-watch/v1","data":{"place":"garden","number":1}}"#,
            "\n"
        )
    );
}

#[test]
fn full_prints_every_field_of_each_item() {
    let streamed = run_ok(&["--json", "--full"], &[robin()]);

    assert_eq!(
        streamed.out,
        concat!(
            r#"{"schema":"birds/sightings-watch/v1","data":{"number":1,"bird":"robin","place":"garden","note":"singing"}}"#,
            "\n"
        )
    );
}

#[test]
fn an_unknown_field_is_refused_with_exit_code_2_before_anything_prints() {
    assert_one_word_apart(&["--fields", "wingspan"], &["--fields", "bird"]);
    let twin = run_ok(&["--fields", "bird"], &[robin()]);
    assert_eq!(twin.code, ExitCode::SUCCESS, "{}", twin.err);
    let mut output = parse(&["--fields", "wingspan"]);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);

    let refused = stream_to(&mut sink, &mut output, Spelling::COLONS, SIGHTINGS);

    assert_eq!(refused.err(), Some(ExitCode::from(2)));
    assert_eq!(String::from_utf8(out).unwrap(), "");
    let err = String::from_utf8(err).unwrap();
    assert!(
        err.starts_with("error: --fields names an unknown field 'wingspan'\n"),
        "{err}"
    );
}

#[test]
fn a_failure_in_the_middle_prints_on_standard_error_in_text_and_the_stream_exits_with_1() {
    let streamed = run(
        &[],
        SIGHTINGS,
        &[robin(), kingfisher()],
        Some(&Unreadable(3)),
        Ok(()),
    );

    assert_eq!(streamed.code, ExitCode::FAILURE);
    assert_eq!(
        streamed.out,
        "NUMBER  BIRD\n1       robin\n12      kingfisher\n"
    );
    let expected = "\
error: line 3 of the log cannot be read
  help:  run `birds watch`
  code:  birds::sightings::unreadable
";
    assert_eq!(streamed.err, expected);
}

#[test]
fn with_json_a_failure_in_the_middle_prints_a_failure_document_between_the_items() {
    let streamed = run(
        &["--json"],
        SIGHTINGS,
        &[robin(), kingfisher()],
        Some(&Unreadable(3)),
        Ok(()),
    );

    assert_eq!(streamed.code, ExitCode::FAILURE);
    let lines: Vec<&str> = streamed.out.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(
        lines[1]
            .starts_with(r#"{"schema":"birds/failure/v1","code":"birds::sightings::unreadable""#),
        "{}",
        lines[1]
    );
    assert!(lines[2].contains("kingfisher"));
    assert_eq!(streamed.err, "");
}

#[test]
fn a_failure_at_the_end_prints_last_and_the_stream_exits_with_1() {
    let text = run(&[], SIGHTINGS, &[robin()], None, Err(Unreadable(9)));
    let json = run(&["--json"], SIGHTINGS, &[robin()], None, Err(Unreadable(9)));

    assert_eq!(text.code, ExitCode::FAILURE);
    assert_eq!(text.out, "NUMBER  BIRD\n1       robin\n");
    assert!(
        text.err
            .starts_with("error: line 9 of the log cannot be read\n")
    );
    assert_eq!(json.code, ExitCode::FAILURE);
    let last = json.out.lines().last().unwrap();
    assert!(
        last.starts_with(r#"{"schema":"birds/failure/v1""#),
        "{last}"
    );
}

#[test]
fn a_dry_run_marks_each_json_line_and_prints_the_dry_run_line_on_standard_error_in_text() {
    let text = run_ok(&["--dry-run"], &[robin()]);
    let json = run_ok(&["--dry-run", "--json"], &[robin()]);

    assert_eq!(text.out, "NUMBER  BIRD\n1       robin\n");
    assert_eq!(text.err, "This was a dry run, so nothing changed.\n");
    assert_eq!(json.err, "");
    assert_eq!(
        json.out,
        concat!(
            r#"{"schema":"birds/sightings-watch/v1","dry_run":true,"data":{"number":1,"bird":"robin"}}"#,
            "\n"
        )
    );
}

#[test]
fn an_unwrapped_stream_prints_bare_items_and_its_failures_as_text() {
    let streamed = run(
        &["--json"],
        SIGHTINGS.envelope(Envelope::Without),
        &[robin()],
        Some(&Unreadable(3)),
        Ok(()),
    );

    assert_eq!(streamed.code, ExitCode::FAILURE);
    assert_eq!(
        streamed.out,
        concat!(
            r#"{"number":1,"bird":"robin","place":"garden","note":"singing"}"#,
            "\n"
        )
    );
    assert!(
        streamed
            .err
            .starts_with("error: line 3 of the log cannot be read\n")
    );
}

/// A writer that counts its flushes and fails every write after its first
/// `writes` with an error of `kind`.
struct Writer {
    written: Vec<u8>,
    flushes: usize,
    writes: usize,
    kind: io::ErrorKind,
}

impl Writer {
    fn new(writes: usize, kind: io::ErrorKind) -> Self {
        Self {
            written: Vec::new(),
            flushes: 0,
            writes,
            kind,
        }
    }
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.writes == 0 {
            return Err(self.kind.into());
        }
        self.writes -= 1;
        self.written.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        Ok(())
    }
}

#[test]
fn the_stream_writes_and_flushes_each_line_at_once() {
    let mut output = parse(&["--json"]);
    let mut out = Writer::new(usize::MAX, io::ErrorKind::Other);
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let mut streaming = stream_to(&mut sink, &mut output, Spelling::COLONS, SIGHTINGS).unwrap();

    let _ = streaming.item(&robin());
    let _ = streaming.item(&kingfisher());
    let _ = streaming.end(Ok::<(), Error>(()));

    assert_eq!(out.flushes, 2);
    assert_eq!(String::from_utf8_lossy(&out.written).lines().count(), 2);
}

#[test]
fn a_reader_that_closes_the_pipe_stops_the_stream_and_the_exit_code_stays_0() {
    let mut output = parse(&[]);
    let mut out = Writer::new(2, io::ErrorKind::BrokenPipe);
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let mut streaming = stream_to(&mut sink, &mut output, Spelling::COLONS, SIGHTINGS).unwrap();

    assert_eq!(streaming.item(&robin()), ControlFlow::Continue(()));
    assert_eq!(streaming.item(&kingfisher()), ControlFlow::Break(()));
    assert_eq!(streaming.item(&robin()), ControlFlow::Break(()));
    let code = streaming.end(Ok::<(), Error>(()));

    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(err, b"");
}

#[test]
fn another_write_error_is_reported_once_and_the_stream_exits_with_1() {
    let mut output = parse(&["--json"]);
    let mut out = Writer::new(0, io::ErrorKind::PermissionDenied);
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let mut streaming = stream_to(&mut sink, &mut output, Spelling::COLONS, SIGHTINGS).unwrap();

    assert_eq!(streaming.item(&robin()), ControlFlow::Break(()));
    assert_eq!(streaming.item(&robin()), ControlFlow::Break(()));
    let code = streaming.end(Ok::<(), Error>(()));

    assert_eq!(code, ExitCode::FAILURE);
    let err = String::from_utf8(err).unwrap();
    assert_eq!(err.lines().count(), 1);
    assert!(
        err.starts_with("error: cannot write to standard output: "),
        "{err}"
    );
}

/// Streams `count` robins with `flags` and a guide whose output budget is
/// 1024 bytes, until the stream breaks, and returns what it wrote and the
/// number of items it accepted.
fn run_budgeted(flags: &[&str], count: usize) -> (Streamed, usize) {
    let mut command = Tool::command();
    let args = ["birds", "watch"].iter().chain(flags);
    let matches = command.try_get_matches_from_mut(args.clone()).unwrap();
    let view = Tool::from_arg_matches(&matches).unwrap().view;
    let guide = crate::help::Guide::new().output_budget(1024, "a short watch");
    let mut output = Output::new(view, command, &guide, args);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let mut accepted = 0;
    let code = match stream_to(&mut sink, &mut output, Spelling::COLONS, SIGHTINGS) {
        Ok(mut streaming) => {
            for _ in 0..count {
                if streaming.item(&robin()).is_break() {
                    break;
                }
                accepted += 1;
                assert!(streaming.budget() <= 1024);
            }
            streaming.end(Ok::<(), Unreadable>(()))
        }
        Err(code) => code,
    };
    let streamed = Streamed {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    };
    (streamed, accepted)
}

#[test]
fn a_stream_stops_before_the_item_that_would_pass_the_budget_and_exits_1() {
    let (streamed, accepted) = run_budgeted(&[], 1000);

    assert_eq!(streamed.code, ExitCode::FAILURE);
    assert!(streamed.out.len() <= 1024);
    // Each line of a robin is 14 bytes, after a header of 13.
    assert_eq!(accepted, (1024 - 13) / 14);
    assert!(
        streamed
            .err
            .starts_with("error: the stream stopped at the output budget of 1024 bytes\n"),
        "{}",
        streamed.err
    );
}

#[test]
fn a_stream_that_fits_its_budget_exits_0() {
    let (streamed, accepted) = run_budgeted(&[], 3);

    assert_eq!(streamed.code, ExitCode::SUCCESS);
    assert_eq!(accepted, 3);
    assert_eq!(streamed.err, "");
}

#[test]
fn a_json_stream_ends_with_the_over_budget_failure_document() {
    let (streamed, _) = run_budgeted(&["--json"], 1000);

    assert_eq!(streamed.code, ExitCode::FAILURE);
    let last = streamed.out.lines().last().unwrap();
    assert!(
        last.contains(r#""code":"birds::output::over_budget""#),
        "{last}"
    );
}

#[test]
fn a_stream_always_prints_its_first_item_even_past_the_budget() {
    let mut command = Tool::command();
    let matches = command
        .try_get_matches_from_mut(["birds", "watch"])
        .unwrap();
    let view = Tool::from_arg_matches(&matches).unwrap().view;
    let guide = crate::help::Guide::new().output_budget(1024, "a short watch");
    let mut output = Output::new(view, command, &guide, ["birds", "watch"]);
    let long: &'static str = Box::leak("w".repeat(2000).into_boxed_str());
    let wren = Sighting {
        bird: long,
        ..robin()
    };
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let mut streaming = stream_to(&mut sink, &mut output, Spelling::COLONS, SIGHTINGS).unwrap();

    assert_eq!(streaming.item(&wren), ControlFlow::Continue(()));
    assert_eq!(streaming.budget(), 0);
    assert_eq!(streaming.item(&robin()), ControlFlow::Break(()));
    assert_eq!(streaming.end(Ok::<(), Unreadable>(())), ExitCode::FAILURE);
    assert!(String::from_utf8(out).unwrap().contains(long));
}
