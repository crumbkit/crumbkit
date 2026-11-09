//! Tests for the text that a spelling writes before a component's areas.

use crumbkit::{Spelling, ToolName};

use super::spelled_prefix;

#[test]
fn the_colon_spelling_gives_the_component_and_its_colons() {
    assert_eq!(spelled_prefix(Spelling::COLONS, "shelf"), "shelf::");
}

#[test]
fn a_spelling_for_a_tool_gives_the_tool_s_name_for_crumbkit_s_component() {
    let spelling = Spelling::COLONS.for_tool(ToolName::new("shelf"));

    assert_eq!(spelled_prefix(spelling, "crumbkit"), "shelf::");
}

#[test]
fn a_component_s_prefix_does_not_match_a_code_of_a_longer_component_that_starts_with_its_name() {
    let shelf = spelled_prefix(Spelling::COLONS, "shelf");
    let cache = spelled_prefix(Spelling::COLONS, "shelf_cache");

    assert_eq!(cache, "shelf_cache::");
    assert!(!"shelf_cache::sync::stale".starts_with(&shelf));
}
