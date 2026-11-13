//! Tests for the hand-over's codes.

use crumbkit::Spelling;

use super::ALL;

#[test]
fn all_lists_the_two_child_codes_in_the_crumbkit_component_in_order() {
    let spelled: Vec<String> = ALL
        .iter()
        .map(|&code| Spelling::COLONS.spell(code))
        .collect();
    assert_eq!(
        spelled,
        ["crumbkit::child::not_found", "crumbkit::child::not_started"]
    );
}
