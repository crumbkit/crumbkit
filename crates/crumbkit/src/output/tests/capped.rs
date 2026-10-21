//! Tests for how `finish_to` prints a listing from a capped source, which
//! gives the number of rows the tool read as a lower bound (ADR-0130).

use clap::{CommandFactory as _, Parser};

use super::{
    PLANET_LISTING, Planet, Tool, built_with, finish_output, finish_with, mercury, saturn,
};
use crate::help::Guide;
use crate::{Call, Listing, Page};

/// Returns `count` planets, alternating Mercury and Saturn.
fn planets(count: usize) -> Vec<Planet> {
    (0..count)
        .map(|n| if n % 2 == 0 { mercury() } else { saturn() })
        .collect()
}

/// Returns a listing of the first 2 of the 100 planets that the tool read
/// from a source that holds more.
fn two_of_at_least_100() -> Listing<Planet> {
    Listing::new(planets(2), 100).capped(true)
}

#[test]
fn a_capped_listing_with_fewer_rows_than_it_read_prints_the_lower_bound_and_the_number_all_shows() {
    let finished = finish_with(
        &[],
        PLANET_LISTING,
        two_of_at_least_100().described("planets"),
    );

    assert_eq!(finished.out, "NAME     MOONS\nMercury  0\nSaturn   274\n");
    assert_eq!(
        finished.err,
        "Showing 2 of at least 100 planets. --all shows 100 of them.\n"
    );
}

#[test]
fn a_capped_listing_with_a_limit_bound_names_the_bound() {
    let listing = two_of_at_least_100().described("planets").bounded(100);

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(
        finished.err,
        "Showing 2 of at least 100 planets. --limit takes up to 100, and --all shows 100 of them.\n"
    );
}

#[test]
fn a_capped_listing_without_a_description_gives_the_lower_bound_alone() {
    let finished = finish_with(&[], PLANET_LISTING, two_of_at_least_100());

    assert_eq!(
        finished.err,
        "Showing 2 of at least 100. --all shows 100 of them.\n"
    );
}

#[test]
fn a_capped_listing_that_shows_every_row_it_read_says_more_match_and_offers_no_all() {
    let listing = Listing::new(planets(3), 3)
        .capped(true)
        .described("planets")
        .bounded(100);

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(finished.err, "Showing 3 planets, and more match.\n");
}

#[test]
fn a_capped_listing_that_shows_its_one_row_names_it_with_the_singular() {
    let listing = Listing::new(planets(1), 1)
        .capped(true)
        .described("planets")
        .singular("planet");

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(finished.err, "Showing 1 planet, and more match.\n");
}

#[test]
fn a_capped_listing_without_a_description_that_shows_every_row_names_rows_or_one_row() {
    let many = finish_with(
        &[],
        PLANET_LISTING,
        Listing::new(planets(2), 2).capped(true),
    );
    let one = finish_with(
        &[],
        PLANET_LISTING,
        Listing::new(planets(1), 1).capped(true),
    );

    assert_eq!(many.err, "Showing 2 rows, and more match.\n");
    assert_eq!(one.err, "Showing 1 row, and more match.\n");
}

#[test]
fn a_capped_listings_json_document_leaves_out_total_and_is_truncated() {
    let finished = finish_with(&["--json"], PLANET_LISTING, two_of_at_least_100());

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}],"truncated":true}"#,
            "\n"
        )
    );
}

#[test]
fn the_json_document_of_a_capped_listing_that_shows_every_row_it_read_is_truncated() {
    let listing = Listing::new(vec![mercury()], 1).capped(true);

    let finished = finish_with(&["--json"], PLANET_LISTING, listing);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[{"name":"Mercury","moons":0}],"truncated":true}"#,
            "\n"
        )
    );
}

#[test]
fn a_listing_that_is_not_capped_prints_as_before() {
    let listing = || Listing::new(planets(2), 100).described("planets");

    let text = finish_with(&[], PLANET_LISTING, listing().capped(false));
    let json = finish_with(&["--json"], PLANET_LISTING, listing().capped(false));

    assert_eq!(text.err, finish_with(&[], PLANET_LISTING, listing()).err);
    assert_eq!(
        text.err,
        "Showing 2 of 100 planets. --all shows every one.\n"
    );
    assert!(
        json.out.contains(r#""total":100,"truncated":true"#),
        "{}",
        json.out
    );
}

#[test]
fn a_capped_listing_prints_the_tools_note_and_next_step_after_its_footer() {
    let listing = two_of_at_least_100()
        .described("planets")
        .note("Read the newest 100 planets.")
        .next(Call::new(&["show"]).arg("names"), "shows planets by name.");

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(
        finished.err,
        "Showing 2 of at least 100 planets. --all shows 100 of them.\n\
         Read the newest 100 planets.\n\
         Next: tool show <NAME>... shows planets by name.\n"
    );
}

#[test]
fn a_capped_listing_cut_by_the_budget_gives_no_count() {
    let args = ["tool", "list"];
    let guide = Guide::new().output_budget(64, "the tests print little");
    let output = built_with(Tool::command(), &guide, &args);
    let listing = Listing::new(planets(40), 40)
        .capped(true)
        .described("planets");

    let finished = finish_output(output, PLANET_LISTING, listing);

    let shown = finished.out.lines().count() - 1;
    assert!(shown < 40, "{}", finished.out);
    assert_eq!(
        finished.err,
        format!(
            "Showing {shown} planets, because output stops at 64 bytes. \
             Ask for less with --limit or --fields.\n"
        )
    );
}

/// A command line that holds only the paging flags of a page of 2 rows
/// with a bound of 100.
#[derive(Parser)]
struct Paged {
    #[command(flatten)]
    page: Page<2, 100>,
}

/// Returns the page that `flags` ask for.
fn page(flags: &[&str]) -> Page<2, 100> {
    let args = std::iter::once("tool").chain(flags.iter().copied());
    Paged::try_parse_from(args).unwrap().page
}

#[test]
fn a_capped_page_from_first_of_names_its_bound_and_with_all_says_more_match() {
    let limited = page(&[]).first_of(planets(5)).capped(true);
    let every = page(&["--all"]).first_of(planets(5)).capped(true);

    let limited = finish_with(&[], PLANET_LISTING, limited.described("planets"));
    let every = finish_with(&[], PLANET_LISTING, every.described("planets"));

    assert_eq!(
        limited.err,
        "Showing 2 of at least 5 planets. --limit takes up to 100, and --all shows 5 of them.\n"
    );
    assert_eq!(every.err, "Showing 5 planets, and more match.\n");
}

#[test]
fn a_capped_page_from_listing_gives_the_lower_bound() {
    let listing = page(&[]).listing(planets(2), 100).capped(true);

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(
        finished.err,
        "Showing 2 of at least 100. --limit takes up to 100, and --all shows 100 of them.\n"
    );
}

#[test]
fn capping_an_uncounted_listing_prints_as_the_uncounted_listing_does() {
    for more in [false, true] {
        let listing = || Listing::uncounted(planets(2), more).described("planets");

        let capped = finish_with(&[], PLANET_LISTING, listing().capped(true));
        let plain = finish_with(&[], PLANET_LISTING, listing());

        assert_eq!(capped.err, plain.err, "more: {more}");
    }
}
