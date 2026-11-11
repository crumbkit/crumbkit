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
    assert_eq!(SKILL.tool(), "shelf");
    assert_eq!(SKILL.name(), "shelf");
    assert_eq!(SKILL.version(), "0.3.0+git-951022a3a14a.dirty");
    assert_eq!(SKILL.text(), TEXT);
}

#[test]
#[should_panic(expected = "a tool name is lowercase ASCII letters")]
fn a_tool_name_with_a_slash_is_refused() {
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

#[test]
fn a_key_that_only_starts_with_metadata_is_accepted() {
    let skill = Skill::new("shelf", "0.3.0", "---\nname: shelf\nmetadatax: one\n---\n");

    assert!(skill.installed().contains("metadatax: one\nmetadata:\n"));
}

#[test]
#[should_panic(expected = "has no `metadata:` key")]
fn an_inline_metadata_map_is_refused() {
    let _ = Skill::new(
        "shelf",
        "0.3.0",
        std::hint::black_box("---\nname: shelf\nmetadata: {a: b}\n---\n"),
    );
}

const FILES: &[(&str, &str)] = &[("REFERENCE.md", "# Reference\n"), ("report.json", "{}\n")];

#[test]
fn a_skill_keeps_its_files_in_the_order_the_tool_gives_them() {
    let skill = Skill::new("shelf", "0.3.0", TEXT).files(FILES);

    assert_eq!(skill.get_files(), FILES);
}

#[test]
fn a_skill_without_files_has_none() {
    assert_eq!(SKILL.get_files(), &[]);
}

#[test]
#[should_panic(
    expected = "a skill file name is ASCII letters, digits, dots, dashes and underscores"
)]
fn a_file_in_a_subfolder_is_refused() {
    let _ = Skill::new("shelf", "0.3.0", TEXT).files(std::hint::black_box(&[("docs/a.md", "")]));
}

#[test]
#[should_panic(
    expected = "a skill file name is ASCII letters, digits, dots, dashes and underscores"
)]
fn a_file_name_that_starts_with_a_dot_is_refused() {
    let _ = Skill::new("shelf", "0.3.0", TEXT).files(std::hint::black_box(&[("..", "")]));
}

#[test]
#[should_panic(expected = "a skill file is not named SKILL.md")]
fn a_file_named_skill_md_is_refused() {
    let _ = Skill::new("shelf", "0.3.0", TEXT).files(std::hint::black_box(&[("SKILL.md", "")]));
}

#[test]
#[should_panic(expected = "a skill names each file once")]
fn a_file_named_twice_is_refused() {
    let _ = Skill::new("shelf", "0.3.0", TEXT)
        .files(std::hint::black_box(&[("a.md", "one"), ("a.md", "two")]));
}

#[test]
#[should_panic(expected = "a skill file is not named SKILL.md")]
fn a_file_named_skill_md_in_other_case_is_refused() {
    let _ = Skill::new("shelf", "0.3.0", TEXT).files(std::hint::black_box(&[("skill.md", "")]));
}

#[test]
#[should_panic(expected = "a skill names each file once")]
fn two_names_that_differ_only_by_case_are_refused() {
    let _ = Skill::new("shelf", "0.3.0", TEXT)
        .files(std::hint::black_box(&[("a.md", "one"), ("A.md", "two")]));
}

const LENDING_TEXT: &str =
    "---\nname: shelf-lending\ndescription: Lend a book.\n---\n\nRun `shelf loans add`.\n";

#[test]
fn a_skill_takes_its_name_from_the_name_line_of_its_frontmatter_and_keeps_the_tool_s_name_apart() {
    let skill = Skill::new("shelf", "0.3.0", LENDING_TEXT);

    assert_eq!(skill.name(), "shelf-lending");
    assert_eq!(skill.tool(), "shelf");
}

#[test]
fn a_quoted_name_with_spaces_around_it_is_read_without_the_quotes_and_spaces() {
    let skill = Skill::new(
        "shelf",
        "0.3.0",
        "---\ndescription: Lend a book.\nname:   \"shelf-lending\"  \n---\n",
    );

    assert_eq!(skill.name(), "shelf-lending");
}

#[test]
fn the_installed_text_of_a_second_skill_records_the_tool_s_name() {
    let skill = Skill::new("shelf", "0.3.0", LENDING_TEXT);

    assert_eq!(
        skill.installed(),
        "---\nname: shelf-lending\ndescription: Lend a book.\nmetadata:\n  tool: \"shelf\"\n  version: \"0.3.0\"\n---\n\nRun `shelf loans add`.\n"
    );
}

#[test]
fn a_copy_of_another_skill_of_the_same_tool_is_written_by_the_tool() {
    let lending = Skill::new("shelf", "0.2.0", LENDING_TEXT);

    assert_eq!(
        SKILL.writer(&lending.installed()),
        Writer::Tool {
            version: "0.2.0".to_owned()
        }
    );
}

#[test]
#[should_panic(expected = "a skill's frontmatter has a `name:` line")]
fn a_text_without_a_name_line_is_refused() {
    let _ = Skill::new(
        "shelf",
        "0.3.0",
        std::hint::black_box("---\ndescription: Lend a book.\n---\n"),
    );
}

#[test]
#[should_panic(expected = "a skill's frontmatter has a `name:` line")]
fn a_name_line_after_the_frontmatter_does_not_count() {
    let _ = Skill::new(
        "shelf",
        "0.3.0",
        std::hint::black_box("---\ndescription: Lend a book.\n---\nname: shelf\n"),
    );
}

#[test]
#[should_panic(expected = "a skill's frontmatter has a `name:` line whose value is lowercase")]
fn a_name_that_cannot_name_a_folder_is_refused() {
    let _ = Skill::new(
        "shelf",
        "0.3.0",
        std::hint::black_box("---\nname: ../shelf\n---\n"),
    );
}
