//! Tests for `Aliases::names`, which names the aliases on the root's help.

use crate::testing::tree;
use crate::{Aliases, Kind, Source};

#[test]
fn the_names_are_the_entries_that_expand_in_name_order() {
    let text = r#"
mine = ["books", "list", "--long"]
books = ["books", "show", "1"]
help = ["books"]
broken = "books list"
empty = []
firsts = ["books", "list"]
"wt ls" = ["books", "list"]
"#;
    let aliases = Aliases::new().source(Source::text(Kind::Personal, Some(text)));

    assert_eq!(aliases.names(&tree()), ["firsts", "mine", "wt ls"]);
}

#[test]
fn a_missing_file_or_a_file_that_does_not_parse_has_no_names() {
    let none = Aliases::new().source(Source::text(Kind::Personal, None));
    let broken = Aliases::new().source(Source::text(Kind::Personal, Some("mine = [")));

    assert_eq!(none.names(&tree()), Vec::<String>::new());
    assert_eq!(broken.names(&tree()), Vec::<String>::new());
}

#[test]
fn the_names_of_every_listed_source_are_merged_and_each_name_appears_once() {
    let aliases = Aliases::new()
        .source(Source::text(Kind::Personal, Some(r#"wt = ["books"]"#)))
        .source(Source::table(
            "shelf",
            &[("wt", &["books", "list"]), ("recent", &["books", "list"])],
        ));

    assert_eq!(aliases.names(&tree()), ["recent", "wt"]);
}

#[test]
fn an_unlisted_source_has_no_names() {
    let aliases = Aliases::new()
        .source(Source::table("shelf", &[("recent", &["books", "list"])]).unlisted())
        .source(Source::table("other", &[("wt", &["books", "list"])]));

    assert_eq!(aliases.names(&tree()), ["wt"]);
}

#[test]
fn an_unreadable_source_adds_no_names_and_the_others_still_do() {
    let aliases = Aliases::new()
        .source(Source::text(Kind::Personal, Some("mine = [")))
        .source(Source::table("shelf", &[("wt", &["books", "list"])]));

    assert_eq!(aliases.names(&tree()), ["wt"]);
}

#[test]
fn the_shipped_names_leave_out_every_personal_source() {
    let aliases = Aliases::new()
        .source(Source::text(Kind::Personal, Some(r#"mine = ["books"]"#)))
        .source(Source::table("shelf", &[("wt", &["books", "list"])]))
        .source(Source::table("other", &[("quiet", &["books", "list"])]).unlisted());

    assert_eq!(aliases.names(&tree()), ["mine", "wt"]);
    assert_eq!(aliases.shipped_names(&tree()), ["wt"]);
}

#[test]
fn an_entry_that_an_earlier_source_wins_over_has_no_name() {
    let aliases = Aliases::new()
        .source(Source::text(
            Kind::Personal,
            Some(r#"wt = ["books", "show"]"#),
        ))
        .source(Source::table(
            "shelf",
            &[
                ("wt", &["books", "list"]),
                ("wt ls", &["books", "list"]),
                ("recent", &["books", "list"]),
            ],
        ));

    assert_eq!(aliases.names(&tree()), ["recent", "wt"]);
    assert_eq!(aliases.shipped_names(&tree()), ["recent"]);
}

#[test]
fn an_unlisted_source_still_wins_over_a_later_entry() {
    let aliases = Aliases::new()
        .source(Source::table("hidden", &[("wt", &["books", "show"])]).unlisted())
        .source(Source::table("shelf", &[("wt ls", &["books", "list"])]));

    assert_eq!(aliases.names(&tree()), Vec::<String>::new());
}
