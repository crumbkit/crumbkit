//! Tests for the note of a listing with no matching rows, and for the bound
//! that a listing's footer names (ADR-0050).

use std::process::ExitCode;

use super::{PLANET_LISTING, Planet, finish_with, mercury, saturn};
use crate::{Call, Document, Listing};

/// Returns a listing with no rows out of no matching rows.
fn none() -> Listing<Planet> {
    Listing::new(Vec::new(), 0)
}

#[test]
fn a_listing_with_no_matching_rows_prints_a_note_after_its_header() {
    let finished = finish_with(&[], PLANET_LISTING, none().described("planets"));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, "NAME  MOONS\nnote: no planets match\n");
    assert_eq!(finished.err, "");
}

#[test]
fn the_note_of_a_listing_without_a_description_names_rows() {
    let finished = finish_with(&[], PLANET_LISTING, none());

    assert_eq!(finished.out, "NAME  MOONS\nnote: no rows match\n");
}

#[test]
fn a_document_sets_its_own_wording_of_the_note() {
    let document = PLANET_LISTING.when_empty("the system has no planets");

    let finished = finish_with(&[], document, none().described("planets"));

    assert_eq!(
        finished.out,
        "NAME  MOONS\nnote: the system has no planets\n"
    );
}

#[test]
fn the_note_comes_before_the_listings_own_note_and_next_step() {
    let listing = none()
        .described("planets")
        .note("Catalogue: solar.toml")
        .next(Call::new(&["show"]).arg("names"), "shows planets by name.");

    let finished = finish_with(&[], PLANET_LISTING, listing);

    let expected = "\
NAME  MOONS
note: no planets match
Catalogue: solar.toml
Next: tool show <NAME>... shows planets by name.
";
    assert_eq!(finished.out, expected);
}

#[test]
fn a_listing_whose_page_is_empty_but_whose_total_is_not_prints_the_footer_and_no_note() {
    let listing = Listing::new(Vec::new(), 3).described("planets");

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(
        finished.out,
        "NAME  MOONS\nShowing 0 of 3 planets. --all shows every one.\n"
    );
}

#[test]
fn the_json_of_an_unwrapped_listing_with_no_rows_is_an_empty_array_and_nothing_else() {
    let document: Document<Listing<Planet>> = PLANET_LISTING.without_envelope();

    let finished = finish_with(&["--json"], document, none().described("planets"));

    assert_eq!(finished.out, "[]\n");
    assert_eq!(finished.err, "");
}

#[test]
fn the_footer_of_a_bounded_listing_names_the_bound() {
    let listing = Listing::new(vec![mercury(), saturn()], 8)
        .described("planets")
        .bounded(100);

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert!(
        finished.out.ends_with(
            "Showing 2 of 8 planets. --limit takes up to 100, and --all shows every one.\n"
        ),
        "{}",
        finished.out
    );
}

#[test]
fn the_footer_of_a_bounded_listing_without_a_description_names_the_bound() {
    let listing = Listing::new(vec![mercury()], 8).bounded(100);

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert!(
        finished
            .out
            .ends_with("Showing 1 of 8. --limit takes up to 100, and --all shows every one.\n"),
        "{}",
        finished.out
    );
}
