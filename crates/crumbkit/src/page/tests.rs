//! Tests for the paging flags.

use clap::error::ErrorKind;
use clap::{CommandFactory as _, Parser, Subcommand};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::json;

use super::{LimitAbove, Page};
use crate::{Listing, Row, Rows as _};

/// A shelf, the row type of the listings that a page builds.
#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema)]
struct Shelf {
    /// The shelf's number.
    number: u32,
}

impl Row for Shelf {
    const COLUMNS: &'static [&'static str] = &["number"];
}

/// Returns the shelves numbered 1 to `count`.
fn rows(count: u32) -> Vec<Shelf> {
    (1..=count).map(|number| Shelf { number }).collect()
}

/// A tool with two commands that flatten pages with different defaults.
#[derive(Parser)]
struct Tool {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// A command with the default page size.
    Books {
        #[command(flatten)]
        page: Page,
    },
    /// A command with a page size of its own.
    Authors {
        #[command(flatten)]
        page: Page<4>,
    },
    /// A command whose --limit takes at most 10.
    Shelves {
        #[command(flatten)]
        page: Page<4, 10>,
    },
}

/// Parses `tool books` followed by `flags`.
fn books(flags: &[&str]) -> Result<Page, clap::Error> {
    let args = ["tool", "books"].iter().chain(flags);
    match Tool::try_parse_from(args)?.command {
        Command::Books { page } => Ok(page),
        _ => unreachable!("the arguments name the books command"),
    }
}

/// Parses `tool authors` followed by `flags`.
fn authors(flags: &[&str]) -> Result<Page<4>, clap::Error> {
    let args = ["tool", "authors"].iter().chain(flags);
    match Tool::try_parse_from(args)?.command {
        Command::Authors { page } => Ok(page),
        _ => unreachable!("the arguments name the authors command"),
    }
}

/// Parses `tool shelves` followed by `flags`.
fn shelves(flags: &[&str]) -> Result<Page<4, 10>, clap::Error> {
    let args = ["tool", "shelves"].iter().chain(flags);
    match Tool::try_parse_from(args)?.command {
        Command::Shelves { page } => Ok(page),
        _ => unreachable!("the arguments name the shelves command"),
    }
}

/// Returns the help text of `--limit` that the subcommand `name` declares.
fn limit_help(name: &str) -> String {
    let tool = Tool::command();
    let command = tool.find_subcommand(name).unwrap();
    let limit = command
        .get_arguments()
        .find(|arg| arg.get_id() == "limit")
        .unwrap();
    limit.get_help().unwrap().to_string()
}

#[test]
fn a_command_that_flattens_page_passes_clap_debug_assertions() {
    Tool::command().debug_assert();
}

#[test]
fn the_limit_defaults_to_25_when_the_command_names_no_size() {
    assert_eq!(books(&[]).unwrap().limit(), Some(25));
}

#[test]
fn the_limit_defaults_to_the_size_the_command_names() {
    assert_eq!(authors(&[]).unwrap().limit(), Some(4));
}

#[test]
fn two_commands_with_different_sizes_each_keep_their_own_default() {
    // Parsing one command first must not change the other's default.
    assert_eq!(authors(&[]).unwrap().limit(), Some(4));
    assert_eq!(books(&[]).unwrap().limit(), Some(25));
    assert_eq!(authors(&[]).unwrap().limit(), Some(4));
}

#[test]
fn limit_sets_the_number_of_rows() {
    assert_eq!(books(&["--limit", "7"]).unwrap().limit(), Some(7));
}

#[test]
fn all_returns_no_limit() {
    assert_eq!(books(&["--all"]).unwrap().limit(), None);
}

/// Returns the reason that clap's `error` gives for a refused value, the
/// text after the value and the flag.
fn reason(error: &clap::Error) -> String {
    assert_eq!(error.kind(), ErrorKind::ValueValidation, "{error}");
    std::error::Error::source(error).unwrap().to_string()
}

#[test]
fn a_limit_of_0_is_refused_with_the_range_of_an_unbounded_page() {
    let error = books(&["--limit", "0"]).unwrap_err();

    assert_eq!(reason(&error), "--limit takes a whole number of at least 1");
}

