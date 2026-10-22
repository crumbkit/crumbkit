//! Tests for a leaf whose command prints no result, which publishes no
//! schema of its own.

use super::tool;
use crate::help::{Guide, Leaf};
use crate::schemas::{Served, documents};

#[test]
fn a_leaf_that_prints_no_result_publishes_only_the_failure_document() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "find"]).prints_no_result()]);

    let served = documents(&tool(), &GUIDE);

    let ids: Vec<String> = served.iter().map(Served::id).collect();
    assert_eq!(ids, ["shelf/failure/v1"]);
}
