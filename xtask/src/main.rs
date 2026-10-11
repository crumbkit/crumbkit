//! `cargo xtask` runs crumbkit's repository checks.
//!
//! `cargo xtask check` runs every check, `cargo xtask check docs` runs only the
//! checks on `docs/`, and `cargo xtask ci-time` checks the median wall time of
//! recent pull-request CI runs. This module holds every effect: it reads the
//! command line, lists and reads the repository's files, runs `gh`, prints the
//! results and sets the exit code. The checks themselves are pure functions in
//! [`check`] and [`ci_time`].

mod check;
mod ci_time;

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, ExitCode};

use clap::{Parser, Subcommand};

use crate::check::{File, Selection};

#[cfg(test)]
mod tests;

/// The repository root, which is the parent of this package's directory.
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

/// Runs crumbkit's repository checks.
#[derive(Debug, PartialEq, Eq, Parser)]
#[command(bin_name = "cargo xtask")]
enum Cli {
    /// Runs every repository check, or only the checks on docs/.
    Check {
        #[command(subcommand)]
        scope: Option<Scope>,
    },
    /// Fails when the median wall time of the last 20 successful pull-request
    /// CI runs is over 300 seconds.
    CiTime,
}

/// A part of the repository checks that `check` can run on its own.
#[derive(Debug, PartialEq, Eq, Subcommand)]
enum Scope {
    /// Runs only the checks on docs/: the directory word check and the
    /// decision record checks.
    Docs,
}

/// Runs the check that the command line selects. The exit code is 0 when the
/// check passes, 1 when it fails or cannot run, and 2 when clap refuses the
/// command line.
fn main() -> ExitCode {
    let cli = Cli::parse();

    #[expect(
        clippy::disallowed_methods,
        reason = "main owns standard error, where xtask prints results and failures"
    )]
    let mut stderr = io::stderr().lock();

    let outcome = match cli {
        Cli::Check { scope: None } => check_repository(Selection::All, &mut stderr),
        Cli::Check {
            scope: Some(Scope::Docs),
        } => check_repository(Selection::Docs, &mut stderr),
        Cli::CiTime => check_ci_time(&mut stderr),
    };
    match outcome {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            // A failed write to standard error has nowhere else to be
            // reported, so main ignores it and the exit code still gives the
            // outcome.
            let _ = writeln!(stderr, "error: {error}");
            ExitCode::FAILURE
        }
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

/// Measures the recent pull-request runs of the CI workflow and writes the
/// report to `out` as one line. Returns whether the median passes.
fn check_ci_time(out: &mut impl Write) -> io::Result<bool> {
    let report = ci_time::measure(&run_seconds()?);
    writeln!(out, "{report}")?;
    Ok(report.passes())
}

/// Returns the wall time in seconds of each of the last 20 successful runs of
/// the CI workflow that GitHub started for `pull_request` events.
///
/// `gh api` fetches the runs of the repository in the current directory, and
/// its built-in jq filter prints one number of seconds per run, so xtask
/// parses no JSON or dates. A run object has no completion time, so the filter
/// uses `updated_at`, which for a completed run is the time it finished.
fn run_seconds() -> io::Result<Vec<u64>> {
    let endpoint = format!(
        "repos/{{owner}}/{{repo}}/actions/workflows/{}/runs?event=pull_request&status=success&per_page={}",
        ci_time::WORKFLOW_FILE,
        ci_time::RUNS
    );
    #[expect(
        clippy::disallowed_methods,
        reason = "this module owns xtask's process effects and runs gh to read the CI runs"
    )]
    let mut gh = Command::new("gh");
    let output = gh
        .args([
            "api",
            &endpoint,
            "--jq",
            ".workflow_runs[] | (.updated_at | fromdate) - (.run_started_at | fromdate)",
        ])
        .current_dir(ROOT)
        .output()
        .map_err(|error| io::Error::other(format!("cannot run gh: {error}")))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!(
            "gh api failed: {}",
            message.trim()
        )));
    }
    let listing = String::from_utf8(output.stdout).map_err(io::Error::other)?;
    listing
        .lines()
        .map(|line| {
            line.parse().map_err(|_| {
                io::Error::other(format!(
                    "gh printed {line:?}, which is not a whole number of seconds"
                ))
            })
        })
        .collect()
}

/// Lists the repository's files with `git ls-files`, including untracked files
/// that git does not ignore, and reads every one of them. The citation check
/// reads every text file, and the listing check needs every path, so that a
/// node record may link to any file in the repository.
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
    for path in listing.split('\0').filter(|path| !path.is_empty()) {
        match fs::read(Path::new(ROOT).join(path)) {
            Ok(bytes) => files.push(File {
                path: path.to_owned(),
                // A file that is not UTF-8 text, such as an image, keeps its
                // path so that a link to it resolves, and has no contents,
                // because no check reads binary data.
                contents: String::from_utf8(bytes).unwrap_or_default(),
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
