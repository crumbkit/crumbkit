//! Tests for shelf as a whole that read its private items. The convention
//! tests, which need only shelf's public command tree and guide, live in
//! `tests/conventions.rs`.

use crate::books::BooksError;
use crate::cli::GUIDE;

#[test]
fn every_failure_code_spells_uniquely_as_printable_ascii() {
    crumbkit_test::check_codes(&GUIDE, &[BooksError::CODES]);
}

/// The choices of the whole catalogue, as `books::choices` offers them when
/// no book is removed.
fn catalogue() -> Result<Vec<crumbkit::pick::Choice>, crumbkit::Error> {
    Ok(crate::books::choices_from(None)?)
}

#[test]
fn books_show_without_ids_asks_the_menu_and_shows_the_picked_books() {
    use clap::CommandFactory as _;
    use crumbkit::help::{Guide, Leaf};
    use crumbkit::pick::Choices;

    static MENU: crumbkit::pick::Menu = crumbkit::pick::Menu::new();
    const GUIDE: Guide = Guide::new()
        .picker(&MENU)
        .leaves(&[Leaf::new(&["books", "show"]).choices(&[Choices::new("ids", catalogue)])]);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = crumbkit::Sink::with_input(&mut out, &mut err, &b"austen\n1\n"[..], true);
    let parsed = crumbkit::parse_to::<crate::Cli, _, _, _, _>(
        &mut sink,
        crate::Cli::command(),
        &GUIDE,
        ["shelf", "books", "show"],
    );

    assert!(parsed.is_ok());
    let err = String::from_utf8(err).unwrap();
    assert!(
        err.contains("  1. Pride and Prejudice  Jane Austen"),
        "{err}"
    );
    assert!(err.ends_with("Running: shelf books show 1\n"), "{err}");
}