#[test]
fn a_limit_that_is_not_a_number_is_refused_with_the_range_of_an_unbounded_page() {
    let error = books(&["--limit", "abc"]).unwrap_err();

    assert_eq!(reason(&error), "--limit takes a whole number of at least 1");
}

#[test]
fn a_negative_limit_is_refused_with_the_range_of_an_unbounded_page() {
    let error = books(&["--limit=-1"]).unwrap_err();

    assert_eq!(reason(&error), "--limit takes a whole number of at least 1");
}

#[test]
fn a_limit_of_0_is_refused_with_the_range_of_a_bounded_page() {
    let error = shelves(&["--limit", "0"]).unwrap_err();

    assert_eq!(reason(&error), "--limit takes a whole number from 1 to 10");
}

#[test]
fn a_limit_that_is_not_a_number_is_refused_with_the_range_of_a_bounded_page() {
    let error = shelves(&["--limit", "abc"]).unwrap_err();

    assert_eq!(reason(&error), "--limit takes a whole number from 1 to 10");
}

#[test]
fn a_negative_limit_is_refused_with_the_range_of_a_bounded_page() {
    let error = shelves(&["--limit=-1"]).unwrap_err();

    assert_eq!(reason(&error), "--limit takes a whole number from 1 to 10");
}

#[test]
fn an_unbounded_limit_too_large_for_usize_is_refused_with_the_largest_value() {
    let error = books(&["--limit", "99999999999999999999999"]).unwrap_err();

    assert_eq!(
        reason(&error),
        format!("--limit takes a whole number from 1 to {}", usize::MAX)
    );
}

#[test]
fn a_bounded_limit_too_large_for_usize_is_refused_as_above_its_bound() {
    let error = shelves(&["--limit", "99999999999999999999999"]).unwrap_err();

    let source = std::error::Error::source(&error).unwrap();
    assert_eq!(source.downcast_ref::<LimitAbove>().unwrap().most, 10);
}

#[test]
fn all_conflicts_with_limit() {
    let error = books(&["--all", "--limit", "2"]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
}

/// Returns the default of `--limit` that the subcommand `name` declares,
/// which clap's help shows as `[default: N]`.
fn declared_default(name: &str) -> String {
    let tool = Tool::command();
    let command = tool.find_subcommand(name).unwrap();
    let limit = command
        .get_arguments()
        .find(|arg| arg.get_id() == "limit")
        .unwrap();
    assert_eq!(limit.get_value_names().unwrap(), ["N"]);
    let [default] = limit.get_default_values() else {
        panic!("--limit has one default");
    };
    default.to_string_lossy().into_owned()
}

#[test]
fn each_command_declares_its_own_default_limit_for_its_help() {
    assert_eq!(declared_default("books"), "25");
    assert_eq!(declared_default("authors"), "4");
}

#[test]
fn a_page_serializes_its_limit_and_all() {
    let page = authors(&["--all"]).unwrap();

    assert_eq!(
        serde_json::to_value(&page).unwrap(),
        json!({"limit": 4, "all": true})
    );
}

#[test]
fn the_schema_describes_limit_with_its_default_and_all() {
    let schema = schemars::schema_for!(Page<4>).to_value();

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["required"], json!(["limit", "all"]));
    assert_eq!(schema["properties"]["limit"]["type"], "integer");
    assert_eq!(schema["properties"]["limit"]["minimum"], 1);
    assert_eq!(schema["properties"]["limit"]["default"], 4);
    assert_eq!(schema["properties"]["all"]["type"], "boolean");
    assert_eq!(schema["properties"]["all"]["default"], false);
}

#[test]
fn the_schema_describes_each_flag_with_the_text_of_its_help() {
    let schema = schemars::schema_for!(Page<4>).to_value();
    let tool = Tool::command();
    let authors = tool.find_subcommand("authors").unwrap();

    for id in ["limit", "all"] {
        let arg = authors
            .get_arguments()
            .find(|arg| arg.get_id() == id)
            .unwrap();
        assert_eq!(
            schema["properties"][id]["description"],
            arg.get_help().unwrap().to_string(),
            "--{id}"
        );
    }
}

#[test]
fn a_bounded_limit_takes_a_value_up_to_its_bound() {
    assert_eq!(shelves(&["--limit", "10"]).unwrap().limit(), Some(10));
}

