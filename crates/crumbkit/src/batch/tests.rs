//! Tests for batches. crumbkit's tests check how `finish_to` prints them.

use std::fmt;

use schemars::JsonSchema;
use serde::Serialize;

use super::Batch;
use crate::{Call, Code, Failure, Listing, Recovery, Row, Rows, Shape};

const NOT_FOUND: Code = Code::new("solar", "planets", "not_found");

#[derive(Serialize, JsonSchema)]
struct Planet {
    name: &'static str,
}

impl Row for Planet {
    const COLUMNS: &'static [&'static str] = &["name"];
}

fn saturn() -> Planet {
    Planet { name: "Saturn" }
}

/// A failure that names a planet the solar system lacks.
#[derive(Debug)]
struct Missing(&'static str);

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the solar system has no planet {}", self.0)
    }
}

impl std::error::Error for Missing {}

impl Failure for Missing {
    fn code(&self) -> &'static Code {
        &NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&["list"]))
    }
}

#[test]
fn only_a_batch_returns_failures_from_rows() {
    let vec = vec![saturn()];
    let listing = Listing::new(vec![saturn()], 1);
    let mut batch = Batch::new(&["show"], "names", &["Saturn", "Vulcan"]);
    batch.extend([Ok(saturn()), Err(Missing("Vulcan"))]);
    let mut empty: Batch<Planet, Missing> = Batch::new(&["show"], "names", &["Saturn"]);
    empty.push(Ok(saturn()));

    assert!(saturn().failures().is_none());
    assert!(vec.failures().is_none());
    assert!(listing.failures().is_none());
    let failures = batch.failures().unwrap();
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].to_string(),
        "the solar system has no planet Vulcan"
    );
    assert_eq!(empty.failures().map(|list| list.len()), Some(0));
}

#[test]
fn a_batch_names_its_shape() {
    assert_eq!(<Batch<Planet, Missing> as Rows>::SHAPE, Shape::Batch);
}

fn planet(name: &'static str) -> Planet {
    Planet { name }
}

#[test]
fn a_batch_holds_its_rows_and_failures_in_the_order_of_their_keys() {
    let mut batch = Batch::new(&["show"], "names", &["Mars", "Vulcan", "Venus"]);
    batch.push(Ok(planet("Mars")));
    batch.push(Err(Missing("Vulcan")));
    batch.push(Ok(planet("Venus")));

    let names: Vec<_> = batch.rows().iter().map(|row| row.name).collect();
    assert_eq!(names, ["Mars", "Venus"]);
    assert_eq!(batch.failures().map(|list| list.len()), Some(1));
    assert_eq!(batch.left_out(2), Vec::<Option<&str>>::new());
}

#[test]
fn the_keys_of_the_rows_after_the_shown_ones_and_the_keys_never_fetched_are_left_out() {
    let mut batch: Batch<Planet, Missing> =
        Batch::new(&["show"], "names", &["Mars", "Venus", "Earth", "Saturn"]);
    batch.push(Ok(planet("Mars")));
    batch.push(Ok(planet("Venus")));

    assert_eq!(
        batch.left_out(1),
        [Some("Venus"), Some("Earth"), Some("Saturn")]
    );
    assert_eq!(batch.left_out(2), [Some("Earth"), Some("Saturn")]);
}

#[test]
fn a_failed_key_is_never_left_out() {
    let mut batch = Batch::new(&["show"], "ids", &[1, 2, 3]);
    batch.push(Ok(planet("Mars")));
    batch.push(Err(Missing("Vulcan")));
    batch.push(Ok(planet("Venus")));

    assert_eq!(batch.left_out(0), [Some("1"), Some("3")]);
}

#[test]
fn a_result_pushed_after_the_last_key_has_no_key() {
    let mut batch: Batch<Planet, Missing> = Batch::new(&["show"], "names", &["Mars"]);
    batch.extend([Ok(planet("Mars")), Ok(planet("Venus"))]);

    assert_eq!(batch.left_out(0), [Some("Mars"), None]);
}

#[test]
fn a_batch_fetches_its_left_out_keys_with_its_command_in_one_call() {
    let batch: Batch<Planet, Missing> = Batch::new(&["planets", "show"], "names", &["Mars"]);

    assert_eq!(
        batch.fetch(),
        Some(Call::new(&["planets", "show"]).all("names", "left_out"))
    );
    assert_eq!(saturn().fetch(), None);
}

#[test]
fn a_key_given_twice_is_left_out_twice_in_the_order_of_the_keys() {
    let mut batch: Batch<Planet, Missing> =
        Batch::new(&["show"], "names", &["Mars", "Venus", "Mars", "Mars"]);
    batch.push(Ok(planet("Mars")));
    batch.push(Ok(planet("Venus")));

    assert_eq!(
        batch.left_out(0),
        [Some("Mars"), Some("Venus"), Some("Mars"), Some("Mars")]
    );
}

#[test]
fn a_batch_keeps_the_index_and_key_of_each_failure() {
    let mut batch = Batch::new(&["show"], "names", &["Vulcan", "Mars", "Vulcan"]);
    batch.push(Err(Missing("Vulcan")));
    batch.push(Ok(planet("Mars")));
    batch.push(Err(Missing("Vulcan")));

    assert_eq!(batch.failure_indexes(), [0, 2]);
    assert_eq!(batch.row_indexes(), [1]);
    assert_eq!(
        (batch.key(0), batch.key(2)),
        (Some("Vulcan"), Some("Vulcan"))
    );
}

#[test]
fn a_failure_pushed_after_the_last_key_keeps_its_index_and_has_no_key() {
    let mut batch: Batch<Planet, Missing> = Batch::new(&["show"], "names", &["Mars"]);
    batch.extend([Ok(planet("Mars")), Err(Missing("Vulcan"))]);

    assert_eq!(batch.failure_indexes(), [1]);
    assert_eq!(batch.key(1), None);
}

#[test]
fn the_answers_left_out_start_at_the_index_of_the_first_row_that_does_not_print() {
    let mut batch = Batch::new(&["show"], "names", &["Mars", "Vulcan", "Venus", "Earth"]);
    batch.push(Ok(planet("Mars")));
    batch.push(Err(Missing("Vulcan")));
    batch.push(Ok(planet("Venus")));
    batch.push(Ok(planet("Earth")));

    assert_eq!(batch.left_out_from(1), Some(2));
    assert_eq!(batch.left_out_from(3), None);
}

#[test]
fn the_answers_left_out_start_at_the_first_unfetched_key_when_every_row_prints() {
    let mut batch: Batch<Planet, Missing> =
        Batch::new(&["show"], "names", &["Mars", "Vulcan", "Venus"]);
    batch.push(Ok(planet("Mars")));
    batch.push(Err(Missing("Vulcan")));

    assert_eq!(batch.left_out_from(1), Some(2));
}
