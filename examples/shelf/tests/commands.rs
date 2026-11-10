//! End-to-end tests of the goal headings in shelf's `commands list` JSON.
//! Each test runs the built binary with a cleared environment and XDG
//! directories in a temporary directory of its own.

use std::error::Error;

use serde_json::{Value, json};
use snapbox::cmd::Command;

/// Runs `shelf commands list --all --json --full` and returns its rows.
///
/// # Errors
///
/// Returns the error of a temporary directory that cannot be created, or of
/// standard output that is not a listing.
fn rows() -> Result<Vec<Value>, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let home = dir.path();
    let output = Command::new(env!("CARGO_BIN_EXE_shelf"))
        .args(["commands", "list", "--all", "--json", "--full"])
        .current_dir(home)
        .env_clear()
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let listing: Value = serde_json::from_slice(&output)?;
    let rows = listing["data"]
        .as_array()
        .ok_or("the listing has no rows")?;
    Ok(rows.clone())
}

/// Returns the row of `rows` whose path is `path`.
fn row<'a>(rows: &'a [Value], path: &str) -> Option<&'a Value> {
    rows.iter().find(|row| row["path"] == path)
}

#[test]
fn the_root_row_lists_the_five_goals_of_shelf_s_help_in_help_order_with_their_commands() {
    let rows = rows().unwrap();

    assert_eq!(
        row(&rows, "shelf").unwrap()["goals"],
        json!([
            { "heading": "Browse the catalogue", "commands": ["books"] },
            { "heading": "Explore the commands", "commands": ["commands", "schemas"] },
            { "heading": "Review how shelf is used", "commands": ["self"] },
            { "heading": "Install shelf's skill", "commands": ["skills"] },
            { "heading": "Set up shell completion", "commands": ["completion"] },
        ])
    );
}
