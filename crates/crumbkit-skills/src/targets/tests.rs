use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use super::{Place, Target, agent_dirs, merge, named_dir, place};

fn dirs(home: &str, codex: Option<&str>) -> Option<Vec<PathBuf>> {
    agent_dirs(Some(PathBuf::from(home)), codex.map(OsString::from))
}

#[test]
fn the_agent_dirs_are_claude_agents_and_codex_under_the_home() {
    assert_eq!(
        dirs("/home/ada", None).unwrap(),
        [
            PathBuf::from("/home/ada/.claude/skills"),
            PathBuf::from("/home/ada/.agents/skills"),
            PathBuf::from("/home/ada/.codex/skills"),
        ]
    );
}

#[test]
fn an_absolute_codex_home_replaces_the_default_codex_directory() {
    let dirs = dirs("/home/ada", Some("/opt/codex")).unwrap();

    assert_eq!(dirs.last().unwrap(), &PathBuf::from("/opt/codex/skills"));
}

#[test]
fn a_relative_codex_home_is_ignored() {
    let dirs = dirs("/home/ada", Some("codex")).unwrap();

    assert_eq!(
        dirs.last().unwrap(),
        &PathBuf::from("/home/ada/.codex/skills")
    );
}

#[test]
fn no_home_gives_no_agent_dirs_even_with_a_codex_home() {
    assert_eq!(agent_dirs(None, Some(OsString::from("/opt/codex"))), None);
}

#[test]
fn a_named_dir_resolves_to_its_real_path() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(
        named_dir(dir.path()),
        Ok(fs::canonicalize(dir.path()).unwrap())
    );
}

#[test]
fn a_named_dir_that_does_not_exist_says_so() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(
        named_dir(&dir.path().join("missing")),
        Err("the directory does not exist".to_owned())
    );
}

#[test]
fn a_named_dir_that_is_a_file_says_it_is_not_a_directory() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("skills");
    crate::testing::write(&file, "a file");

    assert_eq!(
        named_dir(&file),
        Err("the path is not a directory".to_owned())
    );
}

#[test]
fn merge_keeps_only_the_dirs_that_exist() {
    let home = tempfile::tempdir().unwrap();
    let agents = home.path().join("agents");
    fs::create_dir(&agents).unwrap();

    let targets = merge(&[home.path().join("missing"), agents.clone()]);

    assert_eq!(
        targets,
        [Target {
            dirs: vec![agents.clone()],
            real: fs::canonicalize(&agents).unwrap(),
        }]
    );
}

#[cfg(unix)]
#[test]
fn merge_joins_a_dir_and_a_link_to_it_in_the_order_given() {
    let home = tempfile::tempdir().unwrap();
    let agents = home.path().join("agents");
    let claude = home.path().join("claude");
    fs::create_dir(&agents).unwrap();
    crate::testing::link(&agents, &claude);

    let targets = merge(&[claude.clone(), agents.clone()]);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].dirs, [claude, agents.clone()]);
    assert_eq!(targets[0].real, fs::canonicalize(&agents).unwrap());
}

#[test]
fn place_reports_an_absent_folder_and_a_real_one() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(
        place(dir.path(), "shelf"),
        Place::Absent {
            folder: dir.path().join("shelf")
        }
    );
    fs::create_dir(dir.path().join("shelf")).unwrap();
    assert_eq!(
        place(dir.path(), "shelf"),
        Place::Folder {
            folder: dir.path().join("shelf")
        }
    );
}

#[test]
fn place_reports_a_file_where_the_folder_belongs() {
    let dir = tempfile::tempdir().unwrap();
    crate::testing::write(&dir.path().join("shelf"), "a file");

    assert_eq!(
        place(dir.path(), "shelf"),
        Place::NotFolder {
            path: dir.path().join("shelf")
        }
    );
}

#[cfg(unix)]
#[test]
fn place_follows_a_link_to_a_folder_and_reports_a_broken_one() {
    let dir = tempfile::tempdir().unwrap();
    let elsewhere = dir.path().join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    crate::testing::link(&elsewhere, &dir.path().join("shelf"));
    crate::testing::link(&dir.path().join("gone"), &dir.path().join("reading"));

    assert_eq!(
        place(dir.path(), "shelf"),
        Place::Linked {
            link: dir.path().join("shelf"),
            to: fs::canonicalize(&elsewhere).unwrap(),
        }
    );
    assert_eq!(
        place(dir.path(), "reading"),
        Place::Broken {
            link: dir.path().join("reading"),
            to: dir.path().join("gone"),
        }
    );
}

#[cfg(unix)]
#[test]
fn place_reports_a_link_loop_as_broken() {
    let dir = tempfile::tempdir().unwrap();
    crate::testing::link(&dir.path().join("other"), &dir.path().join("shelf"));
    crate::testing::link(&dir.path().join("shelf"), &dir.path().join("other"));

    assert_eq!(
        place(dir.path(), "shelf"),
        Place::Broken {
            link: dir.path().join("shelf"),
            to: dir.path().join("other"),
        }
    );
}

#[cfg(unix)]
#[test]
fn place_follows_a_relative_link_from_the_link_s_own_folder() {
    let dir = tempfile::tempdir().unwrap();
    let skills = dir.path().join("skills");
    let elsewhere = dir.path().join("elsewhere");
    fs::create_dir(&skills).unwrap();
    fs::create_dir(&elsewhere).unwrap();
    crate::testing::link(std::path::Path::new("../elsewhere"), &skills.join("shelf"));

    assert_eq!(
        place(&skills, "shelf"),
        Place::Linked {
            link: skills.join("shelf"),
            to: fs::canonicalize(&elsewhere).unwrap(),
        }
    );
}

#[cfg(unix)]
#[test]
fn merge_joins_a_dir_and_a_relative_link_to_it() {
    let dir = tempfile::tempdir().unwrap();
    let agents = dir.path().join("agents");
    fs::create_dir(&agents).unwrap();
    crate::testing::link(std::path::Path::new("agents"), &dir.path().join("claude"));

    let targets = merge(&[dir.path().join("claude"), agents]);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].dirs.len(), 2);
}
