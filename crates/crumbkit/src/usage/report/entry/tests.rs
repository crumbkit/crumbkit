use serde_json::{Value, json};

use super::Entry;

/// Returns the line of a refused run of `books show` as the usage log writes
/// it, with every field that a refusal fills in.
fn refused() -> Value {
    json!({
        "schema": "crumbkit/usage/v1",
        "tool": "shelf",
        "version": "0.0.0",
        "at": "2026-10-07T12:01:07.120Z",
        "ms": 41,
        "caller": "claude-code",
        "session": "0123456789abcdef",
        "args": "fedcba9876543210",
        "trace": "4bf92f3577b34da6a3ce929d0e0e4736",
        "run": "00f067aa0ba902b7",
        "command": "books show",
        "unknown_flags": 1,
        "nearest": "--from",
        "exit": 2,
        "code": "crumbkit::cli::usage",
    })
}

/// Returns `value` as the bytes of one line of the log.
fn bytes(value: &Value) -> Vec<u8> {
    let mut line = serde_json::to_vec(value).unwrap();
    line.push(b'\n');
    line
}

#[test]
fn a_line_of_the_log_reads_as_an_entry_with_its_end_time_in_milliseconds() {
    let entry = Entry::parse(&bytes(&refused())).unwrap();

    assert_eq!(entry.at, 1_791_374_467_120);
    assert_eq!(entry.ms, 41);
    assert_eq!(entry.caller, "claude-code");
    assert_eq!(entry.session, "0123456789abcdef");
    assert_eq!(entry.args, "fedcba9876543210");
    assert_eq!(entry.command.as_deref(), Some("books show"));
    assert_eq!(entry.nearest.as_deref(), Some("--from"));
    assert_eq!(entry.exit, Some(2));
    assert_eq!(entry.code.as_deref(), Some("crumbkit::cli::usage"));
    assert!(!entry.help);
    assert_eq!(entry.flags, Vec::<String>::new());
}

#[test]
fn the_fields_that_a_run_left_out_read_as_absent() {
    let mut line = refused();
    let object = line.as_object_mut().unwrap();
    for field in ["command", "unknown_flags", "nearest", "exit", "code"] {
        object.remove(field);
    }
    object.insert("help".to_owned(), json!("short"));

    let entry = Entry::parse(&bytes(&line)).unwrap();

    assert!(entry.help);
    assert_eq!(entry.command, None);
    assert_eq!(entry.exit, None);
    assert_eq!(entry.ids, None);
    assert_eq!(entry.out_bytes, None);
    assert!(!entry.truncated);
    assert!(!entry.interrupted);
}

#[test]
fn a_field_that_the_reader_does_not_know_is_ignored() {
    let mut line = refused();
    line["added_later"] = json!({"nested": [1, 2]});

    assert!(Entry::parse(&bytes(&line)).is_some());
}

#[test]
fn a_line_without_a_field_that_every_record_has_is_skipped() {
    for field in ["schema", "at", "ms", "caller", "session", "args"] {
        let mut line = refused();
        line.as_object_mut().unwrap().remove(field);

        assert!(Entry::parse(&bytes(&line)).is_none(), "{field}");
    }
}

#[test]
fn a_line_of_another_schema_is_skipped() {
    let mut line = refused();
    line["schema"] = json!("crumbkit/usage/v2");

    assert!(Entry::parse(&bytes(&line)).is_none());
}

#[test]
fn a_line_whose_end_time_is_not_rfc_3339_is_skipped() {
    let mut line = refused();
    line["at"] = json!("7 October 2026");

    assert!(Entry::parse(&bytes(&line)).is_none());
}

#[test]
fn a_line_that_is_not_json_or_was_cut_short_is_skipped() {
    let whole = bytes(&refused());

    assert!(Entry::parse(b"not json\n").is_none());
    assert!(Entry::parse(&whole[..whole.len() / 2]).is_none());
    assert!(Entry::parse(&[0xff, 0xfe, b'\n']).is_none());
}
