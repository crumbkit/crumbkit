use crumbkit::form::{ReadFailure, Step};
use crumbkit::{FieldValue, ToolName};

use super::{FailureReader, Reader, read_failures};

/// Returns a failure with `code`, no steps and no fields.
fn bare(code: &str) -> ReadFailure {
    ReadFailure::new(code.to_owned(), Vec::new())
}

/// A reader that reads a line `ERR` and a code from standard error.
struct ErrReader;

impl FailureReader for ErrReader {
    fn read(&self, _out: &str, err: &str) -> Vec<ReadFailure> {
        err.lines()
            .filter_map(|line| line.strip_prefix("ERR "))
            .map(bare)
            .collect()
    }
}

#[test]
fn crumbkit_s_document_gives_each_failure_with_its_code_steps_and_every_field_value() {
    let stdout = br#"{"schema":"crumbkit/failure/v1","code":"a","message":"m","fields":{"path":"/a","id":7,"kept":true,"tags":["x"],"found":null,"lines":{"a":1}},"recovery":{"kind":"run","call":"tool books list"},"alternatives":[{"kind":"retry"}]}"#;

    let read = read_failures(ToolName::CRUMBKIT, None, stdout, b"");

    assert_eq!(
        read,
        [ReadFailure::new(
            "a".to_owned(),
            vec![Step::Run("tool books list".to_owned()), Step::Retry],
        )
        .with_fields(vec![
            ("found".to_owned(), FieldValue::Null),
            ("id".to_owned(), FieldValue::Unsigned(7)),
            ("kept".to_owned(), FieldValue::Boolean(true)),
            (
                "lines".to_owned(),
                FieldValue::object([("a", FieldValue::Unsigned(1))]),
            ),
            ("path".to_owned(), FieldValue::from("/a".to_owned())),
            ("tags".to_owned(), FieldValue::list(["x".to_owned()])),
        ])]
    );
}

#[test]
fn a_field_of_crumbkit_s_document_that_holds_a_fraction_is_left_out() {
    let stdout = br#"{"schema":"crumbkit/failure/v1","code":"a","fields":{"ratio":0.5,"id":7},"recovery":{"kind":"retry"}}"#;

    let read = read_failures(ToolName::CRUMBKIT, None, stdout, b"");

    assert_eq!(
        read,
        [ReadFailure::new("a".to_owned(), vec![Step::Retry])
            .with_fields(vec![("id".to_owned(), FieldValue::Unsigned(7))])]
    );
}

#[test]
fn crumbkit_s_document_is_read_indented_and_the_last_one_of_a_stream_wins() {
    let indented = b"{\n  \"schema\": \"crumbkit/failure/v1\",\n  \"code\": \"a\",\n  \"recovery\": {\"kind\": \"retry\"}\n}\n";
    let stream = br#"{"schema":"shelf/books/v1","data":{}}
{"schema":"crumbkit/failure/v1","code":"a","recovery":{"kind":"retry"}}
{"schema":"crumbkit/failure/v1","code":"b","recovery":{"kind":"retry"}}
"#;

    let codes = |stdout: &[u8]| -> Vec<String> {
        read_failures(ToolName::CRUMBKIT, None, stdout, b"")
            .iter()
            .map(|failure| failure.code().to_owned())
            .collect()
    };

    assert_eq!(codes(indented), ["a"]);
    assert_eq!(codes(stream), ["b"]);
}

#[test]
fn every_entry_of_a_batch_is_read_and_the_first_entry_comes_first() {
    let stdout = br#"{"schema":"crumbkit/batch/v1","failures":[{"code":"a","recovery":{"kind":"retry"}},{"code":"b","recovery":{"kind":"retry"}}]}"#;

    let codes: Vec<String> = read_failures(ToolName::CRUMBKIT, None, stdout, b"")
        .iter()
        .map(|failure| failure.code().to_owned())
        .collect();

    assert_eq!(codes, ["a", "b"]);
}

#[test]
fn the_reader_wins_over_crumbkit_s_document_and_the_document_wins_over_the_code_line() {
    let document =
        br#"{"schema":"crumbkit/failure/v1","code":"document","recovery":{"kind":"retry"}}"#;
    let stderr = b"ERR reader\n  code:  line\n";
    let reader = Reader(&ErrReader);

    let first = |read: Vec<ReadFailure>| read.first().map(|failure| failure.code().to_owned());

    assert_eq!(
        first(read_failures(
            ToolName::CRUMBKIT,
            Some(reader),
            document,
            stderr
        ))
        .as_deref(),
        Some("reader")
    );
    assert_eq!(
        first(read_failures(
            ToolName::CRUMBKIT,
            Some(reader),
            document,
            b"  code:  line\n"
        ))
        .as_deref(),
        Some("document")
    );
    assert_eq!(
        first(read_failures(ToolName::CRUMBKIT, None, b"", stderr)).as_deref(),
        Some("line")
    );
    assert_eq!(
        first(read_failures(ToolName::CRUMBKIT, None, b"", b"plain\n")),
        None
    );
}

#[test]
fn a_failure_document_with_the_tools_own_schema_id_gives_its_failure() {
    let stdout =
        br#"{"schema":"shelf/failure/v1","code":"shelf::cli::usage","recovery":{"kind":"retry"}}"#;

    let read = read_failures(ToolName::new("shelf"), None, stdout, b"");

    assert_eq!(
        read,
        [ReadFailure::new(
            "shelf::cli::usage".to_owned(),
            vec![Step::Retry]
        )]
    );
}

#[test]
fn a_failure_document_with_another_tools_schema_id_is_no_failure() {
    let stdout = br#"{"schema":"crumbkit/failure/v1","code":"shelf::cli::usage","recovery":{"kind":"retry"}}"#;

    assert_eq!(read_failures(ToolName::new("shelf"), None, stdout, b""), []);
}
