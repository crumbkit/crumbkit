use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::{InstallRow, Installed, SKILLS_INSTALL, SkillsInstall, install_in};
use crate::skill::Skill;
use crate::targets::agent_dirs;
use crate::testing::write;

const TEXT: &str =
    "---\nname: shelf\ndescription: Browse the catalogue.\n---\n\nRun `shelf books list`.\n";

const SKILL: Skill = Skill::new("shelf", "0.3.0", TEXT);

/// A temporary home and the agent skills directories under it.
struct Home {
    dir: TempDir,
    agent_dirs: Option<Vec<PathBuf>>,
}

impl Home {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let agent_dirs = agent_dirs(Some(dir.path().to_owned()), None);
        Self { dir, agent_dirs }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    /// Creates the directory at `relative` and its parents.
    fn mkdir(&self, relative: &str) -> PathBuf {
        let path = self.path(relative);
        let mut built = PathBuf::new();
        for part in path.components() {
            built.push(part);
            if !built.exists() {
                fs::create_dir(&built).unwrap();
            }
        }
        path
    }

    fn install(&self, args: &SkillsInstall) -> Vec<InstallRow> {
        install_in(&SKILL, self.agent_dirs.clone(), args)
            .rows()
            .to_vec()
    }
}

fn installed(dir: &Path) -> String {
    fs::read_to_string(dir.join("shelf").join("SKILL.md")).unwrap()
}

fn forced() -> SkillsInstall {
    SkillsInstall {
        dir: None,
        force: true,
    }
}

fn states(rows: &[InstallRow]) -> Vec<Installed> {
    rows.iter().map(|row| row.state).collect()
}

#[test]
fn install_writes_the_skill_into_each_agent_dir_that_exists() {
    let home = Home::new();
    let claude = home.mkdir(".claude/skills");
    let codex = home.mkdir(".codex/skills");

    let rows = home.install(&SkillsInstall::default());

    assert_eq!(states(&rows), [Installed::Written, Installed::Written]);
    assert_eq!(rows[0].dirs, [claude.display().to_string()]);
    assert_eq!(rows[1].dirs, [codex.display().to_string()]);
    assert_eq!(installed(&claude), SKILL.installed());
    assert_eq!(installed(&codex), SKILL.installed());
    assert!(!home.path(".agents").exists());
}

#[test]
fn a_second_install_leaves_the_copy_unchanged() {
    let home = Home::new();
    home.mkdir(".agents/skills");
    home.install(&SkillsInstall::default());

    let rows = home.install(&SkillsInstall::default());

    assert_eq!(states(&rows), [Installed::Unchanged]);
}

#[test]
fn install_replaces_a_copy_that_another_version_wrote() {
    let home = Home::new();
    let agents = home.mkdir(".agents/skills");
    fs::create_dir(agents.join("shelf")).unwrap();
    let older = Skill::new("shelf", "0.2.0", TEXT);
    write(&agents.join("shelf/SKILL.md"), &older.installed());

    let rows = home.install(&SkillsInstall::default());

    assert_eq!(states(&rows), [Installed::Written]);
    assert_eq!(installed(&agents), SKILL.installed());
}

#[test]
fn install_skips_a_skill_file_that_it_did_not_write() {
    let home = Home::new();
    let agents = home.mkdir(".agents/skills");
    fs::create_dir(agents.join("shelf")).unwrap();
    write(&agents.join("shelf/SKILL.md"), TEXT);

    let rows = home.install(&SkillsInstall::default());

    assert_eq!(states(&rows), [Installed::Skipped]);
    assert_eq!(
        rows[0].reason.as_deref(),
        Some("not written by shelf skills install")
    );
    assert_eq!(installed(&agents), TEXT);
}

#[test]
fn force_replaces_a_skill_file_that_install_did_not_write() {
    let home = Home::new();
    let agents = home.mkdir(".agents/skills");
    fs::create_dir(agents.join("shelf")).unwrap();
    write(&agents.join("shelf/SKILL.md"), TEXT);

    let rows = home.install(&forced());

    assert_eq!(states(&rows), [Installed::Written]);
    assert_eq!(installed(&agents), SKILL.installed());
}