#[test]
fn a_bounded_limit_refuses_a_value_above_its_bound_and_says_the_range() {
    let error = shelves(&["--limit", "11"]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::ValueValidation);
    let source = std::error::Error::source(&error).unwrap();
    let above = source.downcast_ref::<LimitAbove>().unwrap();
    assert_eq!(above.most, 10);
    assert_eq!(
        above.to_string(),
        "--limit takes a whole number from 1 to 10"
    );
}

#[test]
fn a_bounded_page_still_takes_all() {
    assert_eq!(shelves(&["--all"]).unwrap().limit(), None);
}

#[test]
fn an_unbounded_limit_takes_any_value_above_0() {
    let most = usize::MAX.to_string();

    assert_eq!(
        books(&["--limit", &most]).unwrap().limit(),
        Some(usize::MAX)
    );
}

#[test]
fn most_returns_the_bound_or_none_without_one() {
    assert_eq!(shelves(&[]).unwrap().most(), Some(10));
    assert_eq!(books(&[]).unwrap().most(), None);
}

#[test]
fn the_help_of_a_bounded_limit_names_the_bound() {
    assert_eq!(
        limit_help("shelves"),
        "Print at most this many rows, up to 10"
    );
    assert_eq!(limit_help("books"), "Print at most this many rows");
}

#[test]
fn the_schema_of_a_bounded_page_has_the_bound_as_the_maximum() {
    let bounded = schemars::schema_for!(Page<4, 10>).to_value();
    let unbounded = schemars::schema_for!(Page<4>).to_value();

    assert_eq!(bounded["properties"]["limit"]["maximum"], 10);
    assert_eq!(
        bounded["properties"]["limit"]["description"],
        limit_help("shelves")
    );
    assert_eq!(unbounded["properties"]["limit"].get("maximum"), None);
    assert_ne!(Page::<4, 10>::schema_id(), Page::<4>::schema_id());
}

#[test]
fn first_of_keeps_the_first_page_and_carries_the_bound() {
    let page = shelves(&["--limit", "2"]).unwrap();

    let listing = page.first_of(rows(3));

    assert_eq!(listing, Listing::from_all(rows(3), Some(2)).bounded(10));
}

#[test]
fn listing_keeps_the_rows_and_the_total_and_carries_the_bound() {
    let page = shelves(&[]).unwrap();

    let listing = page.listing(rows(2), 40);

    assert_eq!(listing.limit_bound(), Some(10));
    assert_eq!(listing.total(), Some(40));
}

#[test]
fn a_listing_from_an_unbounded_page_has_no_bound() {
    let page = books(&[]).unwrap();

    assert_eq!(page.first_of(rows(1)).limit_bound(), None);
}

#[test]
fn uncounted_keeps_the_first_page_and_says_more_rows_exist_when_the_source_has_one_more() {
    let page = shelves(&["--limit", "2"]).unwrap();

    let listing = page.uncounted(rows(3));

    assert_eq!(listing, Listing::uncounted(rows(2), true).bounded(10));
}

#[test]
fn uncounted_counts_the_rows_when_the_source_ends_within_the_page() {
    let page = shelves(&["--limit", "2"]).unwrap();

    let listing = page.uncounted(rows(2));

    assert_eq!(listing, Listing::new(rows(2), 2).bounded(10));
}

#[test]
fn uncounted_stops_reading_the_source_one_row_past_the_page() {
    let page = shelves(&["--limit", "2"]).unwrap();
    let mut read = 0;
    let source = (1..).map(|number| {
        read += 1;
        Shelf { number }
    });

    let listing = page.uncounted(source);

    assert_eq!(listing.total(), None);
    assert_eq!(read, 3);
}

#[test]
fn uncounted_with_all_reads_every_row_and_counts_them() {
    let page = shelves(&["--all"]).unwrap();

    let listing = page.uncounted(rows(30));

    assert_eq!(listing, Listing::new(rows(30), 30).bounded(10));
}

#[test]
fn fetch_limit_is_one_past_the_limit_and_none_with_all() {
    assert_eq!(shelves(&["--limit", "2"]).unwrap().fetch_limit(), Some(3));
    assert_eq!(shelves(&["--all"]).unwrap().fetch_limit(), None);
}
