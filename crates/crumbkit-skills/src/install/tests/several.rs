//! Tests of a tool that installs several skills.

use std::fs;
use std::path::Path;

use super::{Home, InstallRow, Installed, SKILL, SkillsInstall, installed, states};
use crate::skill::Skill;
use crate::testing::{LENDING, LENDING_TEXT, write};

fn skills(rows: &[InstallRow]) -> Vec<&str> {
    rows.iter().map(|row| row.skill.as_str()).collect()
}

fn lending(dir: &Path) -> String {
    fs::read_to_string(dir.join("shelf-lending").join("SKILL.md")).unwrap()
}

#[test]
fn two_skills_of_one_tool_install_into_two_folders_and_each_row_names_its_skill() {
    let home = Home::new();
    let agents = home.mkdir(".agents/skills");

    let rows = home.install_all(&[SKILL, LENDING], &SkillsInstall::default());

    assert_eq!(states(&rows), [Installed::Written, Installed::Written]);
    assert_eq!(skills(&rows), ["shelf", "shelf-lending"]);
    assert!(Path::new(&rows[1].path).ends_with("shelf-lending/SKILL.md"));
    assert_eq!(installed(&agents), SKILL.installed());
    assert_eq!(lending(&agents), LENDING.installed());
    assert!(lending(&agents).contains("metadata:\n  tool: \"shelf\"\n"));
}

#[test]
fn a_second_install_of_two_skills_leaves_both_unchanged() {
    let home = Home::new();
    home.mkdir(".agents/skills");
    home.install_all(&[SKILL, LENDING], &SkillsInstall::default());

    let rows = home.install_all(&[SKILL, LENDING], &SkillsInstall::default());

    assert_eq!(states(&rows), [Installed::Unchanged, Installed::Unchanged]);
    assert_eq!(skills(&rows), ["shelf", "shelf-lending"]);
}

#[test]
fn a_copy_of_the_second_skill_that_another_tool_wrote_is_skipped_and_the_first_is_still_written() {
    let home = Home::new();
    let agents = home.mkdir(".agents/skills");
    fs::create_dir(agents.join("shelf-lending")).unwrap();
    let other = Skill::new("reading", "0.3.0", LENDING_TEXT).installed();
    write(&agents.join("shelf-lending/SKILL.md"), &other);

    let rows = home.install_all(&[SKILL, LENDING], &SkillsInstall::default());

    assert_eq!(states(&rows), [Installed::Written, Installed::Skipped]);
    assert_eq!(skills(&rows), ["shelf", "shelf-lending"]);
    assert_eq!(
        rows[1].reason.as_deref(),
        Some("not written by shelf skills install")
    );
    assert_eq!(installed(&agents), SKILL.installed());
    assert_eq!(lending(&agents), other);
}

#[test]
fn a_skill_that_fails_comes_first_and_the_other_skill_is_still_written() {
    let home = Home::new();
    let agents = home.mkdir(".agents/skills");
    fs::create_dir(agents.join("shelf-lending")).unwrap();
    let taken = agents.join(format!(
        "shelf-lending/.SKILL.md.{}.tmp",
        std::process::id()
    ));
    write(&taken, "someone else's file");

    let rows = home.install_all(&[SKILL, LENDING], &SkillsInstall::default());

    assert_eq!(states(&rows), [Installed::Failed, Installed::Written]);
    assert_eq!(skills(&rows), ["shelf-lending", "shelf"]);
    assert_eq!(installed(&agents), SKILL.installed());
}

#[test]
fn a_named_dir_that_does_not_exist_fails_once_for_each_skill() {
    let home = Home::new();

    let rows = home.install_all(
        &[SKILL, LENDING],
        &SkillsInstall {
            dir: Some(home.path("missing")),
            force: false,
        },
    );

    assert_eq!(states(&rows), [Installed::Failed, Installed::Failed]);
    assert_eq!(skills(&rows), ["shelf", "shelf-lending"]);
}
