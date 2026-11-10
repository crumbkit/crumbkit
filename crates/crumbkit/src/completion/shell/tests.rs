use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use super::{Env, Shell, ShellError, chosen, target};

/// An environment whose home is `home`, with nothing else set.
fn env(home: &Path) -> Env {
    Env {
        home: Some(home.to_owned()),
        ..Env::default()
    }
}

fn file(shell: Shell, env: &Env) -> PathBuf {
    target(shell, "shelf", env).unwrap().unwrap().file
}

#[test]
fn the_shell_flag_wins_over_the_shell_variable() {
    let chosen = chosen(Some(Shell::Zsh), Some(&OsString::from("/bin/bash")));

    assert_eq!(chosen.unwrap(), Shell::Zsh);
}

#[test]
fn the_shell_variable_names_the_shell_by_its_file_name() {
    for (variable, shell) in [
        ("/bin/bash", Shell::Bash),
        ("/usr/local/bin/fish", Shell::Fish),
        ("/bin/zsh", Shell::Zsh),
        ("zsh", Shell::Zsh),
        ("/usr/bin/elvish", Shell::Elvish),
    ] {
        assert_eq!(chosen(None, Some(&OsString::from(variable))), Ok(shell));
    }
}

#[test]
fn an_unset_or_empty_shell_variable_is_refused_with_its_own_failure() {
    assert_eq!(chosen(None, None), Err(ShellError::Unset));
    assert_eq!(chosen(None, Some(&OsString::new())), Err(ShellError::Unset));
}

#[test]
fn a_shell_variable_that_names_an_unknown_shell_is_refused_with_the_name() {
    let refused = chosen(None, Some(&OsString::from("/bin/tcsh")));

    assert_eq!(
        refused,
        Err(ShellError::Unknown {
            name: "tcsh".to_owned()
        })
    );
    assert_eq!(
        refused.unwrap_err().to_string(),
        "SHELL names tcsh, which completion does not serve"
    );
}

#[test]
fn fish_completions_go_under_the_config_directory_of_the_home() {
    let file = file(Shell::Fish, &env(Path::new("/home/ada")));

    assert_eq!(
        file,
        Path::new("/home/ada/.config/fish/completions/shelf.fish")
    );
}

#[test]
fn fish_completions_go_under_xdg_config_home_when_it_is_set() {
    let env = Env {
        config_home: Some("/xdg/config".into()),
        ..Env::default()
    };

    assert_eq!(
        file(Shell::Fish, &env),
        Path::new("/xdg/config/fish/completions/shelf.fish")
    );
}

#[test]
fn bash_completions_go_under_the_data_directory_of_the_home() {
    let file = file(Shell::Bash, &env(Path::new("/home/ada")));

    assert_eq!(
        file,
        Path::new("/home/ada/.local/share/bash-completion/completions/shelf")
    );
}

#[test]
fn bash_completions_go_under_xdg_data_home_when_it_is_set() {
    let env = Env {
        home: Some("/home/ada".into()),
        data_home: Some("/xdg/data".into()),
        ..Env::default()
    };

    assert_eq!(
        file(Shell::Bash, &env),
        Path::new("/xdg/data/bash-completion/completions/shelf")
    );
}

#[test]
fn zsh_completions_fall_back_to_zfunc_with_a_note_that_gives_the_fpath_line() {
    let target = target(Shell::Zsh, "shelf", &env(Path::new("/home/ada")))
        .unwrap()
        .unwrap();

    assert_eq!(target.file, Path::new("/home/ada/.zfunc/_shelf"));
    assert_eq!(
        target.note.as_deref(),
        Some(
            "zsh loads completions only from a directory on fpath; add `fpath=(/home/ada/.zfunc $fpath)` before `compinit` in ~/.zshrc."
        )
    );
}

#[test]
fn zsh_completions_go_in_the_first_fpath_directory_under_the_home_that_exists() {
    let home = tempfile::tempdir().unwrap();
    let functions = home.path().join("functions");
    fs::create_dir(&functions).unwrap();
    let missing = home.path().join("missing");
    let fpath = std::env::join_paths([
        PathBuf::from("/usr/share/zsh/functions"),
        missing,
        functions.clone(),
    ])
    .unwrap();
    let env = Env {
        fpath: Some(fpath),
        ..env(home.path())
    };

    let target = target(Shell::Zsh, "shelf", &env).unwrap().unwrap();

    assert_eq!(target.file, functions.join("_shelf"));
    assert_eq!(target.note, None);
}

#[test]
fn no_home_and_no_xdg_directory_gives_no_target() {
    for shell in [Shell::Bash, Shell::Fish, Shell::Zsh] {
        assert_eq!(target(shell, "shelf", &Env::default()), Ok(None));
    }
}

#[test]
fn elvish_and_powershell_have_no_completion_directory() {
    let env = env(Path::new("/home/ada"));

    for shell in [Shell::Elvish, Shell::Powershell] {
        assert_eq!(
            target(shell, "shelf", &env),
            Err(ShellError::NoDirectory { shell })
        );
    }
}
