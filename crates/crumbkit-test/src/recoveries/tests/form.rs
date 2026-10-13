//! Tests for the recovery check on a tool that prints its failures in its
//! own failure form, which the check reads through `FailureForm::read`.

use crumbkit::Document;
use crumbkit::form::{FailureForm, PreparedFailure, ReadFailure, Step};
use crumbkit::help::Guide;
use schemars::{Schema, json_schema};
use snapbox::Assert;
use snapbox::assert::Action;

use super::{Answer, CODES, OTHERS, SHOW_7, Setup, tool};
use crate::allow::{Allow, Rule};
use crate::examples::Examples;
use crate::recoveries::{Recoveries, check};
use crate::testing::panic_message;

/// A failure form whose document is `tool/error/v1`.
struct ErrorForm;

impl FailureForm for ErrorForm {
    fn document(&self) -> Document<()> {
        Document::new("tool", "error", 1, "The document of a failed command.")
    }

    fn schema(&self) -> Schema {
        json_schema!({ "type": "object" })
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> serde_json::Value {
        serde_json::json!({ "error": failure.message() })
    }
}

/// A failure form whose document is `tool/error/v1` with the failure's code
/// and the call of its recovery, and which reads that document back.
struct ReadingForm;

impl FailureForm for ReadingForm {
    fn document(&self) -> Document<()> {
        ErrorForm.document()
    }

    fn schema(&self) -> Schema {
        ErrorForm.schema()
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> serde_json::Value {
        serde_json::json!({ "error": failure.message() })
    }

    fn read(&self, out: &str, _err: &str) -> Option<ReadFailure> {
        let document: serde_json::Value = serde_json::from_str(out.lines().last()?).ok()?;
        let code = document["error"]["code"].as_str()?;
        let call = document["error"]["run"].as_str()?;
        Some(ReadFailure::new(
            code.to_owned(),
            vec![Step::Run(call.to_owned())],
        ))
    }
}

/// The form of the tool that reads no failure back.
static FORM: ErrorForm = ErrorForm;

/// The form of the tool that reads its failures back.
static READING: ReadingForm = ReadingForm;

/// The guide of a tool that prints its failures as `tool/error/v1`.
const GUIDE: Guide = Guide::new().failure_form(&FORM);

/// The guide of a tool that prints its failures as `tool/error/v1` and
/// reads them back.
const READING_GUIDE: Guide = Guide::new().failure_form(&READING);

/// The answers of a tool whose start fails in its own form, and whose
/// recovery `tool books list` succeeds.
const ANSWERS: &[Answer<'_>] = &[
    (
        "books show 7 --json",
        1,
        r#"{"error":{"code":"tool::books::not_found","run":"tool books list"}}"#,
        "",
    ),
    ("books list --json", 0, "[]", ""),
];

/// The opt-out that names the root of the tool with the failure form.
const ROOT: &[Allow] = &[Allow::new(
    Rule::Recoveries,
    "",
    "The tool prints its failures as tool/error/v1.",
)];

/// Runs the check of the tool with `guide` and the opt-outs `allow`, and
/// returns the panic message, or `None` when it passes, and the report.
fn checked(guide: &Guide, allow: &[Allow]) -> (Option<String>, Setup) {
    let setup = Setup::new("recoveries-form", ANSWERS);
    let recoveries = Recoveries::new(
        Examples::new(&setup.script, "").allow(allow),
        crumbkit::Spelling::COLONS,
        CODES,
    )
    .starts(SHOW_7)
    .unprovoked(OTHERS);
    let message = panic_message(|| {
        check(
            &tool(),
            guide,
            &recoveries,
            &setup.golden(),
            &Assert::new().action(Action::Overwrite),
        );
    });
    (message, setup)
}

#[test]
fn a_form_that_reads_no_failure_stops_the_check_at_the_first_failed_run_with_one_problem() {
    let (message, _setup) = checked(&GUIDE, &[]);

    assert_eq!(
        message.as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             tool books show 7: the start ends with exit status: 1, and the failure form \
             tool/error/v1 reads no failure from it, so the check cannot follow the tool's \
             recoveries. The tool implements FailureForm::read, or the tool adds an opt-out \
             from `recoveries/follow` that names the root, with a reason"
        )
    );
}

#[test]
fn an_opt_out_that_names_the_root_skips_every_start_of_a_tool_whose_form_reads_no_failure() {
    let (message, setup) = checked(&GUIDE, ROOT);

    // The opt-out skips the start, so the start's code goes unprovoked.
    let message = message.unwrap();
    assert_eq!(
        message,
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool::books::not_found: no start provokes the declared code, and Recoveries::unprovoked does not list it"
    );
    assert!(!setup.report().contains("tool books show 7"));
}

#[test]
fn a_form_that_reads_its_failures_has_its_recoveries_followed() {
    let (message, setup) = checked(&READING_GUIDE, &[]);

    assert_eq!(message, None);
    assert!(
        setup.report().starts_with(
            "tool books show 7\n  fails with tool::books::not_found\n  run tool books list\n    exits 0\n"
        ),
        "{}",
        setup.report()
    );
}
