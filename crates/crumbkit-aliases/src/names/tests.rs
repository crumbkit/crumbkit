//! Tests for `names`, which names the working aliases on the root's help.

use super::working;
use crate::testing::tool;

#[test]
fn the_names_are_the_entries_that_expand_in_name_order() {
    let text = r#"
mine = ["books", "list", "--long"]
books = ["books", "show", "1"]
help = ["books"]
broken = "books list"
empty = []
firsts = ["books", "list"]
"#;

    assert_eq!(working(&tool(), Some(text)), ["firsts", "mine"]);
}

#[test]
fn a_missing_file_or_a_file_that_does_not_parse_has_no_names() {
    assert_eq!(working(&tool(), None), Vec::<String>::new());
    assert_eq!(working(&tool(), Some("mine = [")), Vec::<String>::new());
}