#[test]
fn install_skips_a_file_where_the_folder_belongs_and_force_replaces_it() {
    let home = Home::new();
    let agents = home.mkdir(".agents/skills");
    write(&agents.join("shelf"), "a file");

    let skipped = home.install(&SkillsInstall::default());
    let replaced = home.install(&forced());

    assert_eq!(states(&skipped), [Installed::Skipped]);
    assert!(
        skipped[0]
            .reason
            .as_deref()
            .unwrap()
            .ends_with("is not a folder")
    );
    assert_eq!(states(&replaced), [Installed::Written]);
    assert!(
        replaced[0]
            .reason
            .as_deref()
            .unwrap()
            .starts_with("replaced: ")
    );
    assert_eq!(installed(&agents), SKILL.installed());
}

#[test]
fn install_leaves_no_temporary_file_behind() {
    let home = Home::new();
    let agents = home.mkdir(".agents/skills");

    home.install(&SkillsInstall::default());

    let names: Vec<_> = fs::read_dir(agents.join("shelf"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, ["SKILL.md"]);
}

#[test]
fn a_named_dir_replaces_the_agent_dirs() {
    let home = Home::new();
    let agents = home.mkdir(".agents/skills");
    let named = home.mkdir("project/skills");

    let rows = home.install(&SkillsInstall {
        dir: Some(named.clone()),
        force: false,
    });

    assert_eq!(states(&rows), [Installed::Written]);
    assert_eq!(installed(&named), SKILL.installed());
    assert!(!agents.join("shelf").exists());
}

#[test]
fn a_named_dir_that_does_not_exist_fails() {
    let home = Home::new();

    let rows = home.install(&SkillsInstall {
        dir: Some(home.path("missing")),
        force: false,
    });

    assert_eq!(states(&rows), [Installed::Failed]);
    assert_eq!(
        rows[0].reason.as_deref(),
        Some("the directory does not exist")
    );
}

#[test]
fn no_home_installs_nothing_and_has_no_note_of_its_own() {
    let listing = install_in(&SKILL, None, &SkillsInstall::default());

    assert_eq!(listing.rows(), []);
    assert_eq!(listing.get_note(), None);
}

#[test]
fn no_agent_dir_installs_nothing() {
    let home = Home::new();

    let listing = install_in(&SKILL, home.agent_dirs.clone(), &SkillsInstall::default());

    assert_eq!(listing.rows(), []);
    assert!(!home.path(".claude").exists());
}

#[test]
fn install_fails_when_its_temporary_file_name_is_taken() {
    let home = Home::new();
    let agents = home.mkdir(".agents/skills");
    fs::create_dir(agents.join("shelf")).unwrap();
    let taken = agents.join(format!("shelf/.SKILL.md.{}.tmp", std::process::id()));
    write(&taken, "someone else's file");

    let rows = home.install(&SkillsInstall::default());

    assert_eq!(states(&rows), [Installed::Failed]);
    assert_eq!(fs::read_to_string(&taken).unwrap(), "someone else's file");
    assert!(!agents.join("shelf/SKILL.md").exists());
}

#[test]
fn a_named_path_that_is_a_file_fails_as_not_a_directory() {
    let home = Home::new();
    let file = home.path("skills");
    write(&file, "a file");

    let rows = home.install(&SkillsInstall {
        dir: Some(file),
        force: false,
    });

    assert_eq!(states(&rows), [Installed::Failed]);
    assert_eq!(
        rows[0].reason.as_deref(),
        Some("the path is not a directory")
    );
}

#[cfg(unix)]
mod links {
    use std::fs;

    use super::{Home, Installed, SKILL, SkillsInstall, TEXT, forced, installed, states};
    use crate::testing::{link, write};

    #[test]
    fn a_claude_dir_that_links_to_the_agents_dir_gets_one_write_and_one_row() {
        let home = Home::new();
        let agents = home.mkdir(".agents/skills");
        home.mkdir(".claude");
        let claude = home.path(".claude/skills");
        link(&agents, &claude);

        let rows = home.install(&SkillsInstall::default());

        assert_eq!(states(&rows), [Installed::Written]);
        assert_eq!(
            rows[0].dirs,
            [claude.display().to_string(), agents.display().to_string()]
        );
        assert_eq!(installed(&agents), SKILL.installed());
    }

    #[test]
    fn an_agents_dir_that_links_to_the_claude_dir_gets_one_write_and_one_row() {
        let home = Home::new();
        let claude = home.mkdir(".claude/skills");
        home.mkdir(".agents");
        let agents = home.path(".agents/skills");
        link(&claude, &agents);

        let rows = home.install(&SkillsInstall::default());

        assert_eq!(states(&rows), [Installed::Written]);
        assert_eq!(
            rows[0].dirs,
            [claude.display().to_string(), agents.display().to_string()]
        );
        assert_eq!(installed(&claude), SKILL.installed());
    }

    #[test]
    fn a_tool_folder_that_links_to_a_copy_install_wrote_is_updated_once() {
        let home = Home::new();
        let agents = home.mkdir(".agents/skills");
        let claude = home.mkdir(".claude/skills");
        fs::create_dir(agents.join("shelf")).unwrap();
        let older = super::Skill::new("shelf", "0.2.0", TEXT);
        write(&agents.join("shelf/SKILL.md"), &older.installed());
        link(&agents.join("shelf"), &claude.join("shelf"));

        let rows = home.install(&SkillsInstall::default());

        assert_eq!(states(&rows), [Installed::Written]);
        assert_eq!(
            rows[0].dirs,
            [claude.display().to_string(), agents.display().to_string()]
        );
        assert!(
            fs::symlink_metadata(claude.join("shelf"))
                .unwrap()
                .is_symlink()
        );
        assert_eq!(installed(&agents), SKILL.installed());
    }

    #[test]
    fn a_tool_folder_that_links_elsewhere_is_skipped() {
        let home = Home::new();
        let agents = home.mkdir(".agents/skills");
        let source = home.mkdir("src/shelf/skill");
        write(&source.join("SKILL.md"), TEXT);
        link(&source, &agents.join("shelf"));

        let rows = home.install(&SkillsInstall::default());

        assert_eq!(states(&rows), [Installed::Skipped]);
        let real = fs::canonicalize(&source).unwrap();
        assert_eq!(
            rows[0].reason.as_deref(),
            Some(format!("linked to {}", real.display()).as_str())
        );
        assert_eq!(fs::read_to_string(source.join("SKILL.md")).unwrap(), TEXT);
    }

    #[test]
    fn force_replaces_a_link_elsewhere_with_a_real_folder_and_leaves_its_target() {
        let home = Home::new();
        let agents = home.mkdir(".agents/skills");
        let source = home.mkdir("src/shelf/skill");
        write(&source.join("SKILL.md"), TEXT);
        link(&source, &agents.join("shelf"));

        let rows = home.install(&forced());

        assert_eq!(states(&rows), [Installed::Written]);
        let folder = fs::symlink_metadata(agents.join("shelf")).unwrap();
        assert!(folder.is_dir() && !folder.is_symlink());
        assert_eq!(installed(&agents), SKILL.installed());
        assert_eq!(fs::read_to_string(source.join("SKILL.md")).unwrap(), TEXT);
    }

    #[test]
    fn a_link_that_appears_after_the_plan_is_planned_again_and_skipped() {
        let home = Home::new();
        let agents = home.mkdir(".agents/skills");
        let real = fs::canonicalize(&agents).unwrap();
        let plan = super::super::plan(&SKILL, &real, false);
        assert_eq!(
            plan,
            super::super::Plan::Write {
                folder: real.join("shelf")
            }
        );
        let source = home.mkdir("src/shelf/skill");
        write(&source.join("SKILL.md"), TEXT);
        link(&source, &agents.join("shelf"));

        let row = super::super::run(
            &SKILL,
            &SKILL.installed(),
            std::slice::from_ref(&agents),
            &real,
            plan,
            false,
        );

        assert_eq!(row.state, Installed::Skipped);
        assert_eq!(fs::read_to_string(source.join("SKILL.md")).unwrap(), TEXT);
    }

    #[test]
    fn a_broken_tool_link_is_skipped_and_force_replaces_it() {
        let home = Home::new();
        let agents = home.mkdir(".agents/skills");
        link(&home.path("gone"), &agents.join("shelf"));

        let skipped = home.install(&SkillsInstall::default());
        let replaced = home.install(&forced());

        assert_eq!(states(&skipped), [Installed::Skipped]);
        assert_eq!(states(&replaced), [Installed::Written]);
        assert_eq!(installed(&agents), SKILL.installed());
    }
}

#[test]
fn the_checked_in_schema_matches_the_install_document_schema() {
    let generated = crumbkit::schemas::document_schema(SKILLS_INSTALL);
    let generated = serde_json::to_string_pretty(&generated).unwrap() + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/skills-install-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/skills-install-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}
