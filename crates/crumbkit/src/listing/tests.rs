//! Tests for listings.

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::json;

use super::{Listing, NoSummary, Total};
use crate::Rows;
use crate::row::Sealed;
use crate::{Call, Problem, Row, Shape, Step};

#[derive(Serialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
struct Book {
    id: u32,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id"];
}

/// Returns the books with the ids 1 to `count`.
fn books(count: u32) -> Vec<Book> {
    (1..=count).map(|id| Book { id }).collect()
}

#[test]
fn new_keeps_the_rows_and_the_total_it_is_given() {
    let listing = Listing::new(books(2), 41);

    assert_eq!(listing.rows(), books(2));
    assert_eq!(listing.total(), Total::Exact(41));
}

#[test]
fn a_total_smaller_than_the_rows_becomes_the_number_of_rows() {
    let listing = Listing::new(books(3), 1);

    assert_eq!(listing.total(), Total::Exact(3));
}

#[test]
fn an_uncounted_listing_with_more_rows_has_no_total() {
    let listing = Listing::uncounted(books(2), true);

    assert_eq!(listing.rows(), books(2));
    assert_eq!(listing.total(), Total::Unknown { more: true });
}

#[test]
fn an_uncounted_listing_without_more_rows_has_no_total() {
    let listing = Listing::uncounted(books(3), false);

    assert_eq!(listing.rows(), books(3));
    assert_eq!(listing.total(), Total::Unknown { more: false });
}

#[test]
fn from_all_with_a_limit_below_the_length_keeps_the_first_rows_and_counts_them_all() {
    let listing = Listing::from_all(books(6), Some(4));

    assert_eq!(listing, Listing::new(books(4), 6));
}

#[test]
fn from_all_with_a_limit_equal_to_the_length_keeps_every_row() {
    let listing = Listing::from_all(books(6), Some(6));

    assert_eq!(listing, Listing::new(books(6), 6));
}

#[test]
fn from_all_with_a_limit_above_the_length_keeps_every_row() {
    let listing = Listing::from_all(books(6), Some(10));

    assert_eq!(listing, Listing::new(books(6), 6));
}

#[test]
fn from_all_without_a_limit_keeps_every_row() {
    let listing = Listing::from_all(books(6), None);

    assert_eq!(listing, Listing::new(books(6), 6));
}

#[test]
fn a_listing_has_no_description_until_one_is_set() {
    let plain = Listing::new(books(1), 3);
    let described = Listing::new(books(1), 3).described("books by author 4");

    assert_eq!(plain.description(), None);
    assert_eq!(described.description(), Some("books by author 4"));
}

#[test]
fn a_listing_keeps_its_steps_in_the_order_they_were_added() {
    let show = Step::new("Show books by id.").call(Call::new(&["books", "show"]).arg("ids"));
    let list = Step::new("List the books.").call(Call::new(&["books", "list"]));

    let listing = Listing::new(books(1), 3)
        .step(show.clone())
        .step(list.clone());

    assert_eq!(listing.steps(), [show, list]);
}

#[test]
fn a_listing_serializes_as_the_array_of_its_rows() {
    let listing = Listing::new(books(2), 41).described("books");

    assert_eq!(
        serde_json::to_value(&listing).unwrap(),
        json!([{"id": 1}, {"id": 2}])
    );
}

#[test]
fn a_single_row_and_a_vec_have_no_total_no_description_and_no_steps() {
    let book = Book { id: 1 };

    assert_eq!(book.total(), None);
    assert_eq!(book.description(), None);
    assert_eq!(book.steps(), []);
    assert_eq!(books(2).total(), None);
    assert_eq!(books(2).description(), None);
    assert_eq!(books(2).steps(), []);
}

/// A summary of the books: how many there are, which prints as a line.
#[derive(Serialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
struct Shelved {
    books: u32,
}

impl std::fmt::Display for Shelved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} books on the shelf.", self.books)
    }
}

#[test]
fn a_listing_has_no_summary_until_one_is_set() {
    let plain = Listing::new(books(1), 3);
    let summarised = Listing::new(books(1), 3)
        .described("books")
        .summary(Shelved { books: 3 });

    // The builder `summary` shadows the `Sealed` method of the same name.
    assert_eq!(Sealed::summary(&plain), None);
    assert_eq!(Sealed::summary(&summarised), Some(&Shelved { books: 3 }));
    assert_eq!(summarised.description(), Some("books"));
}

#[test]
fn a_row_a_vec_and_a_listing_each_name_their_shape() {
    assert_eq!(<Book as Sealed>::SHAPE, Shape::Row);
    assert_eq!(<Vec<Book> as Sealed>::SHAPE, Shape::List);
    assert_eq!(<Listing<Book> as Sealed>::SHAPE, Shape::Listing);
    assert_eq!(<Listing<Book, Shelved> as Sealed>::SHAPE, Shape::Listing);
}

#[test]
fn the_schema_of_no_summary_is_false_and_inline_so_no_value_matches_it() {
    let mut generator = schemars::SchemaGenerator::default();

    assert_eq!(
        generator.subschema_for::<NoSummary>().as_bool(),
        Some(false)
    );
    assert!(generator.definitions().is_empty());
}

#[test]
fn a_listing_has_no_limit_bound_until_one_is_set() {
    let listing = Listing::new(books(2), 2);

    assert_eq!(listing.limit_bound(), None);
}

