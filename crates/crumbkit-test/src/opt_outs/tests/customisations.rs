//! Tests for the customisations section of the opt-out report, which lists
//! the tool's declarations without a reason or a cost.

use clap::Command;
use crumbkit::codes::CLI_USAGE;
use crumbkit::form::{FailureForm, PreparedFailure};
use crumbkit::help::{Exit, Guide, Leaf, RefusalContext, Rename};
use crumbkit::schemas::SCHEMAS_LIST;
use crumbkit::{Call, Code, Document, Prose};
use schemars::{Schema, json_schema};
use serde_json::{Value, json};
use snapbox::assert::Action;

use super::{read, run_guided};
use crate::opt_outs::customisations::customisations;
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

/// Returns the call that a refused `books show` command line offers instead.
fn version_call(_: &RefusalContext<'_>) -> Vec<Call> {
    vec![Call::new(&["version"])]
}

/// The tool's code for crumbkit's usage code.
const USAGE: Code = Code::new("shelf", "cli", "usage");

/// A guide that declares a failure form, a rename, an exit for the renamed
/// usage code and a leaf that passes on its program's exit code and offers
/// its own alternatives when its command line is refused.
const DECLARING: Guide = Guide::new()
    .failure_form(&ErrorForm)
    .codes(&[Rename::new(&CLI_USAGE, &USAGE)])
    .exits(&[Exit::new(64, "The command line was refused.").failures(&[&USAGE])])
    .leaves(&[Leaf::new(&["books", "show"])
        .passes_on_exit("the pager")
        .refusal_alternatives(version_call)]);

#[test]
fn the_report_lists_declarations_under_customisations_without_a_reason_or_a_cost() {
    let dir = TempDir::new("opt-outs-customisations").unwrap();

    run_guided(&DECLARING, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from crumbkit's conventions.\n\
         \n\
         shelf declares 5 customisations, which need no reason.\n\
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
         \x20 Declares: The command exits with the exit code of the pager.\n\
         \n\
         refusal-alternatives: shelf books show\n\
         \x20 Declares: A refused command line for this command also offers the tool's own alternatives.\n"
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
    assert_eq!(report["customisations"].as_array().unwrap().len(), 5);
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

const BOOK_SHOW: Document<Prose<String>> = Document::new("shelf", "book-show", 1, "One book.");
const VERSION: Document<Prose<String>> = Document::new("shelf", "version", 1, "The version.");

#[test]
fn a_document_named_other_than_its_command_path_is_a_document_name_customisation() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["books", "show"]).document(BOOK_SHOW),
        Leaf::new(&["version"]).document(VERSION),
        Leaf::new(&["meta", "schemas"]).document(SCHEMAS_LIST),
    ]);
    let dir = TempDir::new("opt-outs-document-name").unwrap();

    run_guided(&GUIDE, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from crumbkit's conventions.\n\
         \n\
         shelf declares 1 customisation, which needs no reason.\n\
         \n\
         document-name: shelf books show\n\
         \x20 Declares: The command prints shelf/book-show/v1, where its path gives the name books-show.\n"
    );
}

#[test]
fn a_cargo_subcommand_names_its_documents_after_the_path_below_its_own_name() {
    const WEIGH: Document<Prose<String>> = Document::new("crumbkit", "weigh", 1, "The weights.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["crumbkit", "weigh"]).document(WEIGH)]);

    assert!(customisations(&Command::new("cargo"), &GUIDE).is_empty());
}
