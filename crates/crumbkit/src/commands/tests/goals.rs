//! Tests for the goal headings that the row of a guided group lists.

use serde_json::json;

use super::{args, listed, output, row};
use crate::Tool;
use crate::commands::list;
use crate::help::{Goal, Group, Help};

#[test]
fn a_group_s_row_lists_its_goals_in_goal_order_and_leaves_out_hidden_subcommands_as_help_does() {
    let rows = listed(&["books"]);

    assert_eq!(
        row(&rows, "shelf books")["goals"],
        json!([{ "heading": "Read", "commands": ["show", "list"] }])
    );
}

#[test]
fn a_group_whose_only_goal_names_a_hidden_subcommand_has_no_goals_key_as_its_help_prints_no_heading()
 {
    const TOOL: Tool = Tool::new()
        .help(Help::new().groups(&[Group::new(&["books"], &[Goal::new("Clean", &["purge"])])]));

    let listing = list(&mut output(), &TOOL, &args(&["books"])).unwrap();
    let rows = serde_json::to_value(listing).unwrap();

    assert!(row(&rows, "shelf books").get("goals").is_none());
}