#[test]
fn bounded_sets_the_most_that_limit_takes() {
    let listing = Listing::new(books(2), 9).bounded(100);

    assert_eq!(listing.limit_bound(), Some(100));
}

#[test]
fn a_summary_keeps_the_limit_bound() {
    let listing = Listing::new(books(2), 9).bounded(100).summary(7_u32);

    assert_eq!(listing.limit_bound(), Some(100));
}

/// A row whose `problem` field marks a problem when it holds text.
#[derive(Serialize, JsonSchema)]
struct Check {
    problem: String,
}

impl Row for Check {
    const COLUMNS: &'static [&'static str] = &["problem"];
    const PROBLEM: Option<Problem<Self>> = Some(Problem::field("problem"));
}

/// Returns a check for each of `problems`, which reports a problem when its
/// text is not empty.
fn checks(problems: &[&str]) -> Vec<Check> {
    problems
        .iter()
        .map(|problem| Check {
            problem: (*problem).to_owned(),
        })
        .collect()
}

#[test]
fn from_all_counts_the_problem_rows_that_its_limit_cuts() {
    let listing = Listing::from_all(checks(&["", "missing", "broken"]), Some(1));

    assert_eq!(listing.rows().len(), 1);
    assert_eq!(listing.problems(), 2);
}

#[test]
fn from_all_without_a_limit_counts_the_problem_rows_it_holds() {
    let listing = Listing::from_all(checks(&["", "missing"]), None);

    assert_eq!(listing.problems(), 1);
}

#[test]
fn new_counts_only_the_problem_rows_it_holds() {
    let listing = Listing::new(checks(&["missing", ""]), 10);

    assert_eq!(listing.problems(), 1);
}

#[test]
fn a_summary_keeps_the_problem_count_of_the_rows_that_the_limit_cut() {
    let listing = Listing::from_all(checks(&["", "missing"]), Some(1)).summary(1);

    assert_eq!(listing.problems(), 1);
}

/// A row whose type decides its problems from a count with `Problem::by`.
#[derive(Serialize, JsonSchema)]
struct Run {
    errors: u32,
}

impl Row for Run {
    const COLUMNS: &'static [&'static str] = &["errors"];
    const PROBLEM: Option<Problem<Self>> = Some(Problem::by("errors", |run| run.errors > 0));
}

#[test]
fn from_all_counts_the_rows_that_a_problem_by_a_function_marks_among_the_rows_its_limit_cuts() {
    let runs = [0, 3, 1].map(|errors| Run { errors }).into();

    let listing = Listing::from_all(runs, Some(1));

    assert_eq!(listing.problems(), 2);
}

#[test]
fn a_capped_listing_has_a_lower_bound_and_no_exact_total() {
    let listing = Listing::new(books(2), 100).capped(true);

    assert_eq!(listing.total(), Total::AtLeast(100));
    assert_eq!(Rows::total(&listing), None);
    assert!(listing.more(), "a capped listing leaves rows out");
    assert_eq!(listing.at_least(), Some(100));
}

#[test]
fn a_capped_listing_that_holds_every_row_it_read_still_leaves_rows_out() {
    let listing = Listing::from_all(books(3), None).capped(true);

    assert!(listing.more(), "a capped listing leaves rows out");
    assert_eq!(listing.at_least(), Some(3));
}

#[test]
fn a_listing_that_is_not_capped_is_unchanged() {
    let listing = Listing::new(books(2), 100);

    assert_eq!(listing.clone().capped(false), listing);
    assert_eq!(listing.at_least(), None);
}

#[test]
fn capping_an_uncounted_listing_changes_nothing() {
    for more in [false, true] {
        let listing = Listing::uncounted(books(2), more);

        assert_eq!(listing.clone().capped(true), listing, "more: {more}");
    }
}

#[test]
fn a_summary_keeps_the_listings_cap() {
    let listing = Listing::new(books(2), 100).capped(true).summary(1);

    assert_eq!(listing.total(), Total::AtLeast(100));
    assert_eq!(listing.at_least(), Some(100));
}

#[test]
fn a_capped_listing_stays_capped_when_capped_false_follows() {
    let listing = Listing::new(books(2), 100).capped(true).capped(false);

    assert_eq!(listing.total(), Total::AtLeast(100));
}

#[test]
fn rows_total_gives_only_an_exact_count() {
    assert_eq!(Rows::total(&Listing::new(books(2), 6)), Some(6));
    assert_eq!(Rows::total(&Listing::uncounted(books(2), true)), None);
}

#[test]
fn an_uncounted_listing_leaves_rows_out_only_when_its_source_holds_more() {
    assert!(Listing::uncounted(books(2), true).more());
    assert!(!Listing::uncounted(books(2), false).more());
    assert_eq!(Listing::uncounted(books(2), true).at_least(), None);
}

#[test]
fn an_exact_listing_leaves_rows_out_only_when_it_shows_fewer_than_its_total() {
    assert!(Listing::new(books(2), 3).more());
    assert!(!Listing::new(books(3), 3).more());
}

#[test]
fn exact_returns_the_count_of_an_exact_total_only() {
    assert_eq!(Total::Exact(6).exact(), Some(6));
    assert_eq!(Total::AtLeast(6).exact(), None);
    assert_eq!(Total::Unknown { more: true }.exact(), None);
    assert_eq!(Total::Unknown { more: false }.exact(), None);
}
