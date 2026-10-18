//! Tests for the recovery check on a tool that prints its failures without
//! a crumbkit document: through a `FailureReader`, or as a `code:` line.

use crumbkit::FieldValue;
use crumbkit::form::{ReadFailure, Step};
use crumbkit::help::Guide;
use snapbox::Assert;
use snapbox::assert::Action;

use super::{Answer, CODES, OTHERS, SHOW_7, Setup, tool};
use crate::allow::{Allow, Rule};
use crate::examples::Examples;
use crate::failures::FailureReader;
use crate::recoveries::{Recoveries, check};
use crate::testing::panic_message;

/// A reader of the tool's failure line on standard error, such as
/// `ERR tool::books::not_found run=tool books list <path> path=/a`: the
/// code, then the call of the recovery, then one field.
struct LineReader;

impl FailureReader for LineReader {
    fn read(&self, _out: &str, err: &str) -> Vec<ReadFailure> {
        let Some(line) = err
            .lines()
            .last()
            .and_then(|line| line.strip_prefix("ERR "))
        else {
            return Vec::new();
        };
        let Some((code, rest)) = line.split_once(" run=") else {
            return Vec::new();
        };
        let Some((call, path)) = rest.split_once(" path=") else {
            return Vec::new();
        };
        vec![
            ReadFailure::new(code.to_owned(), vec![Step::Run(call.to_owned())])
                .with_fields(vec![("path".to_owned(), FieldValue::from(path.to_owned()))]),
        ]
    }
}

/// The opt-out that lets a recovery of `tool books show` fill a placeholder
/// from a field of its failure.
const FIELD_VALUE: &[Allow] = &[Allow::new(
    Rule::FieldValue,
    "books show",
    "The failure's path changes on each run.",
)];

/// Runs the check of the tool that gives `answers`, with the reader
/// `reader` and the opt-outs `allow`, and returns the panic message, or
/// `None` when it passes, and the report's directory.
fn checked(
    answers: &[Answer<'_>],
    reader: Option<&dyn FailureReader>,
    allow: &[Allow],
) -> (Option<String>, Setup) {
    let setup = Setup::new("recoveries-reading", answers);
    let mut examples = Examples::new(&setup.script, "").allow(allow);
    if let Some(reader) = reader {
        examples = examples.failure_reader(reader);
    }
    let recoveries = Recoveries::new(examples, CODES)
        .starts(SHOW_7)
        .unprovoked(OTHERS);
    let message = panic_message(|| {
        check(
            &tool(),
            &Guide::new(),
            &recoveries,
            &setup.golden(),
            &Assert::new().action(Action::Overwrite),
        );
    });
    (message, setup)
}

#[test]
fn the_recovery_check_follows_the_steps_and_fills_the_placeholders_that_a_failure_reader_returns() {
    let answers: &[Answer<'_>] = &[
        (
            "books show 7 --json",
            1,
            "",
            "ERR tool::books::not_found run=tool books list <path> path=/a",
        ),
        ("books list /a --json", 0, "[]", ""),
    ];

    let (message, setup) = checked(answers, Some(&LineReader), FIELD_VALUE);

    assert_eq!(message, None);
    assert!(
        setup.report().starts_with(
            "tool books show 7\n  fails with tool::books::not_found\n  run tool books list <path> as tool books list /a\n    <path> from the failure's field path\n    exits 0\n"
        ),
        "{}",
        setup.report()
    );
}

/// A reader of the JSON document that the tool prints on standard output in
/// place of crumbkit's, such as
/// `{"errors":[{"id":"tool::books::not_found","next":"tool books list"}]}`:
/// one entry for each failure, each with its code and the command line to
/// run next.
struct DocumentReader;

impl FailureReader for DocumentReader {
    fn read(&self, out: &str, _err: &str) -> Vec<ReadFailure> {
        let Ok(document) = serde_json::from_str::<serde_json::Value>(out) else {
            return Vec::new();
        };
        document["errors"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let code = entry["id"].as_str()?;
                let steps = entry["next"]
                    .as_str()
                    .map(|call| vec![Step::Run(call.to_owned())])
                    .unwrap_or_default();
                Some(ReadFailure::new(code.to_owned(), steps))
            })
            .collect()
    }
}

#[test]
fn the_recovery_check_follows_a_step_that_a_failure_reader_reads_from_the_tool_s_own_document() {
    let answers: &[Answer<'_>] = &[
        (
            "books show 7 --json",
            1,
            r#"{"errors":[{"id":"tool::books::not_found","next":"tool books list"}]}"#,
            "",
        ),
        ("books list --json", 0, "[]", ""),
    ];

    let (message, setup) = checked(answers, Some(&DocumentReader), &[]);

    assert_eq!(message, None);
    assert!(
        setup.report().starts_with(
            "tool books show 7\n  fails with tool::books::not_found\n  run tool books list\n    exits 0\n"
        ),
        "{}",
        setup.report()
    );
}

#[test]
fn a_failure_reader_that_reads_nothing_from_a_failed_run_stops_the_check_with_one_problem() {
    let answers: &[Answer<'_>] = &[("books show 7 --json", 1, "", "it broke")];

    let (message, _setup) = checked(answers, Some(&LineReader), &[]);

    assert_eq!(
        message.as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             tool books show 7: the start ends with exit status: 1, and the tool's \
             FailureReader reads no failure from it, so the check cannot follow the tool's \
             recoveries. The tool's FailureReader reads the failure, or the tool adds an \
             opt-out from `recoveries/follow` that names the root, with a reason"
        )
    );
}

#[test]
fn an_opt_out_that_names_the_root_skips_every_start_of_a_tool_whose_reader_reads_no_failure() {
    const ROOT: &[Allow] = &[Allow::new(
        Rule::Recoveries,
        "",
        "The tool prints its failures in a line that the reader cannot read.",
    )];
    let answers: &[Answer<'_>] = &[("books show 7 --json", 1, "", "it broke")];

    let (message, setup) = checked(answers, Some(&LineReader), ROOT);

    // The opt-out skips the start, so the start's code goes unprovoked.
    assert_eq!(
        message.as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             tool::books::not_found: no start provokes the declared code, and Recoveries::unprovoked does not list it"
        )
    );
    assert!(!setup.report().contains("tool books show 7"));
}

#[test]
fn the_recovery_check_reads_the_code_line_of_a_failed_run_that_prints_no_document() {
    let answers: &[Answer<'_>] = &[(
        "books show 7 --json",
        1,
        "",
        "error: no book 7\n  code:  tool::books::not_found\n",
    )];

    // A `code:` line gives a failure without steps, which
    // `recoveries/next-step` reports unless an opt-out covers it.
    let allow = [Allow::new(
        Rule::NextStep,
        "books show",
        "The tool prints its recoveries only as text.",
    )];

    let (message, setup) = checked(answers, None, &allow);

    assert_eq!(message, None);
    assert!(
        setup
            .report()
            .starts_with("tool books show 7\n  fails with tool::books::not_found\n\n"),
        "{}",
        setup.report()
    );
}
