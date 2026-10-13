//! Tests for how `finish_to` prints a bytes result: the bytes unchanged in
//! text, the text or base64 object with `--json`, the refused flags and the
//! output budget.

use std::process::ExitCode;

use clap::{CommandFactory as _, FromArgMatches as _, Parser};
use schemars::JsonSchema;
use serde::Serialize;

use super::Tool;
use crate::answered::answered;
use crate::help::Guide;
use crate::twins::assert_one_word_apart;
use crate::{
    Batch, Bytes, Document, DryRun, Error, Output, Parsed, Row, Sink, Spelling, finish_to,
};

const FILE: Document<Bytes> = Document::new("files", "file", 1, "The bytes of one file.");

/// The smallest budget that a tool can set.
const BUDGET: usize = 1024;

/// What `finish_to` returned and wrote, as bytes, because a bytes result
/// need not be UTF-8.
struct Finished {
    code: ExitCode,
    out: Vec<u8>,
    err: String,
}

/// Parses `tool list` followed by `flags` with a guide whose budget is
/// `budget`, as a tool's `main` sets the output flags.
fn output_with(budget: usize, flags: &[&str]) -> Output {
    let mut command = Tool::command();
    let args = ["tool", "list"].iter().chain(flags);
    let matches = command.try_get_matches_from_mut(args).unwrap();
    let mut output = Output::from_arg_matches(&matches).unwrap();
    let guide = Guide::new().output_budget(budget, "the tests print little");
    output.set_command(Parsed::guided(command, &guide));
    output
}

/// Finishes `bytes` as `document` with the flags and budget in `output`.
fn finish_output(output: &mut Output, document: Document<Bytes>, bytes: Bytes) -> Finished {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<Bytes, Error> = Ok(bytes);
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        output,
        Spelling::COLONS,
        document,
        result,
    );
    Finished {
        code,
        out,
        err: String::from_utf8(err).unwrap(),
    }
}

/// Finishes `bytes` as [`FILE`] with `flags` and crumbkit's default budget.
fn finish_with(flags: &[&str], bytes: Bytes) -> Finished {
    finish_output(&mut output_with(Output::DEFAULT_BUDGET, flags), FILE, bytes)
}

/// Returns bytes that hold a tab, an escape sequence and a byte that is not
/// UTF-8, and end without a line feed.
fn raw() -> Bytes {
    Bytes::new(b"a\tb\x1b[31m\xff".to_vec())
}

#[test]
fn text_output_writes_the_bytes_unchanged_without_a_line_feed_and_exits_with_code_0() {
    let finished = finish_with(&[], raw());

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, b"a\tb\x1b[31m\xff");
    assert_eq!(finished.err, "");
}

#[test]
fn empty_bytes_print_nothing_in_text() {
    let finished = finish_with(&[], Bytes::new(Vec::new()));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, b"");
}

#[test]
fn json_holds_utf8_bytes_as_text_under_data() {
    let finished = finish_with(&["--json"], Bytes::from("Emma\n".to_owned()));

    assert_eq!(
        String::from_utf8(finished.out).unwrap(),
        "{\"schema\":\"files/file/v1\",\"data\":{\"text\":\"Emma\\n\"}}\n"
    );
}

#[test]
fn json_holds_bytes_that_are_not_utf8_as_base64_under_data() {
    let finished = finish_with(&["--json"], Bytes::new(vec![0x89, 0x50, 0x4e, 0x47]));

    assert_eq!(
        String::from_utf8(finished.out).unwrap(),
        "{\"schema\":\"files/file/v1\",\"data\":{\"base64\":\"iVBORw==\"}}\n"
    );
}

#[test]
fn json_of_a_document_without_the_envelope_is_the_object_alone() {
    let mut output = output_with(Output::DEFAULT_BUDGET, &["--json"]);
    let finished = finish_output(&mut output, FILE.without_envelope(), raw());

    assert_eq!(
        String::from_utf8(finished.out).unwrap(),
        "{\"base64\":\"YQliG1szMW3/\"}\n"
    );
}

#[test]
fn fields_on_bytes_is_a_refusal_that_exits_with_code_2_before_anything_prints() {
    assert_one_word_apart(&["--fields=text"], &[]);
    let twin = finish_with(&[], raw());
    assert_eq!(twin.code, ExitCode::SUCCESS, "{}", twin.err);
    let finished = finish_with(&["--fields=text"], raw());

    let expected = "\
error: --fields selects the columns of a table, and this command prints raw bytes
  help:  use --json to read the bytes as text or base64
  code:  crumbkit::cli::usage
";
    assert_eq!(finished.code, ExitCode::from(2));
    assert_eq!(finished.out, b"");
    assert_eq!(finished.err, expected);
}

