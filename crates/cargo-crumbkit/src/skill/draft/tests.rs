//! Tests for drafting the generated sections of a skill file.

use crumbkit::Row as _;

use super::usage::{Run, Usage};
use super::{Drafted, SKILL_DRAFT, Section, Status, Summary, drafted};
use crate::skill::tree::Tree;

/// A `shelf/commands-list/v1` document of shelf's root with one goal
/// heading, `books` and `books list`.
const DOCUMENT: &str = r#"{"schema":"shelf/commands-list/v1","data":[
{"path":"shelf","about":"Browse books","goals":[
{"heading":"Browse the catalogue","commands":["books"]}],"arguments":[]},
{"path":"shelf books","about":"Work with the books","arguments":[]},
{"path":"shelf books list","about":"List the books","arguments":[]}
]}"#;

/// Returns the tree of [`DOCUMENT`].
fn tree() -> Tree {
    Tree::read(DOCUMENT.as_bytes()).unwrap()
}

/// Returns a run of `tool`'s `command` with span id `id`.
fn run(id: &str, tool: &str, command: &str) -> Run {
    Run {
        id: Some(id.to_owned()),
        tool: tool.to_owned(),
        session: "a".to_owned(),
        at: format!("2026-11-01T09:00:{id}.000Z"),
        command: Some(command.to_owned()),
        flags: Vec::new(),
        failed: false,
        help: false,
    }
}

/// Returns the section and status of each row.
fn statuses(rows: &[Drafted]) -> Vec<(Section, Status)> {
    rows.iter().map(|row| (row.section, row.status)).collect()
}

#[test]
fn a_missing_file_becomes_a_skeleton_whose_usage_section_is_empty_without_usage() {
    let draft = drafted(None, &tree(), None, 10, false).unwrap();

    assert_eq!(
        statuses(&draft.rows),
        [
            (Section::Description, Status::Written),
            (Section::Jobs, Status::Written),
            (Section::Usage, Status::Kept),
        ]
    );
    assert!(draft.text.contains("description: \"Browse books\"\n"));
    assert!(
        draft
            .text
            .contains("| Browse the catalogue | `shelf books` | Work with the books |\n")
    );
    assert!(
        draft
            .text
            .contains("<!-- crumbkit:generated usage -->\n<!-- /crumbkit:generated usage -->\n")
    );
}

#[test]
fn a_redraft_of_a_drafted_file_changes_nothing() {
    let first = drafted(None, &tree(), None, 10, false).unwrap();

    let second = drafted(Some(&first.text), &tree(), None, 10, true).unwrap();

    assert_eq!(second.text, first.text);
    assert_eq!(
        statuses(&second.rows),
        [
            (Section::Description, Status::Unchanged),
            (Section::Jobs, Status::Unchanged),
            (Section::Usage, Status::Kept),
        ]
    );
}

#[test]
fn a_check_puts_the_sections_that_would_change_first() {
    let text =
        "# shelf\n\n<!-- crumbkit:generated jobs -->\nold\n<!-- /crumbkit:generated jobs -->\n";

    let draft = drafted(Some(text), &tree(), None, 10, true).unwrap();

    assert_eq!(
        statuses(&draft.rows),
        [
            (Section::Jobs, Status::WouldChange),
            (Section::Description, Status::Unmarked),
            (Section::Usage, Status::Kept),
        ]
    );
    assert!(draft.rows[0].is_problem());
    assert!(!draft.rows[1].is_problem());
}

#[test]
fn usage_ranks_only_the_tool_s_leaf_commands_and_counts_the_runs_it_ignores() {
    let usage = Usage {
        runs: vec![
            run("01", "shelf", "books list"),
            run("02", "shelf", "books lend"),
            run("03", "shelf", "books"),
            run("04", "other", "books list"),
            Run {
                help: true,
                ..run("05", "other", "books list")
            },
        ],
        unreadable: 0,
    };
    let text = "<!-- crumbkit:generated usage -->\n<!-- /crumbkit:generated usage -->\n";

    let draft = drafted(Some(text), &tree(), Some(&usage), 10, false).unwrap();

    assert_eq!(draft.runs, 3);
    // The run of another tool and the run of `books lend`, which shelf no
    // longer has. The run of the group `books` and the other tool's help
    // page are not ranked and not ignored.
    assert_eq!(draft.ignored, 2);
    assert!(
        draft
            .text
            .contains("- `shelf books list`: List the books\n"),
        "{}",
        draft.text
    );
    assert!(!draft.text.contains("`shelf books`:"));
}

#[test]
fn top_caps_the_commands_in_the_usage_section() {
    let document = DOCUMENT.replace(
        r#"{"path":"shelf books list","about":"List the books","arguments":[]}"#,
        r#"{"path":"shelf books list","about":"List","arguments":[]},
{"path":"shelf books show","about":"Show","arguments":[]}"#,
    );
    let tree = Tree::read(document.as_bytes()).unwrap();
    let usage = Usage {
        runs: vec![
            run("01", "shelf", "books list"),
            run("02", "shelf", "books list"),
            run("03", "shelf", "books show"),
        ],
        unreadable: 0,
    };

    let draft = drafted(None, &tree, Some(&usage), 1, false).unwrap();

    assert!(draft.text.contains("`shelf books list`"));
    assert!(!draft.text.contains("`shelf books show`"));
}

#[test]
fn an_unclosed_marker_names_its_section_and_line() {
    let text = "a\n<!-- crumbkit:generated jobs -->\n";

    let error = drafted(Some(text), &tree(), None, 10, false).err().unwrap();

    assert_eq!(error.0, Section::Jobs);
    assert_eq!(error.1.line, 2);
}

#[test]
fn the_summary_says_whether_usage_was_read_and_what_it_ignored() {
    let counted = |usage: Option<&str>, runs, ignored, unreadable| Summary {
        file: "skill/SKILL.md".to_owned(),
        tool: "shelf".to_owned(),
        check: usage.is_none(),
        usage: usage.map(str::to_owned),
        runs,
        ignored,
        unreadable,
    };
    let summary = |usage, runs, unreadable| counted(usage, runs, 1, unreadable);

    assert_eq!(
        summary(None, 0, 0).to_string(),
        "Checked skill/SKILL.md for shelf without usage, so its usage section keeps its text."
    );
    assert_eq!(
        summary(Some("usage"), 0, 0).to_string(),
        "Drafted skill/SKILL.md for shelf, and usage holds no runs of shelf, so its usage section keeps its text."
    );
    assert_eq!(
        summary(Some("usage"), 12, 2).to_string(),
        "Drafted skill/SKILL.md for shelf from 12 runs, and ignored 1 that name a command shelf no longer has or another tool. 2 lines of the usage files are not exported usage records."
    );
    assert_eq!(
        counted(Some("usage"), 12, 0, 0).to_string(),
        "Drafted skill/SKILL.md for shelf from 12 runs."
    );
}

#[test]
fn the_checked_in_schema_matches_the_skill_draft_document_schema() {
    let generated = serde_json::to_string_pretty(&crumbkit::schemas::document_schema(SKILL_DRAFT))
        .unwrap()
        + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/skill-draft-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/skill-draft-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}
