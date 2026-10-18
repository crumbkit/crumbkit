//! Tests for how a headed batch whose answers pass the output budget prints
//! in text and with `--json`: its first answer, or that answer's own
//! failure when the document does not always print it and it passes the
//! budget alone, then one failure that names the answers left out
//! (ADR-0046, ADR-0062, ADR-0084).

use std::process::ExitCode;

use clap::CommandFactory as _;
use serde_json::{Value, json};

use super::{Finished, PAGES, Page, batch, finish, page};
use crate::help::Guide;
use crate::output::tests::{Tool, built_with};
use crate::{Batch, Document, Error, Sink, finish_to};

/// The keys of the batch, in the order the caller gave them.
const KEYS: [&str; 3] = ["A", "B", "C"];

/// Returns an answer of 100 bytes, so its header and bytes take 110 bytes.
fn answer() -> Vec<u8> {
    let mut bytes = vec![b'x'; 99];
    bytes.push(b'\n');
    bytes
}

/// The budgets that each test runs at in text: 0, which a guide may set,
/// 1, a budget between one header and one answer, and a budget that fits
/// the first answer alone, with its header.
const TEXT_BUDGETS: [usize; 4] = [0, 1, 50, 150];

/// The budgets that each test runs at with `--json`, the last of which fits
/// the document with the first answer alone.
const JSON_BUDGETS: [usize; 4] = [0, 1, 50, 200];

/// The budget below which the first answer alone passes the budget in each
/// form: its header and bytes in text, and the document that holds it with
/// `--json`.
const fn alone(json: bool) -> usize {
    if json { 200 } else { 150 }
}

/// Fetches each of [`KEYS`], while `Room::has_room` allows it when
/// `through_room` is set, as a handler does, then finishes the batch as
/// `document` with `flags` and a budget of `budget`.
fn fetched(
    flags: &[&str],
    budget: usize,
    document: Document<Batch<Page, Error>>,
    through_room: bool,
) -> Finished {
    let guide = Guide::new().output_budget(budget, "the tests print little");
    let args: Vec<&str> = ["tool", "list"].iter().chain(flags).copied().collect();
    let mut output = built_with(Tool::command(), &guide, &args);
    let mut room = output.room(document);
    let mut answers = Batch::new(&["show"], "names", &KEYS);
    for key in KEYS {
        if through_room && !room.has_room(&answers) {
            break;
        }
        answers.push(Ok(page(key, &answer())));
    }
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<_, Error> = Ok(answers);
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
        document,
        result,
    );
    Finished {
        code: ExitCode::from(code),
        out,
        err: String::from_utf8(err).unwrap(),
    }
}

/// Returns the words that name a budget of `budget` bytes.
fn bytes(budget: usize) -> String {
    if budget == 1 {
        "1 byte".to_owned()
    } else {
        format!("{budget} bytes")
    }
}

/// Returns how the text of `finished` differs from A printed whole, or A
/// refused with its own failure when `refused` is set, and B and C named in
/// one over-budget failure, or nothing when it does not.
fn text_problem(finished: &Finished, budget: usize, refused: bool) -> Option<String> {
    let bytes = bytes(budget);
    let (expected_out, refusal) = if refused {
        let refusal = format!(
            "\
error: the result is larger than the output budget of {bytes}
  key:   A (index 0)
  help:  no next step: no flag makes this answer smaller
  code:  tool::output::over_budget
"
        );
        (String::new(), refusal)
    } else {
        let out = format!("==> A <==\n{}", String::from_utf8(answer()).unwrap());
        (out, String::new())
    };
    let expected_err = format!(
        "\
{refusal}error: 2 more answers did not fit in the output budget of {bytes}
  help:  run `tool show B C`
  code:  tool::output::over_budget
"
    );
    let ok = finished.code == ExitCode::FAILURE
        && finished.text() == expected_out
        && finished.err == expected_err;
    (!ok).then(|| {
        format!(
            "text at budget {budget}: exit {:?}\nstdout:\n{}\nstderr:\n{}",
            finished.code,
            finished.text(),
            finished.err
        )
    })
}

