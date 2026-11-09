//! Tests for shelf as a whole that read its private items. The convention
//! tests, which need only shelf's public command tree and tool record, live in
//! `tests/conventions.rs`.

use crate::books::BooksError;
use crate::cli::TOOL;
use crumbkit::help::Help;

#[test]
fn every_failure_code_spells_uniquely_as_printable_ascii() {
    crumbkit_test::check_codes(&TOOL, &[BooksError::CODES]);
}

/// The choices of the whole catalogue, as `books::choices` offers them when
/// no book is removed.
fn catalogue() -> Result<Vec<crumbkit::pick::Choice>, crumbkit::Error> {
    Ok(crate::books::choices_from(None)?)
}

#[test]
fn books_show_without_ids_asks_the_menu_and_shows_the_picked_books() {
    use crumbkit::Tool;
    use crumbkit::help::Leaf;
    use crumbkit::pick::Choices;

    static MENU: crumbkit::pick::Menu = crumbkit::pick::Menu::new();
    const TOOL: Tool =
        Tool::new()
            .help(Help::new().leaves(&[
                Leaf::new(&["books", "show"]).choices(&[Choices::new("ids", catalogue)]),
            ]))
            .picker(&MENU);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = crumbkit::Sink::with_input(&mut out, &mut err, &b"austen\n1\n"[..], true);
    let parsed = crumbkit::parse_to::<crate::Cli>(&mut sink, &TOOL, ["shelf", "books", "show"]);

    assert!(parsed.is_ok());
    let err = String::from_utf8(err).unwrap();
    assert!(
        err.contains("  1. Pride and Prejudice  Jane Austen"),
        "{err}"
    );
    assert!(err.ends_with("Running: shelf books show 1\n"), "{err}");
}
