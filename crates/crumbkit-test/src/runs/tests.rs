use serde_json::json;

use super::{documents, failure};

#[test]
fn the_failure_reader_reads_an_indented_failure_document() {
    let stdout =
        b"{\n  \"schema\": \"crumbkit/failure/v1\",\n  \"code\": \"shelf::books::not_found\"\n}\n";

    let read = failure(stdout).expect("a failure");

    assert_eq!(read["code"], "shelf::books::not_found");
}

#[test]
fn the_failure_reader_reads_the_last_failure_line_of_a_stream() {
    let stdout = b"{\"schema\":\"shelf/books/v1\",\"data\":{}}\n{\"schema\":\"crumbkit/failure/v1\",\"code\":\"a\"}\n{\"schema\":\"crumbkit/failure/v1\",\"code\":\"b\"}\n";

    assert_eq!(failure(stdout).expect("a failure")["code"], "b");
}

#[test]
fn the_document_reader_skips_each_line_that_is_not_json_and_reads_the_documents_around_it() {
    let stdout = "plain text\n{\n  \"a\": 1\n}\n[ not json\n{\"b\":2}\n";

    assert_eq!(documents(stdout), vec![json!({"a": 1}), json!({"b": 2})]);
}
