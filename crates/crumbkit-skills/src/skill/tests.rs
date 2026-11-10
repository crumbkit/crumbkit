use super::{Skill, Writer};

const TEXT: &str =
    "---\nname: shelf\ndescription: Browse the catalogue.\n---\n\nRun `shelf books list`.\n";

const SKILL: Skill = Skill::new("shelf", "0.3.0+git-951022a3a14a.dirty", TEXT);

#[test]
fn the_installed_text_adds_quoted_metadata_at_the_end_of_the_frontmatter() {
    assert_eq!(
        SKILL.installed(),
        "---\nname: shelf\ndescription: Browse the catalogue.\nmetadata:\n  tool: \"shelf\"\n  version: \"0.3.0+git-951022a3a14a.dirty\"\n---\n\nRun `shelf books list`.\n"
    );
}

#[test]
fn the_installed_text_is_written_by_the_tool_at_its_version() {
    assert_eq!(
        SKILL.writer(&SKILL.installed()),
        Writer::Tool {
            version: "0.3.0+git-951022a3a14a.dirty".to_owned()
        }
    );
}

#[test]
fn a_copy_of_another_version_is_still_written_by_the_tool() {
    let older = Skill::new("shelf", "0.2.0", TEXT);

    assert_eq!(
        SKILL.writer(&older.installed()),
        Writer::Tool {
            version: "0.2.0".to_owned()
        }
    );
}

#[test]
fn the_skill_text_without_metadata_is_written_by_someone_else() {
    assert_eq!(SKILL.writer(TEXT), Writer::Other);
}

#[test]
fn a_copy_that_names_another_tool_is_written_by_someone_else() {
    let other = Skill::new("reading", "0.3.0", TEXT);

    assert_eq!(SKILL.writer(&other.installed()), Writer::Other);
}

#[test]
fn a_tool_line_outside_the_metadata_block_does_not_count() {
    let text = "---\nname: shelf\nother:\n  tool: \"shelf\"\n  version: \"0.3.0\"\n---\n";

    assert_eq!(SKILL.writer(text), Writer::Other);
}

#[test]
fn a_tool_line_after_the_frontmatter_does_not_count() {
    let text = "---\nname: shelf\n---\nmetadata:\n  tool: \"shelf\"\n  version: \"0.3.0\"\n";

    assert_eq!(SKILL.writer(text), Writer::Other);
}

#[test]
fn a_text_whose_frontmatter_ends_the_file_gets_its_metadata_before_the_closing_line() {
    let skill = Skill::new("shelf", "1.0.0", "---\nname: shelf\n---");

    assert_eq!(
        skill.installed(),
        "---\nname: shelf\nmetadata:\n  tool: \"shelf\"\n  version: \"1.0.0\"\n---"
    );
}

#[test]
fn the_getters_return_what_the_tool_passed() {
    assert_eq!(SKILL.name(), "shelf");
    assert_eq!(SKILL.version(), "0.3.0+git-951022a3a14a.dirty");
    assert_eq!(SKILL.text(), TEXT);
}

#[test]
#[should_panic(expected = "a skill name is lowercase ASCII letters")]
fn a_name_with_a_slash_is_refused() {
    let _ = Skill::new(std::hint::black_box("../shelf"), "0.3.0", TEXT);
}

#[test]
#[should_panic(expected = "a skill version is one non-empty line")]
fn a_version_with_a_quote_is_refused() {
    let _ = Skill::new("shelf", std::hint::black_box("0.3\"0"), TEXT);
}

#[test]
#[should_panic(expected = "starts with a frontmatter block")]
fn a_text_without_a_closing_line_is_refused() {
    let _ = Skill::new("shelf", "0.3.0", std::hint::black_box("---\nname: shelf\n"));
}

#[test]
#[should_panic(expected = "has no `metadata:` key")]
fn a_text_with_its_own_metadata_is_refused() {
    let _ = Skill::new(
        "shelf",
        "0.3.0",
        std::hint::black_box("---\nname: shelf\nmetadata:\n  a: \"b\"\n---\n"),
    );
}
