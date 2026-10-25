//! Tests for how `finish_to` prints a listing's summary: lines above the
//! table in text, and an object under `summary` in JSON.

use std::fmt;

use clap::CommandFactory as _;
use schemars::JsonSchema;
use serde::Serialize;

use super::{Planet, Tool, built_with, finish_output, finish_with, mercury, saturn};
use crate::help::Guide;
use crate::{Call, Document, Listing};

/// The summary of a listing of planets.
#[derive(Serialize, JsonSchema)]
struct Census {
    planets: u32,
    star: &'static str,
}

impl fmt::Display for Census {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} planets around the {}.", self.planets, self.star)
    }
}

const CENSUS: Document<Listing<Planet, Census>> =
    Document::new("solar", "census", 1, "The test rows.");

/// The summary of a listing of planets whose `Display` writes `text`, which
/// may be empty or only whitespace.
#[derive(Serialize, JsonSchema)]
struct Quiet {
    planets: u32,
    #[serde(skip)]
    text: &'static str,
}

impl fmt::Display for Quiet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text)
    }
}

const QUIET: Document<Listing<Planet, Quiet>> =
    Document::new("solar", "quiet", 1, "The test rows.");

/// Returns a listing of Mercury and Saturn with a summary whose `Display`
/// writes `text`.
fn quiet(text: &'static str) -> Listing<Planet, Quiet> {
    Listing::new(vec![mercury(), saturn()], 2).summary(Quiet { planets: 2, text })
}

/// Returns a listing of Mercury and Saturn out of 8 planets, with a summary
/// and a note.
fn census() -> Listing<Planet, Census> {
    Listing::new(vec![mercury(), saturn()], 8)
        .summary(Census {
            planets: 8,
            star: "Sun",
        })
        .note("Planets from the 2006 definition.")
}

#[test]
fn a_summary_prints_above_the_table_in_text() {
    let finished = finish_with(&[], CENSUS, census());

    let expected = "\
8 planets around the Sun.
NAME     MOONS
Mercury  0
Saturn   274
";
    assert_eq!(finished.out, expected);
    assert_eq!(
        finished.err,
        "Showing 2 of 8. --all shows every one.\nPlanets from the 2006 definition.\n"
    );
}

#[test]
fn json_holds_the_summary_as_an_object_after_the_total() {
    let compact = finish_with(&["--json"], CENSUS, census());
    let full = finish_with(&["--json", "--full"], CENSUS, census());

    assert_eq!(
        compact.out,
        concat!(
            r#"{"schema":"solar/census/v1","data":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}],"#,
            r#""total":8,"truncated":true,"summary":{"planets":8,"star":"Sun"}}"#,
            "\n"
        )
    );
    assert!(
        full.out.ends_with(
            "\"total\":8,\"truncated\":true,\"summary\":{\"planets\":8,\"star\":\"Sun\"}}\n"
        ),
        "{}",
        full.out
    );
}

#[test]
fn a_listing_whose_summary_writes_no_text_prints_the_header_as_its_first_line() {
    let finished = finish_with(&[], QUIET, quiet(""));

    assert_eq!(finished.out, "NAME     MOONS\nMercury  0\nSaturn   274\n");
}

#[test]
fn the_json_of_a_listing_whose_summary_writes_no_text_still_holds_the_summary() {
    let finished = finish_with(&["--json"], QUIET, quiet(""));

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/quiet/v1","data":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}],"#,
            r#""total":2,"truncated":false,"summary":{"planets":2}}"#,
            "\n"
        )
    );
}

#[test]
fn a_listing_whose_summary_writes_text_still_prints_it_above_the_header() {
    let finished = finish_with(&[], QUIET, quiet("2 planets."));

    assert_eq!(
        finished.out,
        "2 planets.\nNAME     MOONS\nMercury  0\nSaturn   274\n"
    );
}

#[test]
fn a_summary_that_writes_only_whitespace_still_prints_its_line() {
    let finished = finish_with(&[], QUIET, quiet(" "));

    assert_eq!(
        finished.out,
        " \nNAME     MOONS\nMercury  0\nSaturn   274\n"
    );
}

#[test]
fn a_summary_that_writes_no_text_spends_none_of_the_output_budget() {
    let table = "NAME     MOONS\nMercury  0\nSaturn   274\n";
    let guide = Guide::new().output_budget(table.len(), "the tests print little");
    let output = built_with(Tool::command(), &guide, &["tool", "list"]);

    let finished = finish_output(output, QUIET, quiet(""));

    assert_eq!(finished.out, table);
    assert_eq!(finished.err, "");
}

/// Returns a listing of no planets out of none, with a summary.
fn empty_census() -> Listing<Planet, Census> {
    Listing::new(Vec::new(), 0)
        .described("planets")
        .summary(Census {
            planets: 0,
            star: "Sun",
        })
}

#[test]
fn a_listing_with_a_summary_and_no_rows_prints_the_summary_alone_without_a_header() {
    let finished = finish_with(&[], CENSUS, empty_census());

    assert_eq!(finished.out, "0 planets around the Sun.\n");
}

#[test]
fn a_listing_with_a_summary_and_no_rows_still_prints_its_empty_note_and_next_step() {
    let listing = empty_census().next(Call::new(&["show"]).arg("names"), "shows planets by name.");

    let finished = finish_with(&[], CENSUS, listing);

    assert_eq!(finished.out, "0 planets around the Sun.\n");
    assert_eq!(
        finished.err,
        "note: no planets match\nNext: tool show <NAME>... shows planets by name.\n"
    );
}

