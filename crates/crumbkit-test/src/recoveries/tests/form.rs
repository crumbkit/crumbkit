//! Tests for the recovery check on a tool that prints its failures in its
//! own failure form.

use crumbkit::Document;
use crumbkit::form::{FailureForm, PreparedFailure};
use crumbkit::help::Guide;
use schemars::{Schema, json_schema};
use snapbox::Assert;
use snapbox::assert::Action;

use super::{CODES, SHOW_7, Setup, tool};
use crate::allow::{Allow, Rule};
use crate::examples::Examples;
use crate::recoveries::{Recoveries, Unprovoked, check};
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

/// The form of the tool in these tests.
static FORM: ErrorForm = ErrorForm;

/// The guide of a tool that prints its failures as `tool/error/v1`.
const GUIDE: Guide = Guide::new().failure_form(&FORM);

/// The opt-out that names the root of the tool with the failure form.
const ROOT: &[Allow] = &[Allow::new(
    Rule::Recoveries,
    "",
    "The tool prints its failures as tool/error/v1.",
)];

/// The opt-out that names one command of the tool with the failure form.
const SHOW: &[Allow] = &[Allow::new(
    Rule::Recoveries,
    "books show",
    "The tool prints its failures as tool/error/v1.",
)];

/// Returns the panic message of the check of the tool with the failure
/// form and the opt-outs `allow`, or `None` when it passes.
fn problems(allow: &[Allow]) -> Option<String> {
    let setup = Setup::new("recoveries-form", &[]);
    let unprovoked: Vec<Unprovoked<'_>> = CODES
        .iter()
        .flat_map(|list| list.iter())
        .map(|&code| Unprovoked::new(code, "The tool's failures are not read."))
        .collect();
    let recoveries = Recoveries::new(
        Examples::new(&setup.script, "").allow(allow),
        crumbkit::Spelling::COLONS,
        CODES,
    )
    .starts(SHOW_7)
    .unprovoked(&unprovoked);
    panic_message(|| {
        check(
            &tool(),
            &GUIDE,
            &recoveries,
            &setup.golden(),
            &Assert::new().action(Action::Overwrite),
        );
    })
}

#[test]
fn a_tool_with_a_failure_form_fails_the_check_before_any_run_unless_an_opt_out_names_the_root() {
    let expected = "the tool's recoveries break crumbkit's recovery rules:\n\
         tool: the tool prints its failures as tool/error/v1 in place of crumbkit/failure/v1, \
         so the check cannot follow its recoveries, and the tool needs an opt-out from \
         `recoveries/follow` that names the root, with a reason";

    assert_eq!(problems(&[]).as_deref(), Some(expected));
    assert_eq!(problems(SHOW).as_deref(), Some(expected));
    assert_eq!(problems(ROOT), None);
}
