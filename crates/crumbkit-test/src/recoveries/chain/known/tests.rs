//! Tests for the text that a spelling writes before a component's areas.

use crumbkit::{Spelling, ToolName};

use super::spelled_prefix;

#[test]
fn the_colon_spelling_gives_the_component_and_its_colons() {
    assert_eq!(
        spelled_prefix(Spelling::COLONS, "shelf").as_deref(),
        Some("shelf::")
    );
}

#[test]
fn a_spelling_that_joins_the_segments_with_dots_gives_the_component_and_a_dot() {
    const DOTS: Spelling = Spelling::custom(|code, f| {
        write!(f, "{}.{}.{}", code.component(), code.area(), code.reason())
    });

    assert_eq!(spelled_prefix(DOTS, "shelf").as_deref(), Some("shelf."));
}

#[test]
fn a_spelling_for_a_tool_gives_the_tool_s_name_for_crumbkit_s_component() {
    let spelling = Spelling::COLONS.for_tool(ToolName::new("shelf"));

    assert_eq!(
        spelled_prefix(spelling, "crumbkit").as_deref(),
        Some("shelf::")
    );
}

#[test]
fn a_spelling_that_writes_the_reason_first_gives_no_prefix() {
    const REASON_FIRST: Spelling = Spelling::custom(|code, f| {
        write!(f, "{}.{}.{}", code.reason(), code.component(), code.area())
    });

    assert_eq!(spelled_prefix(REASON_FIRST, "shelf"), None);
}

#[test]
fn a_spelling_that_writes_the_area_first_gives_no_prefix() {
    const AREA_FIRST: Spelling = Spelling::custom(|code, f| {
        write!(f, "{}.{}.{}", code.area(), code.component(), code.reason())
    });

    assert_eq!(spelled_prefix(AREA_FIRST, "shelf"), None);
}

#[test]
fn a_spelling_that_joins_with_underscores_or_nothing_gives_no_prefix() {
    const UNDERSCORES: Spelling = Spelling::custom(|code, f| {
        write!(f, "{}_{}_{}", code.component(), code.area(), code.reason())
    });
    const JOINED: Spelling = Spelling::custom(|code, f| {
        write!(f, "{}{}{}", code.component(), code.area(), code.reason())
    });

    assert_eq!(spelled_prefix(UNDERSCORES, "shelf"), None);
    assert_eq!(spelled_prefix(JOINED, "shelf"), None);
}

#[test]
fn a_spelling_that_leaves_out_the_component_gives_no_prefix() {
    const NO_COMPONENT: Spelling =
        Spelling::custom(|code, f| write!(f, "E:{}:{}", code.area(), code.reason()));

    assert_eq!(spelled_prefix(NO_COMPONENT, "shelf"), None);
}

#[test]
fn a_spelling_that_changes_the_area_s_letters_gives_no_prefix() {
    const UPPER_AREA: Spelling = Spelling::custom(|code, f| {
        let area = code.area().to_uppercase();
        write!(f, "{}::{area}::{}", code.component(), code.reason())
    });

    assert_eq!(spelled_prefix(UPPER_AREA, "shelf"), None);
}

#[test]
fn a_component_s_prefix_does_not_match_a_code_of_a_longer_component_that_starts_with_its_name() {
    const UNDERSCORES: Spelling = Spelling::custom(|code, f| {
        write!(f, "{}_{}_{}", code.component(), code.area(), code.reason())
    });
    let shelf = spelled_prefix(Spelling::COLONS, "shelf").unwrap();
    let cache = spelled_prefix(Spelling::COLONS, "shelf_cache").unwrap();

    assert_eq!(cache, "shelf_cache::");
    assert!(!"shelf_cache::sync::stale".starts_with(&shelf));
    assert_eq!(spelled_prefix(UNDERSCORES, "shelf"), None);
    assert_eq!(spelled_prefix(UNDERSCORES, "shelf_cache"), None);
}