#[test]
fn a_listing_with_no_rows_whose_summary_writes_no_text_still_prints_its_header() {
    let listing = Listing::new(Vec::new(), 0).summary(Quiet {
        planets: 0,
        text: "",
    });

    let finished = finish_with(&[], QUIET, listing);

    assert_eq!(finished.out, "NAME  MOONS\n");
}

#[test]
fn a_listing_with_a_summary_and_no_rows_prints_no_header_with_long_or_fields() {
    let long = finish_with(&["--long"], CENSUS, empty_census());
    let fields = finish_with(&["--fields", "name"], CENSUS, empty_census());

    assert_eq!(long.out, "0 planets around the Sun.\n");
    assert_eq!(fields.out, "0 planets around the Sun.\n");
}

#[test]
fn a_listing_without_a_summary_and_without_rows_still_prints_its_header() {
    let listing: Listing<Planet> = Listing::new(Vec::new(), 0);

    let finished = finish_with(&[], super::PLANET_LISTING, listing);

    assert_eq!(finished.out, "NAME  MOONS\n");
}

#[test]
fn a_page_with_a_summary_and_no_rows_from_a_larger_total_prints_the_summary_and_the_footer() {
    let listing = Listing::new(Vec::new(), 3)
        .described("planets")
        .summary(Census {
            planets: 3,
            star: "Sun",
        });

    let finished = finish_with(&[], CENSUS, listing);

    assert_eq!(finished.out, "3 planets around the Sun.\n");
    assert_eq!(
        finished.err,
        "Showing 0 of 3 planets. --all shows every one.\n"
    );
}

#[test]
fn the_json_of_a_listing_with_a_summary_and_no_rows_is_unchanged() {
    let finished = finish_with(&["--json"], CENSUS, empty_census());

    assert_eq!(
        finished.out,
        "{\"schema\":\"solar/census/v1\",\"data\":[],\"total\":0,\"truncated\":false,\
         \"summary\":{\"planets\":0,\"star\":\"Sun\"}}\n"
    );
}

#[test]
fn a_summary_that_writes_several_lines_prints_each_above_the_table() {
    let finished = finish_with(&[], QUIET, quiet("2 planets.\n1 ringed.\n0 dwarfs."));

    assert_eq!(
        finished.out,
        "2 planets.\n1 ringed.\n0 dwarfs.\nNAME     MOONS\nMercury  0\nSaturn   274\n"
    );
}

#[test]
fn a_summary_that_ends_with_a_line_feed_prints_no_empty_line_above_the_header() {
    let finished = finish_with(&[], QUIET, quiet("2 planets.\n1 ringed.\n"));

    assert_eq!(
        finished.out,
        "2 planets.\n1 ringed.\nNAME     MOONS\nMercury  0\nSaturn   274\n"
    );
}

#[test]
fn an_empty_line_inside_a_summary_still_prints() {
    let finished = finish_with(&[], QUIET, quiet("2 planets.\n\n1 ringed."));

    assert_eq!(
        finished.out,
        "2 planets.\n\n1 ringed.\nNAME     MOONS\nMercury  0\nSaturn   274\n"
    );
}

#[test]
fn each_summary_line_escapes_its_other_control_characters() {
    let finished = finish_with(&[], QUIET, quiet("2\tplanets.\u{1b}[2J\n1 ringed.\r"));

    assert_eq!(
        finished.out,
        "2\\tplanets.\\u{1b}[2J\n1 ringed.\\r\nNAME     MOONS\nMercury  0\nSaturn   274\n"
    );
}

#[test]
fn a_listing_with_a_summary_of_several_lines_and_no_rows_prints_the_lines_alone() {
    let listing = Listing::new(Vec::new(), 0).summary(Quiet {
        planets: 0,
        text: "0 planets.\n0 ringed.",
    });

    let finished = finish_with(&[], QUIET, listing);

    assert_eq!(finished.out, "0 planets.\n0 ringed.\n");
}

#[test]
fn the_json_of_a_summary_that_writes_several_lines_is_unchanged() {
    let finished = finish_with(&["--json"], QUIET, quiet("2 planets.\n1 ringed."));

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/quiet/v1","data":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}],"#,
            r#""total":2,"truncated":false,"summary":{"planets":2}}"#,
            "\n"
        )
    );
}

#[test]
fn the_output_budget_counts_every_summary_line() {
    let whole = "2 planets.\n1 ringed.\nNAME     MOONS\nMercury  0\nSaturn   274\n";
    let fits = Guide::new().output_budget(whole.len(), "the tests print little");
    let short = Guide::new().output_budget(whole.len() - 1, "the tests print little");

    let fitted = finish_output(
        built_with(Tool::command(), &fits, &["tool", "list"]),
        QUIET,
        quiet("2 planets.\n1 ringed."),
    );
    let cut = finish_output(
        built_with(Tool::command(), &short, &["tool", "list"]),
        QUIET,
        quiet("2 planets.\n1 ringed."),
    );

    assert_eq!(fitted.out, whole);
    assert_eq!(
        cut.out,
        "2 planets.\n1 ringed.\nNAME     MOONS\nMercury  0\n"
    );
}

#[test]
fn a_summary_that_writes_only_a_line_feed_prints_one_empty_line() {
    let finished = finish_with(&[], QUIET, quiet("\n"));

    assert_eq!(finished.out, "\nNAME     MOONS\nMercury  0\nSaturn   274\n");
}
