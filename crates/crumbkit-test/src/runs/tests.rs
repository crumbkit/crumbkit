use serde_json::json;

use super::documents;

#[test]
fn the_document_reader_skips_each_line_that_is_not_json_and_reads_the_documents_around_it() {
    let stdout = "plain text\n{\n  \"a\": 1\n}\n[ not json\n{\"b\":2}\n";

    assert_eq!(documents(stdout), vec![json!({"a": 1}), json!({"b": 2})]);
}
