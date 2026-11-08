//! Tests for the root verbs in the opt-out report, which it lists with
//! their reason among the declarations.

use crumbkit::Tool;
use serde_json::{Value, json};
use snapbox::assert::Action;

use super::{read, run, run_guided};
use crate::Allow;
use crate::temp::TempDir;
use crumbkit::help::Help;

/// The reason that the tests give for the root verb `version`.
const REASON: &str = "People type `shelf version` from habit.";

#[test]
fn root_verbs_on_the_tool_are_a_declaration_with_their_reason_and_heading() {
    const TOOL: Tool = Tool::new().help(Help::new().root_verbs(&["version"], REASON));
    let dir = TempDir::new("opt-outs-root-verbs-guide").unwrap();

    run_guided(&TOOL, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from the conventions.\n\
         \n\
         shelf declares 1 declaration.\n\
         \n\
         root-verbs: shelf version\n\
         \x20 Declares: Each command sits directly under the root as a verb, and root help lists it under the heading Verbs.\n\
         \x20 Reason: People type `shelf version` from habit.\n"
    );
    let report: Value = serde_json::from_str(&read(&dir, "opt-outs.json")).unwrap();
    assert_eq!(
        report["declarations"],
        json!([{
            "declaration": "root-verbs",
            "applies_to": "shelf version",
            "declares": "Each command sits directly under the root as a verb, and root help lists it under the heading Verbs.",
            "reason": REASON,
        }])
    );
}

#[test]
fn root_verbs_declared_with_allow_root_verbs_are_a_declaration_and_no_opt_out() {
    const ALLOW: &[Allow] = &[Allow::root_verbs(&["version"], REASON)];
    let dir = TempDir::new("opt-outs-root-verbs-allow").unwrap();

    run(ALLOW, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from the conventions.\n\
         \n\
         shelf declares 1 declaration.\n\
         \n\
         root-verbs: shelf version\n\
         \x20 Declares: Each command sits directly under the root as a verb.\n\
         \x20 Reason: People type `shelf version` from habit.\n"
    );
    let report: Value = serde_json::from_str(&read(&dir, "opt-outs.json")).unwrap();
    assert_eq!(report["data"], json!([]));
    assert_eq!(report["declarations"][0]["reason"], REASON);
}
