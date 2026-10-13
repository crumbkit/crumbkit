//! Tests for the guide check of the view flags that an example types.

use clap::Args as _;
use crumbkit::help::{Example, Guide};
use crumbkit::{Document, Output, Prose};

use super::{check_guide, leaf, panic_message, shelf};

/// The prose document of `shelf books list` in these tests.
const COUNT: Document<Prose<String>> = Document::new("shelf", "count", 1, "The count.");

#[test]
fn check_guide_fails_on_an_example_that_types_a_view_flag_its_leaf_does_not_offer() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"])
            .examples(&[Example::new(&["--long"], "Count the books at length")])
            .document(COUNT),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&Output::augment_args(shelf()), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books list: the example `shelf books list --long` types --long, which the command does not offer, so crumbkit refuses it"
    );
}
