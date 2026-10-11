//! Tests for batches. crumbkit's tests check how `finish_to` prints them.

use std::fmt;

use schemars::JsonSchema;
use serde::Serialize;

use super::Batch;
use crate::{Call, Code, Failure, Listing, Recovery, Row, Rows};

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
    let batch = Batch::new(vec![saturn()], vec![Missing("Vulcan")]);
    let empty: Batch<Planet, Missing> = Batch::new(vec![saturn()], Vec::new());

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
