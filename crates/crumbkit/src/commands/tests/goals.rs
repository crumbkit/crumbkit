//! Tests for the goal headings that the row of a guided group lists.

use serde_json::json;

use super::{listed, row};

#[test]
fn a_group_s_row_lists_its_goals_in_goal_order_and_leaves_out_hidden_subcommands_as_help_does() {
    let rows = listed(&["books"]);

    assert_eq!(
        row(&rows, "shelf books")["goals"],
        json!([{ "heading": "Read", "commands": ["show", "list"] }])
    );
}
