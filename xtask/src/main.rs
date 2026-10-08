//! `cargo xtask` runs crumbkit's repository checks.
//!
//! `cargo xtask check` runs every check, and `cargo xtask check docs` runs only
//! the docs word check. This module holds every effect: it reads the command
//! line, lists and reads the repository's files, prints violations and sets the
//! exit code. The checks themselves are pure functions in [`check`].

mod check;

use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, ExitCode};
use std::{env, fs};

use crate::check::{File, Selection};

#[cfg(test)]
mod tests;

/// The repository root, which is the parent of this package's directory.
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

/// Runs the checks that the command line selects. The exit code is 0 when the
/// repository passes, 1 when it has a violation or cannot be read, and 2 when
/// the command line is not one of the two accepted forms.
fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    #[expect(
        clippy::disallowed_methods,
        reason = "main owns standard error, where xtask prints usage, violations and failures"
    )]
    let mut stderr = io::stderr().lock();

    // A failed write to standard error has nowhere else to be reported, so
    // main ignores it and the exit code still gives the outcome.
    let Some(selection) = parse_args(&args) else {
        let _ = writeln!(stderr, "usage: cargo xtask check [docs]");
        return ExitCode::from(2);
    };
    match check_repository(selection, &mut stderr) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Returns the checks that `args` select, or `None` when `args`, which exclude
/// the program name, are not `check` or `check docs`.
fn parse_args(args: &[String]) -> Option<Selection> {
    match args {
        [command] if command == "check" => Some(Selection::All),
        [command, check] if command == "check" && check == "docs" => Some(Selection::Docs),
        _ => None,
    }
}

/// Runs the selected checks over the repository and writes each violation to
/// `out` as one line. Returns whether the repository passed.
fn check_repository(selection: Selection, out: &mut impl Write) -> io::Result<bool> {
    let files = read_files()?;
    let violations = check::run(selection, &files);
    for violation in &violations {
        writeln!(out, "{violation}")?;
    }
    Ok(violations.is_empty())
}

/// Lists the repository's files with `git ls-files`, including untracked files
/// that git does not ignore, and reads the Rust and Markdown files among them.
/// The checks read no other kind of file.
fn read_files() -> io::Result<Vec<File>> {
    #[expect(
        clippy::disallowed_methods,
        reason = "this module owns xtask's process effects and runs git to list the files"
    )]
    let mut git = Command::new("git");
    let output = git
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .current_dir(ROOT)
        .output()
        .map_err(|error| io::Error::other(format!("cannot run git: {error}")))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!(
            "git ls-files failed: {}",
            message.trim()
        )));
    }
    let listing = String::from_utf8(output.stdout).map_err(io::Error::other)?;

    let mut files = Vec::new();
    // With -z, git separates paths with a NUL byte and does not quote them.
    for path in listing.split('\0') {
        let extension = Path::new(path).extension().and_then(OsStr::to_str);
        if !matches!(extension, Some("rs" | "md")) {
            continue;
        }
        match fs::read_to_string(Path::new(ROOT).join(path)) {
            Ok(contents) => files.push(File {
                path: path.to_owned(),
                contents,
            }),
            // git still lists a tracked file that was deleted from the working
            // tree until the deletion is staged, and a deleted file has no
            // size to check.
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(io::Error::other(format!("cannot read {path}: {error}"))),
        }
    }
    Ok(files)
}
