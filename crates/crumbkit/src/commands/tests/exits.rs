//! Tests for the exit codes that `commands list` shows: each row's codes
//! in JSON, and the note after the text table.

use super::{args, output, paths};
use crate::commands::{COMMANDS_LIST, list};
use crate::help::{Exit, Guide};
use crate::{Sink, finish_to};

/// A guide with a code that every command returns and a code that only
/// `books list` returns.
const GUIDE: Guide = Guide::new().exits(&[
    Exit::new(3, "The catalogue was locked by another run."),
    Exit::new(124, "The download ran out of time.").commands(&[&["books", "list"]]),
]);

/// Returns the exit codes of each row that `words` list with `guide`, in
/// row order.
fn codes(guide: &Guide, words: &[&str]) -> Vec<(String, Vec<u64>)> {
    let rows = serde_json::to_value(list(&mut output(), guide, &args(words)).unwrap()).unwrap();
    paths(&rows)
        .into_iter()
        .zip(rows.as_array().unwrap())
        .map(|(path, row)| {
            let codes = row["exits"]
                .as_array()
                .unwrap()
                .iter()
                .map(|exit| exit["exit"].as_u64().unwrap())
                .collect();
            (path.to_owned(), codes)
        })
        .collect()
}

/// Returns what `finish_to` prints on the error writer for `commands list`
/// with `words` and `guide`, in text.
fn notes(guide: &Guide, words: &[&str]) -> String {
    let mut output = output();
    let result = list(&mut output, guide, &args(words));
    let mut out = Vec::new();
    let mut err = Vec::new();
    let _ = finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
        COMMANDS_LIST,
        result,
    );
    String::from_utf8(err).unwrap()
}

#[test]
fn each_row_lists_the_tool_s_exit_codes_that_its_command_can_return() {
    assert_eq!(
        codes(&GUIDE, &["books"]),
        [
            ("shelf books".to_owned(), vec![3]),
            ("shelf books list".to_owned(), vec![3, 124]),
            ("shelf books show".to_owned(), vec![3]),
        ]
    );
}

#[test]
fn a_row_of_a_tool_without_exit_codes_has_an_empty_list() {
    assert_eq!(
        codes(&Guide::new(), &["books", "list"]),
        [("shelf books list".to_owned(), vec![])]
    );
}

#[test]
fn an_exit_code_holds_its_number_and_its_meaning() {
    let rows =
        serde_json::to_value(list(&mut output(), &GUIDE, &args(&["books", "list"])).unwrap())
            .unwrap();

    assert_eq!(
        rows[0]["exits"][1],
        serde_json::json!({ "exit": 124, "meaning": "The download ran out of time." })
    );
}

#[test]
fn the_row_schema_describes_each_field_of_an_exit_code_and_requires_it() {
    let rows =
        serde_json::to_value(list(&mut output(), &GUIDE, &args(&["books", "list"])).unwrap())
            .unwrap();
    let schema = schemars::schema_for!(crate::commands::CommandRow);

    super::assert_fields_match(
        &rows[0]["exits"][0],
        &schema.as_value()["properties"]["exits"]["items"],
    );
    assert!(
        schema.as_value()["properties"]["exits"]["items"]["properties"]["meaning"]["description"]
            .is_string()
    );
}

#[test]
fn the_text_tables_last_note_on_standard_error_names_the_exit_codes_and_the_topic() {
    let printed = notes(&GUIDE, &["books", "list"]);

    assert!(
        printed.ends_with(
            "Exit codes: 0, 1 and 2 apply to every command, and shelf adds 3 and 124. `shelf help exit-codes` describes them.\n"
        ),
        "{printed}"
    );
}

#[test]
fn the_note_names_the_topic_with_the_command_line_of_the_guide_s_root() {
    const ROOTED: Guide = Guide::new()
        .root(&["books"])
        .exits(&[Exit::new(3, "The catalogue was locked by another run.")]);

    let printed = notes(&ROOTED, &["books", "list"]);

    assert!(
        printed.ends_with(
            "Exit codes: 0, 1 and 2 apply to every command, and shelf books adds 3. `shelf books help exit-codes` describes them.\n"
        ),
        "{printed}"
    );
}
