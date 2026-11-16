//! Tests for `Aliases::expand`, which replaces an alias with its words.

use std::ffi::OsString;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::testing::tree;
use crate::{Aliases, Entry, Kind, Source};

/// An aliases file with one alias that takes extra words after its own.
const MINE: &str = r#"mine = ["books", "list", "--long"]"#;

/// Returns `words` as the arguments of a process.
fn os(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

/// Expands `words` against the test tool with a personal source that holds
/// `text`.
fn expanded(text: &str, words: &[&str]) -> Vec<OsString> {
    Aliases::new()
        .source(Source::text(Kind::Personal, Some(text)))
        .expand(&tree(), words.iter().copied())
}

/// A loader that fails the test, for a source that a run must not read.
fn boom() -> Result<Vec<Entry>, String> {
    panic!("expand read a source that it must not read")
}

/// Expands `words` against the test tool and fails the test if `expand`
/// reads a file or a lazy source.
fn expanded_unread(words: &[&str]) -> Vec<OsString> {
    Aliases::new()
        .source(Source::lazy("lazy", Kind::Shipped, boom))
        .source(Source::lazy("personal", Kind::Personal, boom))
        .expand(&tree(), words.iter().copied())
}

/// A table with a two-word key, a one-word key and a key that starts with a
/// command.
const WORKTREES: Source = Source::table(
    "shelf",
    &[
        ("wt", &["books", "list"]),
        ("wt ls", &["books", "list", "--long"]),
        ("books recent", &["books", "list", "--limit", "5"]),
    ],
);

#[test]
fn an_alias_is_replaced_by_its_words_and_the_words_typed_after_it_follow() {
    let args = expanded(MINE, &["shelf", "mine", "--limit", "1"]);

    assert_eq!(
        args,
        os(&["shelf", "books", "list", "--long", "--limit", "1"])
    );
}

#[test]
fn a_key_of_two_words_expands_when_the_leading_words_match() {
    let aliases = Aliases::new().source(WORKTREES);

    let args = aliases.expand(&tree(), ["shelf", "wt", "ls", "--limit", "2"]);

    assert_eq!(
        args,
        os(&["shelf", "books", "list", "--long", "--limit", "2"])
    );
}

#[test]
fn the_longest_key_in_a_source_wins() {
    let aliases = Aliases::new().source(WORKTREES);

    assert_eq!(
        aliases.expand(&tree(), ["shelf", "wt", "ls"]),
        os(&["shelf", "books", "list", "--long"])
    );
    assert_eq!(
        aliases.expand(&tree(), ["shelf", "wt", "other"]),
        os(&["shelf", "books", "list", "other"])
    );
}

#[test]
fn the_first_listed_source_wins_when_two_sources_match() {
    const SHIPPED: Source = Source::table("shelf", &[("mine", &["books", "show"])]);
    let first = Aliases::new()
        .source(Source::text(Kind::Personal, Some(MINE)))
        .source(SHIPPED);
    let second = Aliases::new()
        .source(SHIPPED)
        .source(Source::text(Kind::Personal, Some(MINE)));

    assert_eq!(
        first.expand(&tree(), ["shelf", "mine"]),
        os(&["shelf", "books", "list", "--long"])
    );
    assert_eq!(
        second.expand(&tree(), ["shelf", "mine"]),
        os(&["shelf", "books", "show"])
    );
}

#[test]
fn a_shorter_key_in_an_earlier_source_wins_over_a_longer_key_in_a_later_one() {
    let aliases = Aliases::new()
        .source(Source::table("first", &[("wt", &["books", "show"])]))
        .source(WORKTREES);

    let args = aliases.expand(&tree(), ["shelf", "wt", "ls"]);

    assert_eq!(args, os(&["shelf", "books", "show", "ls"]));
}

#[test]
fn a_table_key_that_starts_with_a_command_expands_when_its_last_word_is_no_subcommand() {
    let aliases = Aliases::new().source(WORKTREES);

    let args = aliases.expand(&tree(), ["shelf", "books", "recent"]);

    assert_eq!(args, os(&["shelf", "books", "list", "--limit", "5"]));
}

#[test]
fn a_table_key_that_names_a_real_command_path_never_expands() {
    let aliases = Aliases::new().source(Source::table(
        "shelf",
        &[
            ("books list", &["books", "show"]),
            ("books ls", &["books", "show"]),
            ("books", &["books", "show"]),
        ],
    ));

    for words in [
        &["shelf", "books", "list"][..],
        &["shelf", "books", "ls"],
        &["shelf", "books"],
    ] {
        assert_eq!(
            aliases.expand(&tree(), words.iter().copied()),
            os(words),
            "{words:?}"
        );
    }
}

#[test]
fn an_alias_whose_words_start_with_another_alias_expands_once() {
    let text = format!("{MINE}\nours = [\"mine\", \"--limit\", \"1\"]");

    let args = expanded(&text, &["shelf", "ours"]);

    assert_eq!(args, os(&["shelf", "mine", "--limit", "1"]));
}

#[test]
fn a_real_command_is_left_unchanged_without_reading_a_file_or_a_lazy_source() {
    let words = ["shelf", "books", "list"];

    assert_eq!(expanded_unread(&words), os(&words));
}

#[test]
fn a_built_in_alias_is_left_for_clap_without_reading_a_file_or_a_lazy_source() {
    let visible = ["shelf", "b", "list"];
    let hidden_command = ["shelf", "adm", "reset"];

    assert_eq!(expanded_unread(&visible), os(&visible));
    assert_eq!(expanded_unread(&hidden_command), os(&hidden_command));
}

#[test]
fn help_is_left_unchanged_without_reading_a_file_or_a_lazy_source() {
    let words = ["shelf", "help", "books"];

    assert_eq!(expanded_unread(&words), os(&words));
}

#[test]
fn a_flag_as_the_first_word_is_left_unchanged_without_reading_a_file_or_a_lazy_source() {
    let words = ["shelf", "--json", "mine"];

    assert_eq!(expanded_unread(&words), os(&words));
}

#[test]
fn a_command_line_without_a_first_word_is_left_unchanged_without_reading_a_file_or_a_lazy_source() {
    assert_eq!(expanded_unread(&["shelf"]), os(&["shelf"]));
    assert_eq!(expanded_unread(&[]), os(&[]));
}

/// How many times [`counted`] has run.
static LOADS: AtomicUsize = AtomicUsize::new(0);

/// A loader that counts its runs.
fn counted() -> Result<Vec<Entry>, String> {
    LOADS.fetch_add(1, Ordering::SeqCst);
    Ok(vec![Entry::new("wt", ["books", "list"])])
}

#[test]
fn a_lazy_source_runs_its_loader_once_when_the_first_word_is_not_a_command() {
    let aliases = Aliases::new().source(Source::lazy("lazy", Kind::Shipped, counted));
    let before = LOADS.load(Ordering::SeqCst);

    let real = aliases.expand(&tree(), ["shelf", "books", "list"]);
    let alias = aliases.expand(&tree(), ["shelf", "wt"]);

    assert_eq!(real, os(&["shelf", "books", "list"]));
    assert_eq!(alias, os(&["shelf", "books", "list"]));
    assert_eq!(LOADS.load(Ordering::SeqCst) - before, 1);
}

#[test]
fn a_missing_file_or_no_file_leaves_the_arguments_unchanged() {
    let missing = Path::new(env!("CARGO_MANIFEST_DIR")).join("no-such-aliases.toml");
    let words = ["shelf", "mine"];

    let missing = Aliases::new().source(Source::file(Some(missing)));
    let none = Aliases::new().source(Source::file(None));

    assert_eq!(missing.expand(&tree(), words), os(&words));
    assert_eq!(none.expand(&tree(), words), os(&words));
}

#[test]
fn an_unparsable_file_leaves_the_arguments_unchanged() {
    let words = ["shelf", "mine"];

    assert_eq!(expanded("mine = [", &words), os(&words));
}

#[test]
fn an_unreadable_source_is_skipped_and_a_later_source_still_expands() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    let aliases = Aliases::new()
        .source(Source::file(Some(directory)))
        .source(WORKTREES);

    let args = aliases.expand(&tree(), ["shelf", "wt"]);

    assert_eq!(args, os(&["shelf", "books", "list"]));
}

#[test]
fn a_failing_lazy_loader_is_skipped_and_a_later_source_still_expands() {
    fn fails() -> Result<Vec<Entry>, String> {
        Err("no network".to_owned())
    }
    let aliases = Aliases::new()
        .source(Source::lazy("lazy", Kind::Shipped, fails))
        .source(WORKTREES);

    let args = aliases.expand(&tree(), ["shelf", "wt"]);

    assert_eq!(args, os(&["shelf", "books", "list"]));
}

#[test]
fn a_word_that_no_source_defines_is_left_unchanged() {
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
fn a_file_key_of_two_words_expands_and_a_file_key_that_starts_with_a_command_does_not() {
    let text = "\"wt ls\" = [\"books\", \"list\"]\n\"books recent\" = [\"books\", \"show\"]";

    assert_eq!(
        expanded(text, &["shelf", "wt", "ls", "x"]),
        os(&["shelf", "books", "list", "x"])
    );
    let words = ["shelf", "books", "recent"];
    assert_eq!(expanded(text, &words), os(&words));
}
