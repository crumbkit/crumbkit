//! Tests for how `finish_to` prints a batch whose document is headed: each
//! answer's bytes under a `==> KEY <==` header in text, the ordinary batch
//! with `--json`, the output budget and the view flags (ADR-0062).

use std::process::ExitCode;

use clap::{CommandFactory as _, FromArgMatches as _};
use schemars::JsonSchema;
use serde::Serialize;

use super::Tool;
use crate::help::Guide;
use crate::{
    Batch, Bytes, Document, Error, Headed, Output, Parsed, Row, Sink, Spelling, finish_to,
};

/// The smallest budget that a tool can set.
const BUDGET: usize = 1024;

/// One file's answer: its name, and its bytes unless the answer holds none.
#[derive(Serialize, JsonSchema)]
struct Page {
    /// The file's name.
    name: String,
    /// The file's bytes, or none when the answer holds a record instead.
    #[serde(flatten)]
    content: Option<Bytes>,
}

impl Row for Page {
    const COLUMNS: &'static [&'static str] = &["name"];
}

impl Headed for Page {
    fn bytes(&self) -> Option<&Bytes> {
        self.content.as_ref()
    }
}

const PAGES: Document<Batch<Page, Error>> =
    Document::new("files", "pages", 1, "The test pages.").headed();

/// What `finish_to` returned and wrote. Standard output is bytes, because a
/// page need not be UTF-8.
struct Finished {
    code: ExitCode,
    out: Vec<u8>,
    err: String,
}

impl Finished {
    /// Returns standard output as text.
    fn text(&self) -> &str {
        std::str::from_utf8(&self.out).unwrap()
    }
}

/// Returns the page `name` that holds `content`.
fn page(name: &str, content: &[u8]) -> Page {
    Page {
        name: name.to_owned(),
        content: Some(Bytes::from(content.to_vec())),
    }
}

/// Returns the answer for `key`: a page whose content is `content`, or the
/// failure that no page has that name when `content` is `None`.
fn answer(key: &str, content: Option<&[u8]>) -> Result<Page, Error> {
    content
        .map(|content| page(key, content))
        .ok_or_else(|| Error::unclassified(format!("no page is named {key}")))
}

/// Returns a batch of `tool show` with one answer for each of `answers`.
fn batch(answers: &[(&str, Option<&[u8]>)]) -> Batch<Page, Error> {
    let keys: Vec<&str> = answers.iter().map(|(key, _)| *key).collect();
    let mut batch = Batch::new(&["show"], "names", &keys);
    batch.extend(answers.iter().map(|(key, content)| answer(key, *content)));
    batch
}

/// Finishes `batch` as `document` with `flags` and a budget of `budget`.
fn finish(
    flags: &[&str],
    budget: usize,
    document: Document<Batch<Page, Error>>,
    batch: Batch<Page, Error>,
) -> Finished {
    let mut command = Tool::command();
    let args = ["tool", "list"].iter().chain(flags);
    let matches = command.try_get_matches_from_mut(args).unwrap();
    let mut output = Output::from_arg_matches(&matches).unwrap();
    let guide = Guide::new().output_budget(budget, "the tests print little");
    output.set_command(Parsed::guided(command, &guide));
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<_, Error> = Ok(batch);
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
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

/// Finishes `batch` as [`PAGES`] with `flags` and a large budget.
fn finish_pages(flags: &[&str], batch: Batch<Page, Error>) -> Finished {
    finish(flags, 1 << 20, PAGES, batch)
}

#[test]
fn a_headed_batch_prints_each_answer_s_bytes_under_a_header_with_a_blank_line_between() {
    let finished = finish_pages(&[], batch(&[("a", Some(b"one\n")), ("b", Some(b"two\n"))]));

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.text(), "==> a <==\none\n\n==> b <==\ntwo\n");
    assert_eq!(finished.err, "");
}

#[test]
fn a_headed_batch_of_one_key_prints_its_bytes_without_a_header() {
    let finished = finish_pages(&[], batch(&[("a", Some(b"one\n"))]));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.text(), "one\n");
}

#[test]
fn a_headed_batch_prints_bytes_that_are_not_utf8_unchanged() {
    let finished = finish_pages(&[], batch(&[("a", Some(b"caf\xe9\n"))]));

    assert_eq!(finished.out, b"caf\xe9\n");
}

#[test]
fn a_failed_key_gets_no_header_and_its_failure_goes_to_standard_error() {
    let answers = [
        ("a", Some(&b"one\n"[..])),
        ("gone", None),
        ("b", Some(b"two\n")),
    ];
    let finished = finish_pages(&[], batch(&answers));

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.text(), "==> a <==\none\n\n==> b <==\ntwo\n");
    assert!(
        finished.err.starts_with("error: no page is named gone\n"),
        "{}",
        finished.err
    );
    assert!(
        finished.err.contains("key:   gone (index 1)\n"),
        "{}",
        finished.err
    );
}

