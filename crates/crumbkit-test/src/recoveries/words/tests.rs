use crumbkit::help::quote;
use proptest::prelude::*;

use super::split;

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

#[test]
fn split_reads_a_double_quoted_word_as_a_posix_shell_does() {
    assert_eq!(
        split(
            r#"shelf books list --author "Jane Austen" --title="Emma" "say \"hi\"" "a\\b" "C:\dir" """#
        ),
        Some(vec![
            "shelf".to_owned(),
            "books".to_owned(),
            "list".to_owned(),
            "--author".to_owned(),
            "Jane Austen".to_owned(),
            "--title=Emma".to_owned(),
            r#"say "hi""#.to_owned(),
            r"a\b".to_owned(),
            r"C:\dir".to_owned(),
            String::new(),
        ])
    );
    assert_eq!(
        split("shelf \"one\\\ntwo\" \"\\$HOME\""),
        Some(vec![
            "shelf".to_owned(),
            "onetwo".to_owned(),
            "$HOME".to_owned()
        ])
    );
}

#[test]
fn split_refuses_a_double_quoted_word_that_a_shell_would_expand_or_that_does_not_close() {
    assert_eq!(split(r#"shelf "$HOME""#), None);
    assert_eq!(split("shelf \"`date`\""), None);
    assert_eq!(split(r#"shelf "open"#), None);
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
