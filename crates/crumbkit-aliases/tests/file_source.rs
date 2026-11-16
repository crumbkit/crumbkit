//! A file source is read for every command line, so a personal key can start
//! with a command.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::PathBuf;

use clap::Command;
use crumbkit_aliases::{Aliases, Source};

/// Returns a tool with a `books` command that has a `list` subcommand.
fn tool() -> Command {
    Command::new("shelf")
        .disable_help_subcommand(true)
        .subcommand(Command::new("books").subcommand(Command::new("list")))
}

/// Writes `text` to a new aliases file under cargo's scratch directory.
#[expect(
    clippy::disallowed_methods,
    reason = "the test writes the aliases file that the source reads"
)]
fn aliases_file(name: &str, text: &str) -> io::Result<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    fs::create_dir_all(&dir)?;
    let path = dir.join(name);
    fs::write(&path, text)?;
    Ok(path)
}

#[test]
fn a_file_key_that_starts_with_a_command_expands_when_the_file_is_on_disk() -> io::Result<()> {
    let path = aliases_file(
        "file-source-books-recent.toml",
        r#""books recent" = ["books", "list", "--limit", "5"]"#,
    )?;
    let aliases = Aliases::new().source(Source::file(Some(path)));

    let args = aliases.expand(&tool(), ["shelf", "books", "recent"]);

    let expected: Vec<OsString> = ["shelf", "books", "list", "--limit", "5"]
        .map(OsString::from)
        .into();
    assert_eq!(args, expected);
    Ok(())
}
