//! Tests for the row that `commands list` prints for one command.

use clap::Command;
use serde_json::{Value, json};

use super::CommandRow;
use crate::help::{Effects, Guide, Leaf};

/// Returns the JSON of the row of `shelf books remove` with `guide`.
fn row(guide: &Guide) -> Value {
    let command = Command::new("remove").about("Remove books");
    let row = CommandRow::new("shelf", &["books", "remove"], &command, None, guide);
    serde_json::to_value(row).unwrap()
}

#[test]
fn a_row_holds_the_effects_that_the_leaf_declares() {
    const GUIDE: Guide = Guide::new()
        .leaves(&[Leaf::new(&["books", "remove"]).effects(Effects::destructive().idempotent())]);

    assert_eq!(
        row(&GUIDE)["effects"],
        json!({"read_only": false, "destructive": true, "idempotent": true, "open_world": false})
    );
}

#[test]
fn a_row_holds_null_effects_when_no_leaf_declares_them() {
    assert_eq!(row(&Guide::new())["effects"], Value::Null);
}
