use std::fs;
use std::path::{Path, PathBuf};

use clap::Command;
use tempfile::TempDir;

use super::check_in;
use crate::Listing;
use crate::completion::rows::{CheckRow, Checked};
use crate::completion::script::{installed, registration};
use crate::completion::shell::{Env, Shell};

fn shelf() -> Command {
    Command::new("shelf").subcommand(Command::new("books"))
}

fn text() -> String {
    let script = registration(Shell::Fish, "COMPLETE", &shelf()).unwrap();
    installed(Shell::Fish, &shelf(), &script)
}

/// A temporary home with fish's completion directory.
struct Home {
    dir: TempDir,
}

impl Home {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        #[expect(
            clippy::disallowed_methods,
            reason = "the test creates the completion directory that install finds"
        )]
        fs::create_dir_all(dir.path().join(".config/fish/completions")).unwrap();
        Self { dir }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    fn file(&self) -> PathBuf {
        self.path(".config/fish/completions/shelf.fish")
    }

    fn check(&self) -> Listing<CheckRow> {
        let env = Env {
            home: Some(self.dir.path().to_owned()),
            ..Env::default()
        };
        check_in(&shelf(), Some(Shell::Fish), &env, "COMPLETE").unwrap()
    }
}

fn write(path: &Path, text: &str) {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tests write the files that check then finds"
    )]
    fs::write(path, text).unwrap();
}

fn state(listing: &Listing<CheckRow>) -> Checked {
    let [row] = listing.rows() else {
        panic!("check returns one row")
    };
    row.state
}

/// Reports whether `finish` exits with 1 for the listing: whether a row
/// reports a problem.
fn fails(listing: &Listing<CheckRow>) -> bool {
    listing.rows().iter().any(crate::row::problems::reports)
}

#[test]
fn check_reports_a_missing_file_and_exits_with_1() {
    let home = Home::new();

    let listing = home.check();

    assert_eq!(state(&listing), Checked::Missing);
    assert!(fails(&listing));
}

#[test]
fn check_reports_the_file_that_install_writes_as_current_and_exits_with_0() {
    let home = Home::new();
    write(&home.file(), &text());

    let listing = home.check();

    assert_eq!(state(&listing), Checked::Current);
    assert!(!fails(&listing));
}

#[test]
fn check_reports_its_own_file_from_another_build_and_a_foreign_file_as_differing() {
    let home = Home::new();
    write(&home.file(), &text().replace("COMPLETE=fish", "OLD=fish"));
    let own = home.check();
    write(&home.file(), "complete -c shelf -a books\n");
    let foreign = home.check();

    assert_eq!(state(&own), Checked::Differs);
    assert_eq!(state(&foreign), Checked::Differs);
    let reason = foreign.rows()[0].reason.clone().unwrap();
    assert!(reason.contains("install --force replaces it"), "{reason}");
}

#[cfg(unix)]
#[test]
fn check_follows_a_link_to_its_own_file_and_reports_a_foreign_link_as_linked() {
    let home = Home::new();
    let real = home.path("dotfiles-shelf.fish");
    write(&real, &text());
    std::os::unix::fs::symlink(&real, home.file()).unwrap();
    let own = home.check();
    write(&real, "complete -c shelf -a books\n");
    let foreign = home.check();

    assert_eq!(state(&own), Checked::Current);
    assert_eq!(state(&foreign), Checked::Linked);
    assert!(!fails(&foreign));
}

#[test]
fn check_reports_a_directory_at_the_file_s_path_as_unreadable() {
    let home = Home::new();
    fs::create_dir(home.file()).unwrap();

    assert_eq!(state(&home.check()), Checked::Unreadable);
}
