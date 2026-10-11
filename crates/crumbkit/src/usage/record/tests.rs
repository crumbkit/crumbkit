use serde_json::{Value, json};

use super::{Hash, Help, Name, Record, Run, Section, SpanId, TraceId, TraceIds, line};

const KEY: [u8; 16] = *b"0123456789abcdef";

/// Returns the name `text`, as one of `Name`'s constructors reads it from a
/// tool's command tree or codes.
fn name(text: &str) -> Name {
    Name(text.to_owned())
}

/// Returns the record of a run that reported nothing, as the usage log
/// writes it for a tool whose `main` returned before `parse`.
fn bare() -> Record {
    Record {
        schema: super::SCHEMA,
        tool: name("shelf"),
        version: Some(name("0.0.0")),
        at: "2026-10-07T12:01:07.120Z".to_owned(),
        ms: 41,
        caller: "person",
        session: Hash::new(&KEY, [b"4242".as_slice()]),
        args: Hash::new(&KEY, []),
        trace: TraceIds {
            trace: TraceId::new([0x4b; 16]),
            run: SpanId::new([0x22; 8]),
            parent: None,
        },
        interrupted: false,
        run: Run::default(),
    }
}

/// Returns the record as the JSON object of its line.
fn parsed(record: &Record) -> Value {
    serde_json::from_slice(&line(record).unwrap()).unwrap()
}

#[test]
fn a_record_leaves_out_every_field_that_does_not_apply() {
    let record = parsed(&bare());

    let mut fields: Vec<&String> = record.as_object().unwrap().keys().collect();
    fields.sort();
    assert_eq!(
        fields,
        [
            "args", "at", "caller", "ms", "run", "schema", "session", "tool", "trace", "version"
        ]
    );
    assert_eq!(record["schema"], "crumbkit/usage/v1");
}

#[test]
fn a_record_writes_every_field_of_the_run_beside_its_own() {
    let mut record = bare();
    record.trace.parent = Some(SpanId::new([
        0x00, 0xf0, 0x67, 0xaa, 0x0b, 0xa9, 0x02, 0xb7,
    ]));
    record.interrupted = true;
    record.run = Run {
        command: Some(name("books show")),
        help: Some(Help::Long),
        flags: vec![name("--json")],
        ids: Some(3),
        unknown_flags: Some(1),
        nearest: Some(name("--fields")),
        alias: true,
        exit: Some(1),
        code: Some(name("shelf::books::not_found")),
        out_bytes: Some(1840),
        truncated: true,
        ext: Some(Section {
            schema: "shelf/usage-ext/v1".to_owned(),
            data: json!({"requests": 2}),
        }),
    };

    let record = parsed(&record);

    assert_eq!(record["trace"], "4b4b4b4b4b4b4b4b4b4b4b4b4b4b4b4b");
    assert_eq!(record["run"], "2222222222222222");
    assert_eq!(record["parent"], "00f067aa0ba902b7");
    assert_eq!(record["command"], "books show");
    assert_eq!(record["help"], "long");
    assert_eq!(record["flags"], json!(["--json"]));
    assert_eq!(record["ids"], 3);
    assert_eq!(record["unknown_flags"], 1);
    assert_eq!(record["nearest"], "--fields");
    assert_eq!(record["alias"], true);
    assert_eq!(record["exit"], 1);
    assert_eq!(record["code"], "shelf::books::not_found");
    assert_eq!(record["out_bytes"], 1840);
    assert_eq!(record["truncated"], true);
    assert_eq!(record["interrupted"], true);
    assert_eq!(
        record["ext"],
        json!({"schema": "shelf/usage-ext/v1", "data": {"requests": 2}})
    );
}

#[test]
fn a_hash_is_sixteen_lowercase_hex_digits_that_change_with_the_key() {
    let hash = Hash::new(&KEY, [b"books".as_slice(), b"list".as_slice()]);
    let other_key = Hash::new(
        b"fedcba9876543210",
        [b"books".as_slice(), b"list".as_slice()],
    );

    let text = serde_json::to_value(&hash).unwrap();
    let text = text.as_str().unwrap();
    assert_eq!(text.len(), 16);
    assert!(
        text.bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    );
    assert_ne!(serde_json::to_value(&other_key).unwrap(), text);
}

#[test]
fn a_hash_tells_where_one_word_ends_and_the_next_begins() {
    let split = Hash::new(&KEY, [b"ab".as_slice(), b"c".as_slice()]);
    let joined = Hash::new(&KEY, [b"a".as_slice(), b"bc".as_slice()]);

    assert_ne!(
        serde_json::to_value(&split).unwrap(),
        serde_json::to_value(&joined).unwrap()
    );
}

#[test]
fn a_line_ends_with_a_newline_and_a_line_over_1_kib_is_dropped() {
    let mut record = bare();
    let fits = line(&record).unwrap();
    assert_eq!(fits.last(), Some(&b'\n'));
    // The section's text fills the line to exactly 1 KiB, and one more byte
    // makes it too long.
    let room = super::MAX_LINE - fits.len() - r#","ext":{"schema":"a/b/v1","data":""}"#.len() + 1;
    record.run.ext = Some(Section {
        schema: "a/b/v1".to_owned(),
        data: Value::String("x".repeat(room)),
    });
    assert_eq!(line(&record).unwrap().len(), super::MAX_LINE + 1);

    record.run.ext = Some(Section {
        schema: "a/b/v1".to_owned(),
        data: Value::String("x".repeat(room + 1)),
    });
    assert_eq!(line(&record), None);
}

#[test]
fn the_checked_in_schema_matches_the_usage_record_schema() {
    let generated = schemars::schema_for!(Record);
    let generated = serde_json::to_string_pretty(&generated).unwrap() + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/usage-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/usage-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}
