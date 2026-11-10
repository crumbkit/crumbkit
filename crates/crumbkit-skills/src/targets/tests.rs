use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use super::{Chosen, Homes, Place, Target, choose, merge, place};

fn homes(home: &str, codex: Option<&str>) -> Homes {
    Homes::new(Some(OsString::from(home)), codex.map(OsString::from))
}

#[test]
fn the_agent_dirs_are_claude_agents_and_codex_under_the_home() {
    let dirs = homes("/home/ada", None).agent_dirs().unwrap();

    assert_eq!(
        dirs,
        [
            PathBuf::from("/home/ada/.claude/skills"),
            PathBuf::from("/home/ada/.agents/skills"),
            PathBuf::from("/home/ada/.codex/skills"),
        ]
    );
}

#[test]
fn an_absolute_codex_home_replaces_the_default_codex_directory() {
    let dirs = homes("/home/ada", Some("/opt/codex")).agent_dirs().unwrap();

    assert_eq!(dirs.last().unwrap(), &PathBuf::from("/opt/codex/skills"));
}

#[test]
fn a_relative_codex_home_is_ignored() {
    let dirs = homes("/home/ada", Some("codex")).agent_dirs().unwrap();

    assert_eq!(
        dirs.last().unwrap(),
        &PathBuf::from("/home/ada/.codex/skills")
    );
}

#[test]
fn no_home_gives_no_agent_dirs_even_with_a_codex_home() {
    let homes = Homes::new(None, Some(OsString::from("/opt/codex")));

    assert_eq!(homes.agent_dirs(), None);
    assert!(matches!(choose(None, &homes), Chosen::NoHome));
}

#[test]
fn a_relative_or_empty_home_counts_as_no_home() {
    assert_eq!(homes("home", None).agent_dirs(), None);
    assert_eq!(homes("", None).agent_dirs(), None);
}

#[test]
fn a_named_dir_is_chosen_without_reading_the_homes() {
    let dir = tempfile::tempdir().unwrap();

    let Chosen::Targets(targets) = choose(Some(dir.path()), &Homes::default()) else {
        panic!("expected the named dir");
    };
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].dirs, [dir.path().to_owned()]);
}

#[test]
fn a_named_dir_that_does_not_exist_is_chosen_as_missing() {
    let chosen = choose(Some("missing-skills".as_ref()), &Homes::default());

    assert!(matches!(chosen, Chosen::NamedMissing(dir) if dir == Path::new("missing-skills")));
}

#[test]
fn no_existing_agent_dir_reports_the_paths_tried() {
    let home = tempfile::tempdir().unwrap();
    let homes = homes(home.path().to_str().unwrap(), None);

    let Chosen::NoAgents(tried) = choose(None, &homes) else {
        panic!("expected no agent dirs");
    };
    assert_eq!(tried.len(), 3);
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
