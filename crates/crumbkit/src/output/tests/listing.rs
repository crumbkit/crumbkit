//! Tests for how `finish_to` prints a listing: the total and `truncated`
//! in JSON, and the footer, the note and the `Next:` line on standard error in text.

use std::process::ExitCode;

use clap::{Arg, Command, CommandFactory as _};

use super::{
    PLANET_LISTING, Planet, Tool, finish_output, finish_with, mercury, parse_against, saturn,
};
use crate::{Call, Listing};

/// Returns a listing of Mercury and Saturn out of `total` planets.
fn two_of(total: usize) -> Listing<Planet> {
    Listing::new(vec![mercury(), saturn()], total)
}

/// Returns a listing of Mercury and Saturn out of `total` planets whose next
/// step is `tool show <NAME>...`.
fn two_of_with_next(total: usize) -> Listing<Planet> {
    two_of(total).next(Call::new(&["show"]).arg("names"), "shows planets by name.")
}

#[test]
fn a_listing_that_shows_fewer_rows_than_the_total_has_a_footer_on_standard_error() {
    let finished = finish_with(&[], PLANET_LISTING, two_of(8));

    let expected = "\
NAME     MOONS
Mercury  0
Saturn   274
";
    assert_eq!(finished.out, expected);
    assert_eq!(finished.err, "Showing 2 of 8. --all shows every one.\n");
}

#[test]
fn the_footer_names_the_rows_with_the_listings_description() {
    let finished = finish_with(&[], PLANET_LISTING, two_of(8).described("planets"));

    assert_eq!(
        finished.err,
        "Showing 2 of 8 planets. --all shows every one.\n"
    );
}

#[test]
fn a_listing_that_shows_every_row_has_no_footer() {
    let finished = finish_with(&[], PLANET_LISTING, two_of(2).described("planets"));

    assert_eq!(finished.out, "NAME     MOONS\nMercury  0\nSaturn   274\n");
    assert_eq!(finished.err, "");
}

#[test]
fn the_footer_of_a_listing_without_a_total_says_more_rows_match_without_a_count() {
    let listing = Listing::uncounted(vec![mercury(), saturn()], true).described("planets");

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(
        finished.err,
        "Showing 2 planets, and more match. --all shows every one.\n"
    );
}

#[test]
fn the_footer_of_a_listing_without_a_total_or_a_description_names_rows() {
    let listing = Listing::uncounted(vec![mercury(), saturn()], true);

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(
        finished.err,
        "Showing 2 rows, and more match. --all shows every one.\n"
    );
}

#[test]
fn the_footer_of_a_bounded_listing_without_a_total_names_the_bound() {
    let listing = Listing::uncounted(vec![mercury(), saturn()], true)
        .described("planets")
        .bounded(100);

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(
        finished.err,
        "Showing 2 planets, and more match. --limit takes up to 100, and --all shows every one.\n"
    );
}

#[test]
fn a_listings_json_without_a_total_has_truncated_and_no_total() {
    let listing = Listing::uncounted(vec![mercury(), saturn()], true);

    let finished = finish_with(&["--json"], PLANET_LISTING, listing);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}],"truncated":true}"#,
            "\n"
        )
    );
}

#[test]
fn the_footer_note_and_next_line_are_the_same_with_long_and_with_fields() {
    let plain = finish_with(&[], PLANET_LISTING, two_of_with_note(8));
    let long = finish_with(&["--long"], PLANET_LISTING, two_of_with_note(8));
    let fields = finish_with(&["--fields", "au"], PLANET_LISTING, two_of_with_note(8));

    assert_eq!(
        long.out,
        "NAME     MOONS  RINGED\nMercury  0      false\nSaturn   274    true\n"
    );
    assert_eq!(fields.out, "AU\n0.39\n9.54\n");
    assert_eq!(long.err, plain.err);
    assert_eq!(fields.err, plain.err);
}

#[test]
fn a_listings_json_has_the_total_and_truncated_beside_the_data_and_no_description() {
    let finished = finish_with(&["--json"], PLANET_LISTING, two_of(8).described("planets"));

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}],"total":8,"truncated":true}"#,
            "\n"
        )
    );
}

