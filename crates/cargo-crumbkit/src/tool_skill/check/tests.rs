//! Tests for checking the command lines of a skill file.

use std::path::{Path, PathBuf};

use crumbkit::Row as _;

use super::{Checked, TOOL_SKILL_CHECK, listing, read};
use crate::error::SkillError;
use crate::tool_skill::tree::Tree;

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

/// A second file of the same skill, with one passing command line and one
/// failing one.
const REVIEW: &str = "Run `shelf books list --limit 5`, then `shelf books sumarize`.\n";

/// A skill folder with a `SKILL.md`, a second Markdown file, a text file and
/// a second skill in a subfolder.
const FOLDER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/shelf-skill-folder"
);

/// Returns the row of `line` in `skill/SKILL.md` with `span` and `reason`.
fn row(line: u64, span: &str, reason: Option<&str>) -> Checked {
    row_in("skill/SKILL.md", line, span, reason)
}

/// Returns the row of `line` in `file` with `span` and `reason`.
fn row_in(file: &str, line: u64, span: &str, reason: Option<&str>) -> Checked {
    Checked {
        file: file.to_owned(),
        line,
        span: span.to_owned(),
        reason: reason.map(ToOwned::to_owned),
    }
}

#[test]
fn every_command_line_of_the_tool_gets_a_row_with_the_failing_ones_first() {
    let tree = Tree::read(DOCUMENT.as_bytes()).unwrap();

    let file = PathBuf::from("skill/SKILL.md");
    let checked = listing(&file, &[(file.clone(), SKILL.to_owned())], &tree, "shelf");

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
fn the_rows_of_two_files_put_the_failing_ones_first_and_keep_each_group_in_path_order() {
    let tree = Tree::read(DOCUMENT.as_bytes()).unwrap();
    let files = [
        (PathBuf::from("skill/REVIEW.md"), REVIEW.to_owned()),
        (PathBuf::from("skill/SKILL.md"), SKILL.to_owned()),
    ];

    let checked = listing(Path::new("skill"), &files, &tree, "shelf");

    assert_eq!(
        checked.rows(),
        [
            row_in(
                "skill/REVIEW.md",
                1,
                "shelf books sumarize",
                Some("`sumarize` is not a command of `shelf books`")
            ),
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
            row_in("skill/REVIEW.md", 1, "shelf books list --limit 5", None),
            row(3, "shelf books list", None),
            row(6, "shelf books list --limit 5 --json", None),
        ]
    );
}

#[test]
fn the_summary_of_a_folder_names_the_folder() {
    let tree = Tree::read(DOCUMENT.as_bytes()).unwrap();
    let files = [
        (PathBuf::from("skill/REVIEW.md"), REVIEW.to_owned()),
        (PathBuf::from("skill/SKILL.md"), SKILL.to_owned()),
    ];

    let checked = listing(Path::new("skill"), &files, &tree, "shelf");

    assert_eq!(
        checked.get_summary().unwrap().to_string(),
        "Checked 6 shelf command lines in skill, and 3 failed."
    );
}

#[test]
fn a_folder_gives_the_markdown_files_in_it_and_its_subfolders_in_path_order() {
    let folder = Path::new(FOLDER);

    let files = read(folder).unwrap();

    let paths: Vec<&Path> = files.iter().map(|(path, _)| path.as_path()).collect();
    assert_eq!(
        paths,
        [
            folder.join("REVIEW.md"),
            folder.join("SKILL.md"),
            folder.join("lending").join("SKILL.md"),
        ]
    );
    assert!(files[0].1.starts_with("# Review the catalogue"));
}

#[test]
fn a_file_is_read_alone() {
    let file = Path::new(FOLDER).join("REVIEW.md");

    let files = read(&file).unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, file);
}

#[test]
fn a_folder_with_no_markdown_file_fails_naming_skill_md_in_it() {
    // The usage export fixture holds only `.jsonl` files.
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/shelf-usage-export");

    let Err(SkillError::Unreadable { path, .. }) = read(&folder) else {
        panic!("a folder with no Markdown file is read");
    };

    assert_eq!(path, folder.join("SKILL.md"));
}

#[test]
fn a_path_that_does_not_exist_fails_naming_the_path() {
    for missing in ["missing", "MISSING.md"] {
        let missing = Path::new(FOLDER).join(missing);

        let Err(SkillError::Unreadable { path, .. }) = read(&missing) else {
            panic!("a path that does not exist is read");
        };

        assert_eq!(path, missing);
    }
}

#[test]
fn a_failing_row_prints_its_file_line_span_and_reason_on_one_line() {
    let failing = row(
        8,
        "shelf books lsit",
        Some("`lsit` is not a command of `shelf books`"),
    );

    let reports = |row: &Checked| Checked::PROBLEM.is_some_and(|problem| problem.reports(row));
    assert!(reports(&failing));
    assert_eq!(
        Checked::LINE.unwrap().text(&failing).unwrap(),
        "skill/SKILL.md:8: shelf books lsit: `lsit` is not a command of `shelf books`"
    );
    assert!(!reports(&row(3, "shelf books list", None)));
}

#[test]
fn the_checked_in_schema_matches_the_skill_check_document_schema() {
    let generated =
        serde_json::to_string_pretty(&crumbkit::schemas::document_schema(TOOL_SKILL_CHECK))
            .unwrap()
            + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/tool-skill-check-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/tool-skill-check-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}

#[test]
fn a_tool_that_runs_under_another_program_checks_only_lines_that_name_its_subcommands() {
    // cargo-crumbkit's root is `cargo`, so `cargo test` is cargo's own
    // command and `cargo crumbkit weigh` is the tool's.
    let document = DOCUMENT
        .replace("shelf", "cargo")
        .replace("cargo books", "cargo crumbkit");
    let tree = Tree::read(document.as_bytes()).unwrap();
    let text = "Run `cargo test`, then `cargo crumbkit list --limit`.\n";

    let file = PathBuf::from("SKILL.md");
    let checked = listing(
        &file,
        &[(file.clone(), text.to_owned())],
        &tree,
        "cargo-crumbkit",
    );

    let spans: Vec<(&str, Option<&str>)> = checked
        .rows()
        .iter()
        .map(|row| (row.span.as_str(), row.reason.as_deref()))
        .collect();
    assert_eq!(
        spans,
        [(
            "cargo crumbkit list --limit",
            Some("`--limit` needs a value")
        )]
    );
}
