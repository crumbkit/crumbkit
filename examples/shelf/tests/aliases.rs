//! End-to-end tests of a person's aliases in shelf. Each test writes an
//! aliases file under an `XDG_CONFIG_HOME` in a temporary directory of its
//! own, runs the built binary and
//! checks its output (ADR-0027).

use std::io;
use std::path::Path;

use tempfile::TempDir;

use snapbox::cmd::{Command, OutputAssert};
use snapbox::file;

/// The aliases file that every test writes: a good alias, an entry that a
/// real command shadows and an entry with a bad value.
const ALIASES: &str = r#"
firsts = ["books", "list", "--long"]
books = ["books", "show", "1"]
broken = "books list"
"#;

/// Returns a temporary directory of the test's own, which is removed when
/// the test ends, with [`ALIASES`] written to shelf's aliases file under its
/// `config` directory.
///
/// # Errors
///
/// Returns the I/O error of a directory or file that cannot be written.
fn home() -> io::Result<TempDir> {
    home_with(ALIASES)
}

/// Returns a temporary directory as [`home`] does, with `aliases` written
/// to shelf's aliases file.
///
/// # Errors
///
/// Returns the I/O error of a directory or file that cannot be written.
fn home_with(aliases: &str) -> io::Result<TempDir> {
    let home = tempfile::tempdir()?;
    let config = home.path().join("config").join("shelf");
    #[expect(
        clippy::disallowed_methods,
        reason = "this helper owns the tests' file writes, which set up the aliases file"
    )]
    {
        std::fs::create_dir_all(&config)?;
        std::fs::write(config.join("aliases.toml"), aliases)?;
    }
    Ok(home)
}

/// Runs the built `shelf` binary with `args`, a cleared environment and
/// `XDG_CONFIG_HOME` and `XDG_STATE_HOME` set to the directories `config`
/// and `state` under `home`, so shelf's usage log stays out of the user's
/// home directory, and returns its output for assertions.
fn shelf(home: &Path, args: &[&str]) -> OutputAssert {
    Command::new(env!("CARGO_BIN_EXE_shelf"))
        .args(args)
        .env_clear()
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .assert()
}

#[test]
fn a_personal_alias_runs_its_command_with_the_words_typed_after_it() {
    let home = home().unwrap();
    let real = shelf(home.path(), &["books", "list", "--long", "--limit", "1"]).success();

    shelf(home.path(), &["firsts", "--limit", "1"])
        .success()
        .stdout_eq(real.get_output().stdout.clone())
        .stderr_eq(real.get_output().stderr.clone());
}

#[test]
fn aliases_list_prints_the_entries_with_a_problem_then_the_built_in_alias_then_the_rest() {
    let home = home().unwrap();

    shelf(home.path(), &["aliases", "list"])
        .code(1)
        .stdout_eq(file!["golden/aliases-list.txt"])
        .stderr_eq(file!["golden/aliases-list.stderr"]);
}

#[test]
fn aliases_list_exits_with_0_when_every_alias_works() {
    let home = home_with(r#"firsts = ["books", "list", "--long"]"#).unwrap();

    shelf(home.path(), &["aliases", "list"])
        .success()
        .stderr_eq("Aliases file: [..]/config/shelf/aliases.toml\n");
}

#[test]
fn the_root_s_help_names_each_personal_alias_that_expands() {
    let home = home().unwrap();

    for flag in ["-h", "--help"] {
        shelf(home.path(), &[flag])
            .success()
            .stdout_eq(
                "...\nPersonal aliases: firsts. `shelf aliases list` shows what each runs.\nNext: shelf <COMMAND> -h\n",
            )
            .stderr_eq("");
    }
}