#[test]
fn long_on_bytes_is_a_refusal_that_exits_with_code_2() {
    assert_one_word_apart(&["--long"], &[]);
    let twin = finish_with(&[], raw());
    assert_eq!(twin.code, ExitCode::SUCCESS, "{}", twin.err);
    let finished = finish_with(&["--long"], raw());

    let expected = "\
error: --long adds the long columns of a table, and this command prints raw bytes
  help:  use --json to read the bytes as text or base64
  code:  crumbkit::cli::usage
";
    assert_eq!(finished.code, ExitCode::from(2));
    assert_eq!(finished.out, b"");
    assert_eq!(finished.err, expected);
}

#[test]
fn full_on_bytes_is_a_refusal_that_names_the_flag_under_fields() {
    assert_one_word_apart(&["--json", "--full"], &["--json"]);
    let twin = finish_with(&["--json"], raw());
    assert_eq!(twin.code, ExitCode::SUCCESS, "{}", twin.err);
    let finished = finish_with(&["--json", "--full"], raw());

    let out = String::from_utf8(finished.out).unwrap();
    assert_eq!(finished.code, ExitCode::from(2));
    assert!(
        out.contains("--full prints every field of every row, and this command prints raw bytes"),
        "{out}"
    );
    assert!(out.contains("drop --full, because --json already prints the whole result"));
    assert!(out.contains(r#""fields":{"argument":"--full"}"#), "{out}");
}

#[test]
fn bytes_exactly_as_long_as_the_budget_print_whole() {
    let mut output = output_with(BUDGET, &[]);
    let finished = finish_output(&mut output, FILE, Bytes::new(vec![b'a'; BUDGET]));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out.len(), BUDGET);
    assert_eq!(output.budget(), 0);
}

#[test]
fn bytes_over_the_budget_are_refused_whole_and_no_flag_makes_them_smaller() {
    let mut output = output_with(BUDGET, &[]);
    let finished = finish_output(&mut output, FILE, Bytes::new(vec![b'a'; BUDGET + 1]));

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, b"");
    assert!(
        finished
            .err
            .starts_with("error: the result is larger than the output budget of 1024 bytes\n"),
        "{}",
        finished.err
    );
    assert!(
        finished
            .err
            .contains("no next step: no flag makes this answer smaller\n")
    );
}

#[test]
fn json_measures_the_base64_form_so_bytes_that_fit_as_text_can_be_refused() {
    // 900 bytes fit in the budget, and their base64 form of 1200 characters
    // does not.
    let binary = Bytes::new(vec![0xff; 900]);
    let mut text = output_with(BUDGET, &[]);
    let mut json = output_with(BUDGET, &["--json"]);

    let printed = finish_output(&mut text, FILE, binary.clone());
    let refused = finish_output(&mut json, FILE, binary);

    assert_eq!(printed.code, ExitCode::SUCCESS);
    assert_eq!(refused.code, ExitCode::FAILURE);
    let out = String::from_utf8(refused.out).unwrap();
    assert!(
        out.contains(r#""code":"crumbkit::output::over_budget""#),
        "{out}"
    );
}

#[test]
fn a_document_that_always_prints_its_first_answer_prints_bytes_past_the_budget() {
    let mut output = output_with(BUDGET, &[]);
    let big = Bytes::new(vec![b'a'; 2 * BUDGET]);
    let finished = finish_output(&mut output, FILE.always_print_first(), big);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out.len(), 2 * BUDGET);
}

/// A command that changes something and prints the bytes it would write.
#[derive(Parser)]
#[command(name = "tool")]
struct Write {
    #[command(flatten)]
    dry_run: DryRun,
    #[command(flatten)]
    output: Output,
}

#[test]
fn the_dry_run_line_of_bytes_goes_to_standard_error_so_the_bytes_stay_unchanged() {
    let mut write = Write::parse_from(["tool", "--dry-run"]);
    assert!(write.output.dry_run(write.dry_run));
    let finished = finish_output(&mut write.output, FILE, Bytes::from("Emma".to_owned()));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, b"Emma");
    assert_eq!(finished.err, "This was a dry run, so nothing changed.\n");
}

/// A row that holds a file's name and its bytes.
#[derive(Serialize, JsonSchema)]
struct File {
    /// The file's name.
    name: &'static str,
    /// The file's bytes.
    content: Bytes,
}

impl Row for File {
    const COLUMNS: &'static [&'static str] = &["name"];
}

#[test]
fn a_batch_prints_a_bytes_field_in_its_json_form_with_full() {
    let files: Document<Batch<File, Error>> =
        Document::new("files", "files", 1, "The bytes of each file.");
    let mut output = output_with(Output::DEFAULT_BUDGET, &["--json", "--full"]);
    let batch = answered(
        vec![File {
            name: "cover.png",
            content: Bytes::new(vec![0x89, 0x50]),
        }],
        Vec::<Error>::new(),
    );
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<_, Error> = Ok(batch);
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
        Spelling::COLONS,
        files,
        result,
    );

    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(
        String::from_utf8(out).unwrap(),
        concat!(
            r#"{"schema":"files/files/v1","data":[{"name":"cover.png","content":{"base64":"iVA="}}],"failures":[]}"#,
            "\n"
        )
    );
}
