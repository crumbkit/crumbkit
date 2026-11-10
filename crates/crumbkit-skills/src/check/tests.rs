use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use super::{CheckRow, Checked, SKILLS_CHECK, SkillsCheck, check_in};
use crate::skill::Skill;
use crate::targets::Homes;
use crate::testing::write;

const TEXT: &str =
    "---\nname: shelf\ndescription: Browse the catalogue.\n---\n\nRun `shelf books list`.\n";

const SKILL: Skill = Skill::new("shelf", "0.3.0", TEXT);

/// A temporary home with `.claude/skills` and `.agents/skills`.
struct Home {
    dir: TempDir,
    homes: Homes,
}

impl Home {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        for path in [".claude", ".claude/skills", ".agents", ".agents/skills"] {
            fs::create_dir(dir.path().join(path)).unwrap();
        }
        let homes = Homes::new(Some(OsString::from(dir.path())), None);
        Self { dir, homes }
    }

    fn skills(&self, agent: &str) -> PathBuf {
        self.dir.path().join(agent).join("skills")
    }

    /// Writes `text` as the copy in the skills directory of `agent`.
    fn put(&self, agent: &str, text: &str) {
        let folder = self.skills(agent).join("shelf");
        fs::create_dir(&folder).unwrap();
        write(&folder.join("SKILL.md"), text);
    }

    fn check(&self) -> Vec<CheckRow> {
        check_in(&SKILL, &self.homes, &SkillsCheck::default())
            .rows()
            .to_vec()
    }
}

fn states(rows: &[CheckRow]) -> Vec<Checked> {
    rows.iter().map(|row| row.state).collect()
}

#[test]
fn check_lists_a_missing_copy_before_a_current_one() {
    let home = Home::new();
    home.put(".agents", &SKILL.installed());

    let rows = home.check();

    assert_eq!(states(&rows), [Checked::Missing, Checked::Current]);
    assert_eq!(rows[0].dirs, [home.skills(".claude").display().to_string()]);
}

#[test]
fn a_copy_from_another_version_differs_and_names_that_version() {
    let home = Home::new();
    home.put(".claude", &SKILL.installed());
    home.put(".agents", &Skill::new("shelf", "0.2.0", TEXT).installed());

    let rows = home.check();

    assert_eq!(states(&rows), [Checked::Differs, Checked::Current]);
    assert_eq!(rows[0].reason.as_deref(), Some("written by shelf 0.2.0"));
}

#[test]
fn a_copy_that_install_did_not_write_differs_and_names_force() {
    let home = Home::new();
    home.put(".claude", &SKILL.installed());
    home.put(".agents", TEXT);

    let rows = home.check();

    assert_eq!(states(&rows), [Checked::Differs, Checked::Current]);
    assert_eq!(
        rows[0].reason.as_deref(),
        Some("not written by shelf skills install; install --force replaces it")
    );
}

#[test]
fn a_file_where_the_folder_belongs_is_unreadable() {
    let home = Home::new();
    home.put(".claude", &SKILL.installed());
    write(&home.skills(".agents").join("shelf"), "a file");

    let rows = home.check();

    assert_eq!(states(&rows), [Checked::Unreadable, Checked::Current]);
}

#[test]
fn every_current_copy_reports_no_problem() {
    let home = Home::new();
    home.put(".claude", &SKILL.installed());
    home.put(".agents", &SKILL.installed());

    let listing = check_in(&SKILL, &home.homes, &SkillsCheck::default());

    assert_eq!(states(listing.rows()), [Checked::Current, Checked::Current]);
    assert!(
        listing
            .rows()
            .iter()
            .all(|row| !crumbkit::Row::is_problem(row))
    );
}

#[test]
fn no_home_checks_nothing_and_says_so() {
    let listing = check_in(&SKILL, &Homes::default(), &SkillsCheck::default());

    assert_eq!(listing.rows(), []);
    assert_eq!(
        listing.get_note(),
        Some("The environment names no home directory, so shelf checked nothing.")
    );
}

#[test]
fn a_named_dir_that_does_not_exist_is_missing() {
    let home = Home::new();

    let rows = check_in(
        &SKILL,
        &home.homes,
        &SkillsCheck {
            dir: Some(home.dir.path().join("missing")),
        },
    )
    .rows()
    .to_vec();

    assert_eq!(states(&rows), [Checked::Missing]);
    assert_eq!(
        rows[0].reason.as_deref(),
        Some("the directory does not exist")
    );
}

#[cfg(unix)]
#[test]
fn a_claude_dir_that_links_to_the_agents_dir_is_one_row() {
    let home = Home::new();
    fs::remove_dir(home.skills(".claude")).unwrap();
    crate::testing::link(&home.skills(".agents"), &home.skills(".claude"));
    home.put(".agents", &SKILL.installed());

    let rows = home.check();

    assert_eq!(states(&rows), [Checked::Current]);
    assert_eq!(rows[0].dirs.len(), 2);
}

#[cfg(unix)]
#[test]
fn a_tool_folder_that_links_elsewhere_differs_and_names_the_link() {
    let home = Home::new();
    home.put(".claude", &SKILL.installed());
    let source = home.dir.path().join("source");
    fs::create_dir(&source).unwrap();
    write(&source.join("SKILL.md"), TEXT);
    crate::testing::link(&source, &home.skills(".agents").join("shelf"));

    let rows = home.check();

    assert_eq!(states(&rows), [Checked::Differs, Checked::Current]);
    let real = fs::canonicalize(&source).unwrap();
    assert_eq!(
        rows[0].reason.as_deref(),
        Some(
            format!(
                "linked to {}, which shelf skills install did not write",
                real.display()
            )
            .as_str()
        )
    );
}

#[test]
fn the_checked_in_schema_matches_the_check_document_schema() {
    let generated = crumbkit::schemas::document_schema(SKILLS_CHECK);
    let generated = serde_json::to_string_pretty(&generated).unwrap() + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/skills-check-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/skills-check-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}
