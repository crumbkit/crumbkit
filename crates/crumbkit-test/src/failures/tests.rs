use crumbkit::form::{ReadFailure, Step};

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
fn crumbkit_s_document_gives_each_failure_with_its_code_steps_and_fields() {
    let stdout = br#"{"schema":"crumbkit/failure/v1","code":"a","message":"m","fields":{"path":"/a","id":7,"kept":true,"tags":["x"]},"recovery":{"kind":"run","call":"tool books list"},"alternatives":[{"kind":"retry"}]}"#;

    let read = read_failures(None, stdout, b"");

    assert_eq!(
        read,
        [ReadFailure::new(
            "a".to_owned(),
            vec![Step::Run("tool books list".to_owned()), Step::Retry],
        )
        .with_fields(vec![
            ("id".to_owned(), "7".to_owned()),
            ("kept".to_owned(), "true".to_owned()),
            ("path".to_owned(), "/a".to_owned()),
        ])]
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
        read_failures(None, stdout, b"")
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

    let codes: Vec<String> = read_failures(None, stdout, b"")
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
    let reader = Reader::Own(&ErrReader);

    let first = |read: Vec<ReadFailure>| read.first().map(|failure| failure.code().to_owned());

    assert_eq!(
        first(read_failures(Some(reader), document, stderr)).as_deref(),
        Some("reader")
    );
    assert_eq!(
        first(read_failures(Some(reader), document, b"  code:  line\n")).as_deref(),
        Some("document")
    );
    assert_eq!(
        first(read_failures(None, b"", stderr)).as_deref(),
        Some("line")
    );
    assert_eq!(first(read_failures(None, b"", b"plain\n")), None);
}
