use std::fs;
use std::path::{Path, PathBuf};

use clap::Command;
use tempfile::TempDir;

use super::install_in;
use crate::completion::CompletionInstall;
use crate::completion::rows::{InstallRow, Installed};
use crate::completion::script::{installed, registration};
use crate::completion::shell::{Env, Shell};
use crate::{Error, Listing};

fn shelf() -> Command {
    Command::new("shelf").subcommand(Command::new("books"))
}

/// A temporary home whose environment names only the home.
struct Home {
    dir: TempDir,
}

impl Home {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    fn env(&self) -> Env {
        Env {
            home: Some(self.dir.path().to_owned()),
            ..Env::default()
        }
    }

    fn install(&self, shell: Shell, force: bool) -> Listing<InstallRow> {
        install_with(&self.env(), shell, force).unwrap()
    }

    /// The path of fish's completion file, with its folder created.
    fn fish_file(&self) -> PathBuf {
        let dir = self.path(".config/fish/completions");
        #[expect(
            clippy::disallowed_methods,
            reason = "the test creates the completion directory that install finds"
        )]
        fs::create_dir_all(&dir).unwrap();
        dir.join("shelf.fish")
    }
}

fn install_with(env: &Env, shell: Shell, force: bool) -> Result<Listing<InstallRow>, Error> {
    let args = CompletionInstall {
        shell: Some(shell),
        force,
    };
    install_in(&shelf(), &args, env, "COMPLETE")
}

/// The text that install writes for `shell`.
fn text(shell: Shell) -> String {
    let script = registration(shell, "COMPLETE", &shelf()).unwrap();
    installed(shell, &shelf(), &script)
}

fn only(listing: &Listing<InstallRow>) -> &InstallRow {
    let [row] = listing.rows() else {
        panic!("install returns one row")
    };
    row
}

fn write(path: &Path, text: &str) {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tests write the files that install then finds"
    )]
    fs::write(path, text).unwrap();
}

#[test]
fn install_writes_each_shell_s_file_and_creates_its_directory() {
    for (shell, relative) in [
        (Shell::Fish, ".config/fish/completions/shelf.fish"),
        (
            Shell::Bash,
            ".local/share/bash-completion/completions/shelf",
        ),
        (Shell::Zsh, ".zfunc/_shelf"),
    ] {
        let home = Home::new();

        let listing = home.install(shell, false);

        assert_eq!(only(&listing).state, Installed::Written);
        assert_eq!(
            fs::read_to_string(home.path(relative)).unwrap(),
            text(shell)
        );
    }
}

#[test]
fn install_into_zfunc_adds_the_note_with_the_fpath_line() {
    let home = Home::new();

    let listing = home.install(Shell::Zsh, false);

    let note = listing.get_note().unwrap();
    assert!(note.contains("fpath=("), "{note}");
    assert!(note.contains(".zfunc $fpath)"), "{note}");
}

#[test]
fn an_unchanged_zfunc_file_gives_no_fpath_note() {
    let home = Home::new();
    home.install(Shell::Zsh, false);

    let listing = home.install(Shell::Zsh, false);

    assert_eq!(only(&listing).state, Installed::Unchanged);
    assert_eq!(listing.get_note(), None);
}

#[test]
fn a_second_install_leaves_the_file_unchanged() {
    let home = Home::new();
    home.install(Shell::Fish, false);

    let listing = home.install(Shell::Fish, false);

    assert_eq!(only(&listing).state, Installed::Unchanged);
}

#[test]
fn install_replaces_its_own_file_from_another_build() {
    let home = Home::new();
    let file = home.fish_file();
    let old = text(Shell::Fish).replace("COMPLETE=fish", "OLD=fish");
    write(&file, &old);

    let listing = home.install(Shell::Fish, false);

    assert_eq!(only(&listing).state, Installed::Written);
    assert_eq!(fs::read_to_string(&file).unwrap(), text(Shell::Fish));
}

#[test]
fn install_skips_a_foreign_file_and_force_replaces_it() {
    let home = Home::new();
    let file = home.fish_file();
    write(&file, "complete -c shelf -a books\n");

    let skipped = home.install(Shell::Fish, false);

    assert_eq!(only(&skipped).state, Installed::Skipped);
    assert_eq!(
        only(&skipped).reason.as_deref(),
        Some("not written by shelf completion install")
    );
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "complete -c shelf -a books\n"
    );

    let replaced = home.install(Shell::Fish, true);

    assert_eq!(only(&replaced).state, Installed::Written);
    assert_eq!(
        only(&replaced).reason.as_deref(),
        Some("replaced: not written by shelf completion install")
    );
    assert_eq!(fs::read_to_string(&file).unwrap(), text(Shell::Fish));
}

