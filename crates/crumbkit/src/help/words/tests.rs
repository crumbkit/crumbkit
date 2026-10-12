use proptest::prelude::*;

use super::{quote, split};

#[test]
fn a_word_that_a_posix_shell_would_split_or_expand_is_single_quoted() {
    assert_eq!(quote("--fields=title,year"), "--fields=title,year");
    assert_eq!(quote("./shelf.toml"), "./shelf.toml");
    assert_eq!(quote("Jane Austen"), "'Jane Austen'");
    assert_eq!(quote("$HOME"), "'$HOME'");
    assert_eq!(quote("*.txt"), "'*.txt'");
    assert_eq!(quote("~"), "'~'");
    assert_eq!(quote(""), "''");
    assert_eq!(quote("Ceres's"), r"'Ceres'\''s'");
}

#[test]
fn split_reads_plain_and_quoted_words_back_as_a_shell_would() {
    assert_eq!(
        split(r"shelf books list --author 'Jane Austen' 'Ceres'\''s' ''"),
        Some(vec![
            "shelf".to_owned(),
            "books".to_owned(),
            "list".to_owned(),
            "--author".to_owned(),
            "Jane Austen".to_owned(),
            "Ceres's".to_owned(),
            String::new(),
        ])
    );
}

#[test]
fn split_keeps_a_placeholder_such_as_id_as_one_word() {
    assert_eq!(
        split("shelf books show <ID>..."),
        Some(vec![
            "shelf".to_owned(),
            "books".to_owned(),
            "show".to_owned(),
            "<ID>...".to_owned(),
        ])
    );
}

#[test]
fn split_refuses_a_line_that_quote_cannot_print() {
    assert_eq!(split("shelf 'open"), None);
    assert_eq!(split(r"shelf a\b"), None);
}

proptest! {
    /// `split` reads back every line that `quote` and single spaces build,
    /// whatever characters the words hold.
    #[test]
    fn split_reads_back_the_words_that_quote_joined(words in prop::collection::vec(".*", 0..6)) {
        let line = words.iter().map(|word| quote(word)).collect::<Vec<_>>().join(" ");
        prop_assert_eq!(split(&line), Some(words));
    }
}
