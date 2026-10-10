//! Tests for `expand`, which replaces a personal alias with its words.

use std::ffi::OsString;
use std::path::Path;

use super::{expand, expand_with};
use crate::testing::tool;

/// An aliases file with one alias that takes extra words after its own.
const MINE: &str = r#"mine = ["books", "list", "--author", "4"]"#;

/// Returns `words` as the arguments of a process.
fn os(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

/// Expands `words` against the test tool with an aliases file that holds
/// `text`.
fn expanded(text: &str, words: &[&str]) -> Vec<OsString> {
    expand_with(&tool(), os(words), || Some(text.to_owned()))
}

/// Expands `words` against the test tool and fails the test if `expand`
/// reads the aliases file.
fn expanded_unread(words: &[&str]) -> Vec<OsString> {
    expand_with(&tool(), os(words), || {
        panic!("expand read the aliases file for {words:?}")
    })
}

#[test]
fn an_alias_is_replaced_by_its_words_and_the_words_typed_after_it_follow() {
    let args = expanded(MINE, &["shelf", "mine", "--limit", "1"]);

    assert_eq!(
        args,
        os(&["shelf", "books", "list", "--author", "4", "--limit", "1"])
    );
}

#[test]
fn a_real_command_is_left_unchanged_without_reading_the_file() {
    let words = ["shelf", "books", "list"];

    assert_eq!(expanded_unread(&words), os(&words));
}

#[test]
fn a_built_in_alias_is_left_for_clap_without_reading_the_file() {
    let visible = ["shelf", "b", "list"];
    let hidden_command = ["shelf", "adm", "reset"];

    assert_eq!(expanded_unread(&visible), os(&visible));
    assert_eq!(expanded_unread(&hidden_command), os(&hidden_command));
}

#[test]
fn help_is_left_unchanged_without_reading_the_file() {
    let words = ["shelf", "help", "books"];

    assert_eq!(expanded_unread(&words), os(&words));
}

#[test]
fn a_flag_as_the_first_word_is_left_unchanged_without_reading_the_file() {
    let words = ["shelf", "--json", "mine"];

    assert_eq!(expanded_unread(&words), os(&words));
}

#[test]
fn a_command_line_without_a_first_word_is_left_unchanged_without_reading_the_file() {
    assert_eq!(expanded_unread(&["shelf"]), os(&["shelf"]));
    assert_eq!(expanded_unread(&[]), os(&[]));
}

#[test]
fn a_missing_file_or_no_file_leaves_the_arguments_unchanged() {
    let missing = Path::new(env!("CARGO_MANIFEST_DIR")).join("no-such-aliases.toml");
    let words = ["shelf", "mine"];

    assert_eq!(expand(&tool(), words, Some(&missing)), os(&words));
    assert_eq!(expand(&tool(), words, None), os(&words));
}

#[test]
fn an_unparsable_file_leaves_the_arguments_unchanged() {
    let words = ["shelf", "mine"];

    assert_eq!(expanded("mine = [", &words), os(&words));
}

#[test]
fn a_word_that_the_file_does_not_define_is_left_unchanged() {
    let words = ["shelf", "theirs"];

    assert_eq!(expanded(MINE, &words), os(&words));
}

#[test]
fn an_entry_whose_value_is_not_a_list_of_one_or_more_words_is_left_unexpanded() {
    let words = ["shelf", "mine"];

    for text in [
        "mine = []",
        r#"mine = "books list""#,
        r#"mine = ["books", 4]"#,
    ] {
        assert_eq!(expanded(text, &words), os(&words), "{text}");
    }
}

#[test]
fn an_alias_that_starts_with_another_alias_expands_once() {
    let text = format!("{MINE}\nours = [\"mine\", \"--limit\", \"1\"]");

    let args = expanded(&text, &["shelf", "ours"]);

    assert_eq!(args, os(&["shelf", "mine", "--limit", "1"]));
}
