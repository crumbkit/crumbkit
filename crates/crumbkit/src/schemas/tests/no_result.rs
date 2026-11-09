//! Tests for a leaf whose command prints no result, which publishes no
//! schema of its own.

use super::tree;
use crate::Tool;
use crate::help::{Help, Leaf};
use crate::schemas::{Served, documents};

#[test]
fn a_leaf_that_prints_no_result_publishes_only_the_failure_document() {
    const TOOL: Tool =
        Tool::new().help(Help::new().leaves(&[Leaf::new(&["books", "find"]).prints_no_result()]));

    let served = documents(&tree(), &TOOL);

    let ids: Vec<String> = served.iter().map(Served::id).collect();
    assert_eq!(ids, ["shelf/failure/v1"]);
}
