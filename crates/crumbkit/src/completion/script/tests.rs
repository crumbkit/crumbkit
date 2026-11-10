use clap::Command;

use super::{installed, registration, written_by};
use crate::completion::shell::Shell;

fn shelf() -> Command {
    Command::new("shelf").subcommand(Command::new("books"))
}

#[test]
fn the_registration_runs_the_tool_by_its_name_with_the_completion_variable() {
    let script = registration(Shell::Fish, "COMPLETE", &shelf()).unwrap();

    assert_eq!(
        script,
        "complete --keep-order --exclusive --command shelf --arguments \"(COMPLETE=fish shelf -- (commandline --current-process --tokenize --cut-at-cursor) (commandline --current-token))\"\n"
    );
}

#[test]
fn the_registration_sets_the_variable_that_the_tool_declared() {
    let script = registration(Shell::Bash, "SHELF_COMPLETE", &shelf()).unwrap();

    assert!(script.contains("SHELF_COMPLETE=\"bash\""), "{script}");
    assert!(script.contains("-F _clap_complete_shelf shelf"), "{script}");
}

#[test]
fn every_shell_has_a_registration() {
    for shell in [
        Shell::Bash,
        Shell::Elvish,
        Shell::Fish,
        Shell::Powershell,
        Shell::Zsh,
    ] {
        let script = registration(shell, "COMPLETE", &shelf()).unwrap();
        assert!(script.contains("shelf"), "{script}");
    }
}

#[test]
fn the_installed_fish_and_bash_files_start_with_the_mark() {
    for shell in [Shell::Fish, Shell::Bash] {
        let script = registration(shell, "COMPLETE", &shelf()).unwrap();
        let text = installed(shell, &shelf(), &script);

        assert!(
            text.starts_with(
                "# shelf completion install wrote this file, and the next install replaces it.\n"
            ),
            "{text}"
        );
        assert!(text.ends_with(&script));
    }
}

#[test]
fn the_installed_zsh_file_keeps_compdef_first_and_answers_the_first_tab_press() {
    let script = registration(Shell::Zsh, "COMPLETE", &shelf()).unwrap();

    let text = installed(Shell::Zsh, &shelf(), &script);

    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("#compdef shelf"));
    assert_eq!(
        lines.next(),
        Some("# shelf completion install wrote this file, and the next install replaces it.")
    );
    assert!(script.contains("function _clap_dynamic_completer_shelf() {"));
    assert!(
        text.ends_with("\n_clap_dynamic_completer_shelf \"$@\"\n"),
        "{text}"
    );
}

#[test]
fn a_file_is_the_tool_s_own_only_when_its_first_or_second_line_is_its_mark() {
    for shell in [Shell::Fish, Shell::Zsh] {
        let script = registration(shell, "COMPLETE", &shelf()).unwrap();
        assert!(written_by(&installed(shell, &shelf(), &script), "shelf"));
        assert!(!written_by(
            &installed(shell, &Command::new("shelves"), &script),
            "shelf"
        ));
        assert!(!written_by(&script, "shelf"));
    }
    assert!(!written_by(
        "a\nb\n# shelf completion install wrote this file, and the next install replaces it.\n",
        "shelf"
    ));
}
