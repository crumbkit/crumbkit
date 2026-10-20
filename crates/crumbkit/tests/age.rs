//! A tool outside crumbkit whose rows hold an `Age` column, which prints a
//! compact age in the text table and in a stream's text lines, and an RFC
//! 3339 time in every JSON form.

use std::ops::ControlFlow;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::{CommandFactory as _, Parser, Subcommand};
use crumbkit::help::Guide;
use crumbkit::{Age, Batch, Document, Error, Output, Row, Sink, Stream, ViewFlags};
use schemars::JsonSchema;
use serde::Serialize;

/// A tool with a `list` command and a `watch` command.
#[derive(Parser)]
#[command(name = "files")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: ViewFlags,
}

#[derive(Subcommand)]
enum Command {
    /// Lists the files.
    List,
    /// Prints each file as it changes.
    Watch,
}

/// A file and when it last changed.
#[derive(Serialize, JsonSchema)]
struct File {
    /// The file's name.
    name: &'static str,
    /// When the file last changed.
    changed: Age,
    /// When the file was last read, if it was.
    read: Option<Age>,
    /// The file's size in bytes.
    size: u64,
}

impl Row for File {
    const COLUMNS: &'static [&'static str] = &["name", "changed", "read"];
}

const FILES: Document<Vec<File>> = Document::new("files", "files", 1, "The files.");

const FILES_SHOW: Document<Batch<File, Error>> =
    Document::new("files", "files-show", 1, "The files with the given names.");

const FILES_STREAM: Document<Stream<File>> =
    Document::new("files", "files-stream", 1, "Each file as it changes.");

/// The time the file changed, 2026-10-07T12:01:07.120Z.
fn changed() -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(1_791_374_467_120)
}

/// The current time that the tool passes, 3 hours and 25 minutes after the
/// file changed.
fn now() -> SystemTime {
    changed() + Duration::from_secs(3 * 3_600 + 25 * 60)
}

/// One file that changed 3 hours ago and was never read.
fn file() -> File {
    File {
        name: "notes.txt",
        changed: Age::since(changed(), now()),
        read: None,
        size: 12,
    }
}

/// Parses `files` followed by `args`, and returns the output, or `None`
/// when the line does not parse.
fn parse(args: &[&str]) -> Option<Output> {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let words = ["files"].iter().chain(args);
    let parsed = crumbkit::parse_to::<Tool, _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        Tool::command(),
        &Guide::new(),
        words,
    );
    parsed.ok().map(|(_tool, output)| output)
}

/// Runs `files list` with `flags` and returns what it printed on standard
/// output, or `None` when the line does not parse or the output is not
/// UTF-8.
fn list(flags: &[&str]) -> Option<String> {
    let output = parse(&[&["list"], flags].concat())?;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = crumbkit::finish_to(
        &mut Sink::new(&mut out, &mut err),
        output,
        FILES,
        Ok::<_, Error>(vec![file()]),
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
    String::from_utf8(out).ok()
}

/// Runs `files watch` with `flags`, streams one file, and returns what it
/// printed on standard output, or `None` when the line does not parse, the
/// stream does not open or the output is not UTF-8.
fn watch(flags: &[&str]) -> Option<String> {
    let output = parse(&[&["watch"], flags].concat())?;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let mut streaming = crumbkit::stream_to(&mut sink, output, FILES_STREAM).ok()?;
    assert_eq!(streaming.item(&file()), ControlFlow::Continue(()));
    assert_eq!(streaming.end(Ok::<(), Error>(())), ExitCode::SUCCESS);
    String::from_utf8(out).ok()
}

#[test]
fn the_text_table_shows_an_age_column_as_a_compact_age() {
    // The file was never read, so `read` is null in the only row and the
    // table leaves its column out.
    let expected = "\
NAME       CHANGED
notes.txt  3h
";
    assert_eq!(list(&[]).unwrap(), expected);
}

#[test]
fn json_holds_an_age_column_as_an_rfc_3339_time() {
    let json = list(&["--json"]).unwrap();

    assert!(
        json.contains(r#""changed":"2026-10-07T12:01:07.120Z","read":null"#),
        "{json}"
    );
    assert!(!json.contains("3h"), "{json}");
}

#[test]
fn json_with_fields_holds_an_age_column_as_an_rfc_3339_time() {
    let json = list(&["--json", "--fields", "changed"]).unwrap();

    assert!(
        json.contains(r#""data":[{"changed":"2026-10-07T12:01:07.120Z"}]"#),
        "{json}"
    );
}

#[test]
fn json_with_full_holds_an_age_field_as_an_rfc_3339_time() {
    let json = list(&["--json", "--full"]).unwrap();

    assert!(
        json.contains(r#""changed":"2026-10-07T12:01:07.120Z","read":null,"size":12"#),
        "{json}"
    );
}

#[test]
fn the_fields_flag_names_an_age_column_in_text() {
    let expected = "\
CHANGED
3h
";
    assert_eq!(list(&["--fields", "changed"]).unwrap(), expected);
}

#[test]
fn a_streams_text_line_shows_an_age_column_as_a_compact_age() {
    // A stream's column widths come from its header, so the name pushes
    // the rest of its line right. A stream prints its header before any
    // row, so it keeps the `read` column that is null in every row.
    let expected = "\
NAME  CHANGED  READ
notes.txt  3h       -
";
    assert_eq!(watch(&[]).unwrap(), expected);
}

#[test]
fn a_streams_json_holds_an_age_column_as_an_rfc_3339_time() {
    let json = watch(&["--json"]).unwrap();

    assert!(
        json.contains(r#""changed":"2026-10-07T12:01:07.120Z""#),
        "{json}"
    );
    assert!(!json.contains("3h"), "{json}");
}

#[test]
fn a_streams_json_with_full_holds_an_age_field_as_an_rfc_3339_time() {
    let json = watch(&["--json", "--full"]).unwrap();

    assert!(
        json.contains(r#""changed":"2026-10-07T12:01:07.120Z""#),
        "{json}"
    );
}

#[test]
fn a_tool_that_serializes_a_row_itself_gets_the_rfc_3339_time() {
    let json = serde_json::to_string(&file()).unwrap();

    assert!(
        json.contains(r#""changed":"2026-10-07T12:01:07.120Z""#),
        "{json}"
    );
}

#[test]
fn room_counts_an_age_column_of_a_text_table_as_its_compact_age() {
    let output = parse(&["list"]).unwrap();
    let mut room = output.room(FILES_SHOW);
    let mut batch = Batch::new(&["show"], "names", &["notes.txt"]);
    // The file has been read, so the table keeps the `read` column that the
    // room counts.
    batch.push(Ok(File {
        read: Some(Age::since(changed(), now())),
        ..file()
    }));
    assert!(room.has_room(&batch));
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = crumbkit::finish_to(
        &mut Sink::new(&mut out, &mut err),
        output,
        FILES_SHOW,
        Ok::<_, Error>(batch),
    );

    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("notes.txt  3h"), "{text}");
    assert_eq!(room.printed(), text.len(), "{text}");
}