#[test]
fn a_listings_json_that_holds_every_row_has_the_total_and_truncated_false() {
    let finished = finish_with(&["--json"], PLANET_LISTING, Listing::new(Vec::new(), 0));

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[],"total":0,"truncated":false}"#,
            "\n"
        )
    );
}

#[test]
fn a_listings_json_has_the_total_with_full() {
    let listing = Listing::new(vec![saturn()], 8);
    let finished = finish_with(&["--json", "--full"], PLANET_LISTING, listing);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[{"name":"Saturn","moons":274,"ringed":true,"au":9.54}],"total":8,"truncated":true}"#,
            "\n"
        )
    );
}

#[test]
fn a_listings_json_has_the_total_with_fields() {
    let listing = Listing::new(vec![saturn()], 8);
    let finished = finish_with(&["--json", "--fields", "au"], PLANET_LISTING, listing);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[{"au":9.54}],"total":8,"truncated":true}"#,
            "\n"
        )
    );
}

#[test]
fn the_next_line_follows_the_footer() {
    let finished = finish_with(&[], PLANET_LISTING, two_of_with_next(8));

    let expected = "\
Showing 2 of 8. --all shows every one.
Next: tool show <NAME>... shows planets by name.
";
    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.err, expected);
}

#[test]
fn the_next_line_is_the_only_note_when_there_is_no_footer() {
    let finished = finish_with(&[], PLANET_LISTING, two_of_with_next(2));

    assert_eq!(
        finished.err,
        "Next: tool show <NAME>... shows planets by name.\n"
    );
}

#[test]
fn json_output_has_no_next_line_and_no_next_field() {
    let finished = finish_with(&["--json"], PLANET_LISTING, two_of_with_next(8));

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}],"total":8,"truncated":true}"#,
            "\n"
        )
    );
    assert_eq!(finished.err, "");
}

/// Returns [`two_of_with_next`] with the note `Planets from the 2006
/// definition.`
fn two_of_with_note(total: usize) -> Listing<Planet> {
    two_of_with_next(total).note("Planets from the 2006 definition.")
}

#[test]
fn a_note_prints_after_the_footer_and_before_the_next_line() {
    let finished = finish_with(&[], PLANET_LISTING, two_of_with_note(8));

    let expected = "\
Showing 2 of 8. --all shows every one.
Planets from the 2006 definition.
Next: tool show <NAME>... shows planets by name.
";
    assert_eq!(finished.err, expected);
}

#[test]
fn json_output_has_no_note() {
    let compact = finish_with(&["--json"], PLANET_LISTING, two_of_with_note(8));
    let full = finish_with(&["--json", "--full"], PLANET_LISTING, two_of_with_note(8));

    assert!(!compact.out.contains("2006"), "{}", compact.out);
    assert!(!full.out.contains("2006"), "{}", full.out);
}

#[test]
fn a_next_step_whose_call_does_not_resolve_is_left_out_and_the_exit_code_is_0() {
    let listing = two_of(8).next(Call::new(&["remove"]), "removes a planet.");

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.err, "Showing 2 of 8. --all shows every one.\n");
}

#[test]
fn a_next_step_into_a_branch_the_run_did_not_parse_names_a_global_flag_of_the_root() {
    // The run parses `tool list`, so clap builds neither `moons` nor
    // `moons show`, and only a build of `moons` copies `--json` into `show`.
    let names = Arg::new("names")
        .required(true)
        .num_args(1..)
        .value_name("NAME");
    let moons = Command::new("moons").subcommand(Command::new("show").arg(names));
    let mut output = parse_against(Tool::command().subcommand(moons), &[]);
    let call = Call::new(&["moons", "show"]).arg("names").arg("json");
    let listing = two_of(2).next(call, "shows moons as JSON.");

    let finished = finish_output(&mut output, PLANET_LISTING, listing);

    assert_eq!(
        finished.err,
        "Next: tool moons show <NAME>... --json shows moons as JSON.\n"
    );
}
