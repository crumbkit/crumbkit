//! Tests for checking the command lines of a skill file.

use std::path::Path;

use crumbkit::Row as _;

use super::{Checked, SKILL_CHECK, listing};
use crate::skill::tree::Tree;

/// A `shelf/commands-list/v1` document of shelf's `books list` and its
/// parents.
const DOCUMENT: &str = r#"{"schema":"shelf/commands-list/v1","data":[
{"path":"shelf","arguments":[
{"name":"--json","short":null,"value_names":[],"many":false,"possible_values":[],"global":true}]},
{"path":"shelf books","arguments":[]},
{"path":"shelf books list","arguments":[
{"name":"--limit","short":null,"value_names":["N"],"many":false,"possible_values":[],"global":false}]}
]}"#;

/// A skill file with two passing command lines, two failing ones and code
/// that names another program.
const SKILL: &str = "\
# shelf

Run `shelf books list` to see the books, and `cargo test` to test.

```sh
$ shelf books list --limit 5 --json
shelf books lsit
git status
```

Never run `shelf books list --author 'Jane`.
";

/// Returns the row of `line` in `file` with `span` and `reason`.
fn row(line: u64, span: &str, reason: Option<&str>) -> Checked {
    Checked {
        file: "skill/SKILL.md".to_owned(),
        line,
        span: span.to_owned(),
        reason: reason.map(ToOwned::to_owned),
    }
}

#[test]
fn every_command_line_of_the_tool_gets_a_row_with_the_failing_ones_first() {
    let tree = Tree::read(DOCUMENT.as_bytes()).unwrap();

    let checked = listing(Path::new("skill/SKILL.md"), SKILL, &tree);

    assert_eq!(
        checked.rows(),
        [
            row(
                7,
                "shelf books lsit",
                Some("`lsit` is not a command of `shelf books`")
            ),
            row(
                11,
                "shelf books list --author 'Jane",
                Some("the command line has an unclosed quote")
            ),
            row(3, "shelf books list", None),
            row(6, "shelf books list --limit 5 --json", None),
        ]
    );
    assert_eq!(
        checked.get_summary().unwrap().to_string(),
        "Checked 4 shelf command lines in skill/SKILL.md, and 2 failed."
    );
}

#[test]
fn a_failing_row_prints_its_file_line_span_and_reason_on_one_line() {
    let failing = row(
        8,
        "shelf books lsit",
        Some("`lsit` is not a command of `shelf books`"),
    );

    assert!(failing.is_problem());
    assert_eq!(
        failing.text_line().unwrap(),
        "skill/SKILL.md:8: shelf books lsit: `lsit` is not a command of `shelf books`"
    );
    assert!(!row(3, "shelf books list", None).is_problem());
}

#[test]
fn the_checked_in_schema_matches_the_skill_check_document_schema() {
    let generated = serde_json::to_string_pretty(&crumbkit::schemas::document_schema(SKILL_CHECK))
        .unwrap()
        + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/skill-check-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/skill-check-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}
