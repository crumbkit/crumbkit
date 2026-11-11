//! Tests for the queries that a program which reads a tool runs.

use super::{
    COMMANDS_LIST, QUERIES, SCHEMAS_LIST, SCHEMAS_SHOW, SELF_FIGURES_LIST, SELF_RUNS_LIST,
    SELF_USAGE,
};

#[test]
fn commands_list_asks_for_every_row_whole_as_json_without_a_question() {
    assert_eq!(
        COMMANDS_LIST.words(),
        [
            "commands",
            "list",
            "--json",
            "--full",
            "--all",
            "--no-input"
        ]
    );
}

#[test]
fn schemas_list_asks_for_every_row_whole_as_json_without_a_question() {
    assert_eq!(
        SCHEMAS_LIST.words(),
        ["schemas", "list", "--json", "--full", "--all", "--no-input"]
    );
}

#[test]
fn schemas_show_puts_the_ids_after_two_dashes() {
    let ids = ["shelf/books-list/v1".to_owned(), "-odd".to_owned()];

    assert_eq!(
        SCHEMAS_SHOW.words_with_ids(&ids),
        [
            "schemas",
            "show",
            "--json",
            "--no-input",
            "--",
            "shelf/books-list/v1",
            "-odd"
        ]
    );
}

#[test]
fn schemas_show_without_ids_still_ends_with_two_dashes() {
    assert_eq!(
        SCHEMAS_SHOW.words(),
        ["schemas", "show", "--json", "--no-input", "--"]
    );
}

#[test]
fn self_figures_list_asks_for_every_row_whole_as_json_without_a_question() {
    assert_eq!(
        SELF_FIGURES_LIST.words(),
        [
            "self",
            "figures",
            "list",
            "--json",
            "--full",
            "--all",
            "--no-input"
        ]
    );
}

#[test]
fn self_usage_asks_for_the_whole_report_as_json_without_a_question() {
    assert_eq!(
        SELF_USAGE.words(),
        ["self", "usage", "--json", "--full", "--no-input"]
    );
}

#[test]
fn self_runs_list_asks_for_every_row_whole_as_json_without_a_question() {
    assert_eq!(
        SELF_RUNS_LIST.words(),
        [
            "self",
            "runs",
            "list",
            "--json",
            "--full",
            "--all",
            "--no-input"
        ]
    );
}

#[test]
fn a_query_that_takes_no_ids_leaves_out_the_ids_it_is_given() {
    assert_eq!(
        SELF_USAGE.words_with_ids(&["shelf/books-list/v1"]),
        ["self", "usage", "--json", "--full", "--no-input"]
    );
}

#[test]
fn the_list_holds_each_query_once_in_the_order_a_reader_runs_them() {
    let paths: Vec<String> = QUERIES.iter().map(|query| query.path().join(" ")).collect();

    assert_eq!(
        paths,
        [
            "commands list",
            "schemas list",
            "schemas show",
            "self figures list",
            "self usage",
            "self runs list"
        ]
    );
}