/// Returns how the JSON document of `finished` differs from A in `data`,
/// or A refused with its own failure when `refused` is set, and one
/// over-budget failure that names B and C, or nothing when it does not.
fn json_problem(finished: &Finished, budget: usize, refused: bool) -> Option<String> {
    let document: Value = serde_json::from_slice(&finished.out).unwrap();
    let text = String::from_utf8(answer()).unwrap();
    let mut failures = document["failures"].as_array().cloned().unwrap_or_default();
    let (data, refusal) = if refused {
        let refusal = json!({
            "message": format!("the result is larger than the output budget of {}", bytes(budget)),
            "key": "A",
            "index": 0,
            "fields": {"budget": budget},
            "recovery": "none",
        });
        (json!([]), Some(refusal))
    } else {
        (json!([{"name": "A", "text": text}]), None)
    };
    let found = refusal.as_ref().map(|_| {
        let first = failures.remove(0);
        json!({
            "message": first["message"],
            "key": first["key"],
            "index": first["index"],
            "fields": first["fields"],
            "recovery": first["recovery"]["kind"],
        })
    });
    let ok = finished.code == ExitCode::FAILURE
        && document["data"] == data
        && found == refusal
        && failures.len() == 1
        && failures[0]["message"]
            == format!(
                "2 more answers did not fit in the output budget of {}",
                bytes(budget)
            )
        && failures[0]["fields"]
            == json!({"left_out": ["B", "C"], "left_out_from": 1, "budget": budget})
        && failures[0]["recovery"]["kind"] == "run"
        && failures[0]["recovery"]["all"] == "left_out";
    (!ok).then(|| {
        format!(
            "JSON at budget {budget}: exit {:?}\n{}",
            finished.code,
            finished.text()
        )
    })
}

/// Fetches and finishes the batch as `document` at each of
/// [`TEXT_BUDGETS`] in text and each of [`JSON_BUDGETS`] with `--json`,
/// with every key fetched and with the keys that `Room` allows, and returns
/// every way that a run differs from the expected run: A printed, or
/// refused when the document does not always print its first answer and A
/// alone passes the budget, then B and C left out.
fn problems(document: Document<Batch<Page, Error>>) -> Vec<String> {
    let mut problems = Vec::new();
    for through_room in [false, true] {
        for budget in TEXT_BUDGETS {
            let refused = !document.always_prints_first() && budget < alone(false);
            let text = fetched(&[], budget, document, through_room);
            problems.extend(text_problem(&text, budget, refused));
        }
        for budget in JSON_BUDGETS {
            let refused = !document.always_prints_first() && budget < alone(true);
            let json = fetched(&["--json"], budget, document, through_room);
            problems.extend(json_problem(&json, budget, refused));
        }
        for problem in &mut problems {
            if !problem.starts_with("through Room") {
                *problem = format!("through Room: {through_room}: {problem}");
            }
        }
    }
    problems
}

#[test]
fn a_headed_batch_that_always_prints_its_first_answer_prints_it_in_text_and_json_at_every_budget() {
    let problems = problems(PAGES.always_print_first());

    assert!(problems.is_empty(), "{}", problems.join("\n\n"));
}

#[test]
fn a_headed_batch_without_the_mark_refuses_a_first_answer_alone_past_the_budget_and_leaves_out_the_rest_in_text_and_json()
 {
    let problems = problems(PAGES);

    assert!(problems.is_empty(), "{}", problems.join("\n\n"));
}

#[test]
fn a_later_answer_too_large_alone_is_left_out_with_the_answers_after_it_in_text_and_json() {
    let big = vec![b'x'; 300];
    let answers = [
        ("A", Some(&b"one\n"[..])),
        ("B", Some(&big[..])),
        ("C", Some(b"three\n")),
    ];
    for document in [PAGES, PAGES.always_print_first()] {
        let label = format!("always prints first: {}", document.always_prints_first());
        let text = finish(&[], 100, document, batch(&answers));
        let json = finish(&["--json"], 100, document, batch(&answers));

        assert_eq!(text.code, ExitCode::FAILURE, "{label}");
        assert_eq!(text.text(), "==> A <==\none\n", "{label}");
        assert_eq!(
            text.err,
            "\
error: 2 more answers did not fit in the output budget of 100 bytes
  help:  run `tool show B C`
  code:  tool::output::over_budget
",
            "{label}"
        );
        assert_eq!(json.code, ExitCode::FAILURE, "{label}");
        let parsed: Value = serde_json::from_slice(&json.out).unwrap();
        assert_eq!(
            parsed["data"],
            json!([{"name": "A", "text": "one\n"}]),
            "{label}"
        );
        let failures = parsed["failures"].as_array().unwrap();
        assert_eq!(failures.len(), 1, "{label}: {failures:?}");
        assert_eq!(
            failures[0]["fields"],
            json!({"left_out": ["B", "C"], "left_out_from": 1, "budget": 100}),
            "{label}"
        );
    }
}
