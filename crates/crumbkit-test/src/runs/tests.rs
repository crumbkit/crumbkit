use serde_json::json;

use super::{documents, with_json};

#[test]
fn the_document_reader_skips_each_line_that_is_not_json_and_reads_the_documents_around_it() {
    let stdout = "plain text\n{\n  \"a\": 1\n}\n[ not json\n{\"b\":2}\n";

    assert_eq!(documents(stdout), vec![json!({"a": 1}), json!({"b": 2})]);
}

#[test]
fn json_goes_at_the_end_or_before_a_double_dash_and_is_not_added_twice() {
    let words = |line: &str| line.split(' ').map(str::to_owned).collect::<Vec<_>>();

    assert_eq!(with_json(&words("books list")), words("books list --json"));
    assert_eq!(
        with_json(&words("books add -- --title")),
        words("books add --json -- --title")
    );
    assert_eq!(
        with_json(&words("books list --json")),
        words("books list --json")
    );
}

#[cfg(unix)]
#[test]
fn a_run_that_never_reads_more_input_than_a_pipe_holds_is_stopped_at_its_timeout() {
    use std::time::Duration;

    use super::{Place, Ran, Xdg, run};

    let xdg = Xdg::new("runs-unread-input", None).unwrap();
    let input = vec![b'x'; 1024 * 1024];
    let place = Place {
        dir: None,
        stdin: Some(&input),
    };

    let ran = run(
        "/bin/sh",
        &[],
        &xdg,
        &["-c", "exec sleep 5"],
        place,
        Duration::from_millis(200),
    )
    .unwrap();

    assert!(matches!(ran, Ran::Stopped));
}