#[test]
fn an_answer_without_bytes_prints_as_one_line_of_compact_json_under_its_header() {
    let mut answers = Batch::new(&["show"], "names", &["a", "b"]);
    answers.push(Ok(Page {
        name: "a".to_owned(),
        content: None,
    }));
    answers.push(Ok(page("b", b"two\n")));
    let finished = finish_pages(&[], answers);

    assert_eq!(
        finished.text(),
        "==> a <==\n{\"name\":\"a\"}\n\n==> b <==\ntwo\n"
    );
}

#[test]
fn a_header_escapes_the_control_characters_of_its_key() {
    let finished = finish_pages(
        &[],
        batch(&[("a\tb", Some(b"one\n")), ("c", Some(b"two\n"))]),
    );

    assert!(
        finished.text().starts_with("==> a\\tb <==\n"),
        "{}",
        finished.text()
    );
}

#[test]
fn an_answer_larger_than_the_budget_fails_on_its_own_and_the_later_answers_still_print() {
    let big = vec![b'x'; 2000];
    let answers = [("a", Some(&big[..])), ("b", Some(b"two\n"))];
    let finished = finish(&[], BUDGET, PAGES, batch(&answers));

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.text(), "==> b <==\ntwo\n");
    assert_eq!(
        finished.err,
        "\
error: the result is larger than the output budget of 1024 bytes
  key:   a (index 0)
  help:  no next step: no flag makes this answer smaller
  code:  crumbkit::output::over_budget
"
    );
}

#[test]
fn a_document_that_always_prints_its_first_answer_prints_a_first_answer_larger_than_the_budget() {
    let big = vec![b'x'; 2000];
    let finished = finish(
        &[],
        BUDGET,
        PAGES.always_print_first(),
        batch(&[("a", Some(&big[..]))]),
    );

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.out, big);
}

#[test]
fn the_answers_past_the_budget_are_left_out_whole_and_named_in_one_failure() {
    let half = vec![b'x'; 400];
    let names = ["a", "b", "c", "d"];
    let answers: Vec<(&str, Option<&[u8]>)> =
        names.iter().map(|name| (*name, Some(&half[..]))).collect();
    let finished = finish(&[], BUDGET, PAGES, batch(&answers));

    assert_eq!(finished.code, ExitCode::FAILURE);
    let header = |name: &str| format!("==> {name} <==\n");
    let expected = [
        header("a").into_bytes(),
        half.clone(),
        b"\n".to_vec(),
        header("b").into_bytes(),
        half.clone(),
    ]
    .concat();
    assert_eq!(finished.out, expected);
    assert_eq!(
        finished.err,
        "\
error: 2 more answers did not fit in the output budget of 1024 bytes
  help:  run `tool show c d`
  code:  crumbkit::output::over_budget
"
    );
}

#[test]
fn fields_without_json_is_refused_on_a_headed_document() {
    let fields = finish_pages(&["--fields", "name"], batch(&[("a", Some(b"one\n"))]));
    let json = finish_pages(
        &["--json", "--fields", "name"],
        batch(&[("a", Some(b"one\n"))]),
    );

    assert_eq!(fields.code, ExitCode::from(2));
    assert_eq!(fields.text(), "");
    assert!(
        fields.err.starts_with(
            "error: --fields selects the fields of JSON rows, and this command's text prints each answer's bytes\n  help:  add --json, or drop --fields\n"
        ),
        "{}",
        fields.err
    );
    assert_eq!(json.code, ExitCode::SUCCESS, "{}", json.err);
}

#[test]
fn long_and_full_are_refused_on_a_headed_document() {
    let long = finish_pages(&["--json", "--long"], batch(&[("a", Some(b"one\n"))]));
    let full = finish_pages(&["--json", "--full"], batch(&[("a", Some(b"one\n"))]));

    assert_eq!(long.code, ExitCode::from(2));
    assert!(
        long.text().contains(r#""message":"--long adds the long columns of a table, and this command's text prints each answer's bytes""#),
        "{}",
        long.text()
    );
    assert_eq!(full.code, ExitCode::from(2));
    assert!(
        full.text().contains(r#""message":"--full prints every field of every row, and this command's JSON already holds every field""#),
        "{}",
        full.text()
    );
}

#[test]
fn json_prints_every_field_of_a_headed_batch_with_the_bytes_at_the_top_level_of_each_row() {
    let answers = [("a", Some(&b"one\n"[..])), ("b", Some(b"caf\xe9"))];
    let every = finish_pages(&["--json"], batch(&answers));
    let picked = finish_pages(&["--json", "--fields", "name,text"], batch(&answers));

    let data = |finished: &Finished| -> serde_json::Value {
        serde_json::from_slice::<serde_json::Value>(&finished.out).unwrap()["data"].clone()
    };
    assert_eq!(
        data(&every),
        serde_json::json!([
            {"name": "a", "text": "one\n"},
            {"name": "b", "base64": "Y2Fm6Q=="},
        ])
    );
    assert_eq!(
        data(&picked),
        serde_json::json!([
            {"name": "a", "text": "one\n"},
            {"name": "b", "text": null},
        ])
    );
}
