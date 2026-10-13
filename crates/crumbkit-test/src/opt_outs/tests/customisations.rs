//! Tests for the customisations section of the opt-out report, which lists
//! the tool's declarations without a reason or a cost.

use crumbkit::codes::CLI_USAGE;
use crumbkit::form::{FailureForm, PreparedFailure};
use crumbkit::help::{Exit, Guide, Leaf, Rename};
use crumbkit::{Code, Document};
use schemars::{Schema, json_schema};
use serde_json::{Value, json};
use snapbox::assert::Action;

use super::{read, run_guided};
use crate::temp::TempDir;

/// A failure form whose document is `shelf/error/v2`.
struct ErrorForm;

impl FailureForm for ErrorForm {
    fn document(&self) -> Document<()> {
        Document::new("shelf", "error", 2, "The document of a failed command.")
    }

    fn schema(&self) -> Schema {
        json_schema!({ "type": "object" })
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> Value {
        json!({ "error": failure.message() })
    }
}

/// The tool's code for crumbkit's usage code.
const USAGE: Code = Code::new("shelf", "cli", "usage");

/// A guide that declares a failure form, a rename, an exit for the renamed
/// usage code and a leaf that passes on its program's exit code.
const DECLARING: Guide = Guide::new()
    .failure_form(&ErrorForm)
    .codes(&[Rename::new(&CLI_USAGE, &USAGE)])
    .exits(&[Exit::new(64, "The command line was refused.").failures(&[&USAGE])])
    .leaves(&[Leaf::new(&["books", "show"]).passes_on_exit("the pager")]);

#[test]
fn the_report_lists_declarations_under_customisations_without_a_reason_or_a_cost() {
    let dir = TempDir::new("opt-outs-customisations").unwrap();

    run_guided(&DECLARING, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from crumbkit's conventions.\n\
         \n\
         shelf declares 4 customisations, which need no reason.\n\
         \n\
         failure-form: every run of shelf\n\
         \x20 Declares: Failures print as shelf/error/v2 in place of crumbkit/failure/v1.\n\
         \n\
         code-rename: every run of shelf\n\
         \x20 Declares: Runs print shelf::cli::usage in place of crumbkit::cli::usage.\n\
         \n\
         exit-code: every run of shelf\n\
         \x20 Declares: A failure with shelf::cli::usage exits with 64, where crumbkit's default is 2.\n\
         \n\
         passes-on-exit: shelf books show\n\
         \x20 Declares: The command exits with the exit code of the pager.\n"
    );
    let report: Value = serde_json::from_str(&read(&dir, "opt-outs.json")).unwrap();
    assert_eq!(report["data"], json!([]));
    assert_eq!(
        report["customisations"][3],
        json!({
            "declaration": "passes-on-exit",
            "applies_to": "shelf books show",
            "declares": "The command exits with the exit code of the pager.",
        })
    );
    assert_eq!(report["customisations"].as_array().unwrap().len(), 4);
}

#[test]
fn a_report_without_declarations_leaves_out_customisations() {
    let dir = TempDir::new("opt-outs-no-customisations").unwrap();

    run_guided(&Guide::new(), &dir, Action::Overwrite);

    let report: Value = serde_json::from_str(&read(&dir, "opt-outs.json")).unwrap();
    assert_eq!(
        report,
        json!({ "schema": "crumbkit/opt-outs/v1", "data": [] })
    );
}

#[test]
fn an_exit_that_lists_the_tool_s_own_code_is_no_customisation() {
    const MINE: Code = Code::new("shelf", "books", "not_found");
    const GUIDE: Guide = Guide::new().exits(&[Exit::new(3, "No such book.").failures(&[&MINE])]);
    let dir = TempDir::new("opt-outs-own-exit").unwrap();

    run_guided(&GUIDE, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from crumbkit's conventions.\n"
    );
}
