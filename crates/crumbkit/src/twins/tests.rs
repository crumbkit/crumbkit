//! Tests for the twin comparison.

use super::assert_one_word_apart;

#[test]
fn words_that_differ_by_one_word_added_removed_or_replaced_pass() {
    assert_one_word_apart(&["show", "2,5"], &["show", "2"]);
    assert_one_word_apart(&["show"], &["show", "2"]);
    assert_one_word_apart(
        &["list", "--all", "--limit", "2"],
        &["list", "--limit", "2"],
    );
}

#[test]
#[should_panic(expected = "must differ by one word added, removed or replaced")]
fn equal_words_fail() {
    assert_one_word_apart(&["show", "2"], &["show", "2"]);
}

#[test]
#[should_panic(expected = "must differ by one word added, removed or replaced")]
fn words_that_differ_by_two_words_fail() {
    assert_one_word_apart(&["show", "2", "5"], &["show", "3", "6"]);
}
