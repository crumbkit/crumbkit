//! Tests for the root verbs in the opt-out report, which it lists with
//! their reason among the declarations.

use crumbkit::Tool;
use serde_json::{Value, json};
use snapbox::assert::Action;

use super::{read, run, run_with_tool};
use crate::Allow;
use crate::temp::TempDir;
use crumbkit::help::{Goal, Group, Help};

/// The reason that the tests give for the root verb `version`.
const REASON: &str = "People type `shelf version` from habit.";

#[test]
fn root_verbs_on_the_tool_are_a_declaration_with_their_reason_and_heading() {
    const TOOL: Tool = Tool::new().help(Help::new().root_verbs(&["version"], REASON));
    let dir = TempDir::new("opt-outs-root-verbs-tool").unwrap();

    run_with_tool(&TOOL, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from the conventions.\n\
         \n\
         shelf has 1 declaration.\n\
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
fn the_root_verbs_declaration_names_the_heading_that_root_help_lists_them_under() {
    const READ: &[Goal] = &[Goal::new("Read", &["version"])];
    const ALL: Tool = Tool::new().help(
        Help::new()
            .groups(&[Group::new(&[], READ)])
            .root_verbs(&["version"], REASON),
    );
    const SOME: Tool = Tool::new().help(
        Help::new()
            .groups(&[Group::new(&[], READ)])
            .root_verbs(&["version", "books"], REASON),
    );
    const NESTED: Tool = Tool::new().help(
        Help::new()
            .groups(&[Group::new(&["books"], READ)])
            .root_verbs(&["version"], REASON),
    );
    let goal = "the heading of the root goal that names it";
    let cases = [
        (&ALL, format!("{goal}.")),
        (
            &SOME,
            format!("{goal}, or under the heading Verbs when no goal names it."),
        ),
        (&NESTED, "the heading Verbs.".to_owned()),
    ];
    for (tool, heading) in cases {
        let dir = TempDir::new("opt-outs-root-verbs-heading").unwrap();

        run_with_tool(tool, &dir, Action::Overwrite);

        let report: Value = serde_json::from_str(&read(&dir, "opt-outs.json")).unwrap();
        let declares = format!(
            "Each command sits directly under the root as a verb, and root help lists it under \
             {heading}"
        );
        assert_eq!(report["declarations"][0]["declares"], declares);
    }
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
         shelf has 1 declaration.\n\
         \n\
         root-verbs: shelf version\n\
         \x20 Declares: Each command sits directly under the root as a verb.\n\
         \x20 Reason: People type `shelf version` from habit.\n"
    );
    let report: Value = serde_json::from_str(&read(&dir, "opt-outs.json")).unwrap();
    assert_eq!(report["data"], json!([]));
    assert_eq!(report["declarations"][0]["reason"], REASON);
}
