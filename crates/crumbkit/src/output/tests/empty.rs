//! Tests for the note of a listing with no matching rows, and for the bound
//! that a listing's footer names (ADR-0050).

use std::process::ExitCode;

use super::{PLANET_LISTING, Planet, finish_with, mercury, saturn};
use crate::Envelope;
use crate::{Call, Document, Listing};

/// Returns a listing with no rows out of no matching rows.
fn none() -> Listing<Planet> {
    Listing::new(Vec::new(), 0)
}

#[test]
fn a_listing_with_no_matching_rows_prints_its_header_and_a_note_on_standard_error() {
    let finished = finish_with(&[], PLANET_LISTING, none().described("planets"));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, "NAME  MOONS\n");
    assert_eq!(finished.err, "note: no planets match\n");
}

#[test]
fn the_note_of_a_listing_without_a_description_names_rows() {
    let finished = finish_with(&[], PLANET_LISTING, none());

    assert_eq!(finished.err, "note: no rows match\n");
}

#[test]
fn a_document_sets_its_own_wording_of_the_note() {
    let document = PLANET_LISTING.when_empty("the system has no planets");

    let finished = finish_with(&[], document, none().described("planets"));

    assert_eq!(finished.err, "note: the system has no planets\n");
}

#[test]
fn the_note_comes_before_the_listings_own_note_and_next_step() {
    let listing = none()
        .described("planets")
        .note("Catalogue: solar.toml")
        .next(Call::new(&["show"]).arg("names"), "shows planets by name.");

    let finished = finish_with(&[], PLANET_LISTING, listing);

    let expected = "\
note: no planets match
Catalogue: solar.toml
Next: tool show <NAME>... shows planets by name.
";
    assert_eq!(finished.err, expected);
}

#[test]
fn a_listing_whose_page_is_empty_but_whose_total_is_not_prints_the_footer_and_no_note() {
    let listing = Listing::new(Vec::new(), 3).described("planets");

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(finished.out, "NAME  MOONS\n");
    assert_eq!(
        finished.err,
        "Showing 0 of 3 planets. --all shows every one.\n"
    );
}

#[test]
fn a_listing_without_a_total_whose_page_is_empty_but_has_more_rows_prints_the_footer_and_no_note() {
    let listing = Listing::uncounted(Vec::new(), true).described("planets");

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(
        finished.err,
        "Showing 0 planets, and more match. --all shows every one.\n"
    );
}

#[test]
fn an_uncounted_listing_with_no_rows_and_no_more_prints_the_note_and_no_total() {
    let listing = || Listing::uncounted(Vec::new(), false).described("planets");

    let text = finish_with(&[], PLANET_LISTING, listing());
    let json = finish_with(&["--json"], PLANET_LISTING, listing());

    assert_eq!(text.err, "note: no planets match\n");
    assert_eq!(
        json.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[],"truncated":false}"#,
            "\n"
        )
    );
}

#[test]
fn the_json_of_an_unwrapped_listing_with_no_rows_is_an_empty_array_and_nothing_else() {
    let document: Document<Listing<Planet>> = PLANET_LISTING.envelope(Envelope::Without);

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

    assert_eq!(
        finished.err,
        "Showing 2 of 8 planets. --limit takes up to 100, and --all shows every one.\n"
    );
}

#[test]
fn the_footer_of_a_bounded_listing_without_a_description_names_the_bound() {
    let listing = Listing::new(vec![mercury()], 8).bounded(100);

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(
        finished.err,
        "Showing 1 of 8. --limit takes up to 100, and --all shows every one.\n"
    );
}