#[cfg(unix)]
#[test]
fn install_follows_a_link_to_its_own_file_and_updates_that_file() {
    let home = Home::new();
    let file = home.fish_file();
    let real = home.path("dotfiles-shelf.fish");
    write(
        &real,
        &text(Shell::Fish).replace("COMPLETE=fish", "OLD=fish"),
    );
    std::os::unix::fs::symlink(&real, &file).unwrap();

    let listing = home.install(Shell::Fish, false);

    assert_eq!(only(&listing).state, Installed::Written);
    assert!(fs::symlink_metadata(&file).unwrap().is_symlink());
    assert_eq!(fs::read_to_string(&real).unwrap(), text(Shell::Fish));
}

#[cfg(unix)]
#[test]
fn install_skips_a_link_to_a_foreign_file_and_force_replaces_only_the_link() {
    let home = Home::new();
    let file = home.fish_file();
    let real = home.path("dotfiles-shelf.fish");
    write(&real, "complete -c shelf -a books\n");
    std::os::unix::fs::symlink(&real, &file).unwrap();

    let skipped = home.install(Shell::Fish, false);

    assert_eq!(only(&skipped).state, Installed::Skipped);
    let reason = only(&skipped).reason.clone().unwrap();
    assert!(reason.starts_with("linked to "), "{reason}");
    assert!(reason.ends_with("dotfiles-shelf.fish"), "{reason}");

    let replaced = home.install(Shell::Fish, true);

    assert_eq!(only(&replaced).state, Installed::Written);
    assert!(!fs::symlink_metadata(&file).unwrap().is_symlink());
    assert_eq!(fs::read_to_string(&file).unwrap(), text(Shell::Fish));
    assert_eq!(
        fs::read_to_string(&real).unwrap(),
        "complete -c shelf -a books\n"
    );
}

#[cfg(unix)]
#[test]
fn install_skips_a_broken_link() {
    let home = Home::new();
    let file = home.fish_file();
    std::os::unix::fs::symlink(home.path("gone"), &file).unwrap();

    let listing = home.install(Shell::Fish, false);

    assert_eq!(only(&listing).state, Installed::Skipped);
}

#[test]
fn install_fails_on_a_directory_at_the_file_s_path_even_with_force() {
    let home = Home::new();
    let file = home.fish_file();
    fs::create_dir(&file).unwrap();

    let listing = home.install(Shell::Fish, true);

    assert_eq!(only(&listing).state, Installed::Failed);
    assert!(file.is_dir());
}

#[test]
fn install_fails_and_overwrites_nothing_when_its_temporary_name_is_taken() {
    let home = Home::new();
    let file = home.fish_file();
    let taken = file.with_file_name(format!(".shelf.fish.{}.tmp", std::process::id()));
    write(&taken, "someone else's\n");

    let listing = home.install(Shell::Fish, false);

    assert_eq!(only(&listing).state, Installed::Failed);
    assert!(!file.exists());
    assert_eq!(fs::read_to_string(&taken).unwrap(), "someone else's\n");
}

#[test]
fn install_writes_under_xdg_config_home_without_a_home() {
    let home = Home::new();
    let env = Env {
        config_home: Some(home.path("config")),
        ..Env::default()
    };

    let listing = install_with(&env, Shell::Fish, false).unwrap();

    assert_eq!(only(&listing).state, Installed::Written);
    assert!(home.path("config/fish/completions/shelf.fish").is_file());
}

#[test]
fn install_without_a_home_writes_nothing_and_lists_no_row() {
    let listing = install_with(&Env::default(), Shell::Zsh, false).unwrap();

    assert_eq!(listing.rows(), []);
}

#[test]
fn install_uses_the_shell_that_the_shell_variable_names() {
    let home = Home::new();
    let env = Env {
        shell: Some("/usr/bin/bash".into()),
        ..home.env()
    };
    let args = CompletionInstall::default();

    let listing = install_in(&shelf(), &args, &env, "COMPLETE").unwrap();

    assert_eq!(only(&listing).shell, Shell::Bash);
}

#[test]
fn install_refuses_an_unset_or_unknown_shell_and_a_shell_without_a_directory() {
    let home = Home::new();
    let args = CompletionInstall::default();
    let unknown = Env {
        shell: Some("/bin/tcsh".into()),
        ..home.env()
    };

    let codes = [
        install_in(&shelf(), &args, &home.env(), "COMPLETE"),
        install_in(&shelf(), &args, &unknown, "COMPLETE"),
        install_with(&home.env(), Shell::Elvish, false),
    ]
    .map(|result| result.unwrap_err().failure().code().to_buf());

    assert_eq!(
        codes,
        [
            crate::codes::SHELL_UNSET,
            crate::codes::SHELL_UNKNOWN,
            crate::codes::SHELL_NO_DIRECTORY
        ]
        .map(crate::Code::to_buf)
    );
}
