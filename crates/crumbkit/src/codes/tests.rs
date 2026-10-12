use super::*;
use crate::Spelling;

#[test]
fn every_crumbkit_code_is_in_all_once_and_spells_with_its_own_component_area_and_reason() {
    let spelled: Vec<String> = ALL
        .iter()
        .map(|code| Spelling::COLONS.spell(code))
        .collect();
    let mut expected = vec![
        "crumbkit::cli::usage",
        "crumbkit::commands::not_found",
        "crumbkit::commands::unavailable",
        "crumbkit::schemas::not_found",
        "crumbkit::schemas::unavailable",
        "crumbkit::confirm::declined",
        "crumbkit::confirm::needs_yes",
    ];
    if cfg!(feature = "hand-over") {
        expected.extend([
            "crumbkit::child::no_json",
            "crumbkit::child::not_found",
            "crumbkit::child::not_started",
            "crumbkit::child::bad_json",
            "crumbkit::child::failed",
        ]);
    }
    assert_eq!(spelled, expected);
}
