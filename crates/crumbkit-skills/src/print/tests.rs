use super::{SKILLS_PRINT, print};
use crate::skill::Skill;

const TEXT: &str =
    "---\nname: shelf\ndescription: Browse the catalogue.\n---\n\nRun `shelf books list`.\n";

const SKILL: Skill = Skill::new("shelf", "0.3.0+git-951022a3a14a", TEXT);

#[test]
fn print_shows_the_skill_text_without_the_metadata_lines() {
    let prose = print(&SKILL);

    assert_eq!(prose.value().to_string(), TEXT);
}

#[test]
fn the_json_of_print_holds_the_tool_and_its_version_beside_the_text() {
    let json = serde_json::to_value(print(&SKILL)).unwrap();

    assert_eq!(
        json,
        serde_json::json!({
            "tool": "shelf",
            "version": "0.3.0+git-951022a3a14a",
            "text": TEXT,
        })
    );
}

#[test]
fn the_checked_in_schema_matches_the_print_document_schema() {
    let generated = crumbkit::schemas::document_schema(SKILLS_PRINT);
    let generated = serde_json::to_string_pretty(&generated).unwrap() + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/skills-print-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/skills-print-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}
