//! End-to-end tests of shelf's `completion` commands. Each test runs the
//! built binary with a cleared environment and `HOME` set to a temporary
//! directory of its own, so no test installs into the user's home directory
//! (ADR-0195).

use std::fs;
use std::path::Path;

use snapbox::cmd::{Command, OutputAssert};

/// Runs the built `shelf` binary with `args`, a cleared environment, `HOME`
/// set to `home`, the XDG state directory under it and `SHELL` set to
/// `shell` when it is given, and returns its output for assertions.
fn shelf(home: &Path, shell: Option<&str>, args: &[&str]) -> OutputAssert {
    let mut command = Command::new(env!("CARGO_BIN_EXE_shelf"))
        .args(args)
        .env_clear()
        .env("HOME", home)
        .env("XDG_STATE_HOME", home.join("state"));
    if let Some(shell) = shell {
        command = command.env("SHELL", shell);
    }
    command.assert()
}

fn stdout(output: &OutputAssert) -> String {
    String::from_utf8_lossy(&output.get_output().stdout).into_owned()
}

fn stderr(output: &OutputAssert) -> String {
    String::from_utf8_lossy(&output.get_output().stderr).into_owned()
}

#[test]
fn install_for_the_shell_that_shell_names_writes_zfunc_and_gives_the_fpath_line() {
    let home = tempfile::tempdir().unwrap();

    let output = shelf(home.path(), Some("/bin/zsh"), &["completion", "install"]).success();

    let text = fs::read_to_string(home.path().join(".zfunc/_shelf")).unwrap();
    assert!(
        text.starts_with("#compdef shelf\n# shelf completion install wrote this file"),
        "{text}"
    );
    assert!(text.contains("COMPLETE=\"zsh\""), "{text}");
    assert!(stdout(&output).contains("written"), "{}", stdout(&output));
    assert!(stderr(&output).contains("fpath=("), "{}", stderr(&output));
}

#[test]
fn check_exits_with_1_for_a_missing_file_and_with_0_after_install() {
    let home = tempfile::tempdir().unwrap();
    let check = ["completion", "check", "--shell", "fish"];

    let missing = shelf(home.path(), None, &check).code(1);
    shelf(
        home.path(),
        None,
        &["completion", "install", "--shell", "fish"],
    )
    .success();
    let current = shelf(home.path(), None, &check).success();

    assert!(stdout(&missing).contains("missing"), "{}", stdout(&missing));
    assert!(
        home.path()
            .join(".config/fish/completions/shelf.fish")
            .is_file()
    );
    assert_eq!(stdout(&current), "");
}

#[test]
fn install_skips_a_foreign_file_and_force_replaces_it() {
    let home = tempfile::tempdir().unwrap();
    let dir = home.path().join(".local/share/bash-completion/completions");
    #[expect(
        clippy::disallowed_methods,
        reason = "the test creates the completion directory that install finds"
    )]
    fs::create_dir_all(&dir).unwrap();
    #[expect(
        clippy::disallowed_methods,
        reason = "the test leaves a file of another program's"
    )]
    fs::write(dir.join("shelf"), "complete -W books shelf\n").unwrap();
    let install = ["completion", "install", "--shell", "bash"];

    let skipped = shelf(home.path(), None, &install).success();
    let forced = [&install[..], &["--force"]].concat();
    shelf(home.path(), None, &forced).success();

    assert!(stdout(&skipped).contains("skipped"), "{}", stdout(&skipped));
    let text = fs::read_to_string(dir.join("shelf")).unwrap();
    assert!(
        text.starts_with("# shelf completion install wrote this file"),
        "{text}"
    );
}

#[test]
fn print_without_shell_fails_with_the_unset_code_and_with_shell_prints_the_script() {
    let home = tempfile::tempdir().unwrap();

    let refused = shelf(home.path(), None, &["completion", "print", "--json"]).code(1);
    let twin = shelf(
        home.path(),
        None,
        &["completion", "print", "--json", "--shell", "fish"],
    )
    .success();

    assert!(
        stdout(&refused).contains("shelf::shell::unset"),
        "{}",
        stdout(&refused)
    );
    assert!(
        stdout(&twin).contains("COMPLETE=fish shelf"),
        "{}",
        stdout(&twin)
    );
}

#[test]
fn install_refuses_elvish_with_the_no_directory_code() {
    let home = tempfile::tempdir().unwrap();

    let refused = shelf(
        home.path(),
        Some("/usr/bin/elvish"),
        &["completion", "install", "--json"],
    )
    .code(1);

    assert!(
        stdout(&refused).contains("shelf::shell::no_directory"),
        "{}",
        stdout(&refused)
    );
}
