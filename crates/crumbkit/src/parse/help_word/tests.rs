//! Tests for `ask_for_help`, which turns the word `help` into `--help`.

use std::ffi::OsString;

use super::ask_for_help;

/// Returns the words of `line` after `ask_for_help` turned the `help` at
/// `position` into `--help`.
fn asked(line: &[&str], position: usize) -> Vec<OsString> {
    let mut args: Vec<OsString> = line.iter().map(OsString::from).collect();
    ask_for_help(&mut args, position);
    args
}

/// Returns `words` as the words of a command line.
fn words(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

#[test]
fn help_goes_after_the_last_command_word_and_before_the_flags() {
    assert_eq!(
        asked(&["tool", "help", "planets", "show", "--json"], 1),
        words(&["tool", "planets", "show", "--help", "--json"])
    );
}

#[test]
fn help_goes_where_the_help_word_was_when_only_flags_follow_it() {
    assert_eq!(
        asked(&["tool", "planets", "help", "--json", "--long"], 2),
        words(&["tool", "planets", "--help", "--json", "--long"])
    );
}

#[test]
fn help_goes_at_the_end_when_the_line_ends_with_a_command_word() {
    assert_eq!(
        asked(&["tool", "planets", "help", "show"], 2),
        words(&["tool", "planets", "show", "--help"])
    );
}

#[test]
fn help_goes_before_a_double_dash_and_leaves_the_words_after_it_untouched() {
    assert_eq!(
        asked(
            &["tool", "help", "planets", "--json", "--", "show", "-x"],
            1
        ),
        words(&["tool", "planets", "--help", "--json", "--", "show", "-x"])
    );
    assert_eq!(
        asked(&["tool", "help", "--", "planets"], 1),
        words(&["tool", "--help", "--", "planets"])
    );
}

#[test]
fn the_words_before_the_help_word_stay_where_they_are() {
    assert_eq!(
        asked(&["tool", "--json", "planets", "help"], 3),
        words(&["tool", "--json", "planets", "--help"])
    );
}
