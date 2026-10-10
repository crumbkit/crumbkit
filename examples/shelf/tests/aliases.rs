//! End-to-end tests of a person's aliases in shelf. Each test writes an
//! aliases file under its own `XDG_CONFIG_HOME`, runs the built binary and
//! checks its output (ADR-0027).

use std::io;
use std::path::{Path, PathBuf};

use snapbox::cmd::{Command, OutputAssert};
use snapbox::file;

/// The aliases file that every test writes: a good alias, an entry that a
/// real command shadows and an entry with a bad value.
const ALIASES: &str = r#"
firsts = ["books", "list", "--long"]
books = ["books", "show", "1"]
broken = "books list"
"#;

/// Writes [`ALIASES`] to shelf's aliases file under a config directory of
/// the test's own, and returns that config directory.
///
/// `test` names the directory under cargo's temporary directory for
/// integration tests.
///
/// # Errors
///
/// Returns the I/O error of a directory or file that cannot be written.
fn config(test: &str) -> io::Result<PathBuf> {
    // RUST-10: the directory stays under cargo's temporary directory after
    // the test, and the next run overwrites the file. When a test needs a
    // directory that starts empty, replace it with one that each test
    // creates and removes, such as the temporary directories of snapbox's
    // `dir` feature.
    let config = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(test)
        .join("config");
    #[expect(
        clippy::disallowed_methods,
        reason = "this helper owns the tests' file writes, which set up the aliases file"
    )]
    {
        std::fs::create_dir_all(config.join("shelf"))?;
        std::fs::write(config.join("shelf").join("aliases.toml"), ALIASES)?;
    }
    Ok(config)
}

/// Runs the built `shelf` binary with `args`, a cleared environment and
/// `XDG_CONFIG_HOME` set to `config`, and returns its output for
/// assertions. `XDG_STATE_HOME` is the directory `state` beside `config`,
/// so shelf's usage log stays out of the user's home directory.
fn shelf(config: &Path, args: &[&str]) -> OutputAssert {
    Command::new(env!("CARGO_BIN_EXE_shelf"))
        .args(args)
        .env_clear()
        .env("XDG_CONFIG_HOME", config)
        .env("XDG_STATE_HOME", config.with_file_name("state"))
        .assert()
}

#[test]
fn a_personal_alias_runs_its_command_with_the_words_typed_after_it() {
    let config = config("aliases-firsts").unwrap();
    let real = shelf(&config, &["books", "list", "--long", "--limit", "1"]).success();

    shelf(&config, &["firsts", "--limit", "1"])
        .success()
        .stdout_eq(real.get_output().stdout.clone())
        .stderr_eq("");
}

#[test]
fn aliases_list_prints_the_built_in_alias_then_each_entry_with_its_problem() {
    let config = config("aliases-list").unwrap();

    shelf(&config, &["aliases", "list"])
        .success()
        .stdout_eq(file!["golden/aliases-list.txt"])
        .stderr_eq("");
}
