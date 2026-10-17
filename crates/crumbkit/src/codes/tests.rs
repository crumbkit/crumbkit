use super::*;
use crate::Spelling;

#[test]
fn every_crumbkit_code_is_in_all_once_and_spells_with_its_own_component_area_and_reason() {
    let spelled: Vec<String> = ALL
        .iter()
        .map(|code| Spelling::COLONS.spell(code))
        .collect();
    let expected = vec![
        "crumbkit::cli::usage",
        "crumbkit::commands::not_found",
        "crumbkit::schemas::not_found",
        "crumbkit::confirmation::declined",
        "crumbkit::confirmation::needs_yes",
        "crumbkit::version::conflict",
        "crumbkit::output::over_budget",
        "crumbkit::report::unchecked",
    ];
    assert_eq!(spelled, expected);
}
