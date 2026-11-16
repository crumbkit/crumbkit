//! The help golden check, which pins each command's `-h` and `--help` pages
//! in golden files.

use std::fs;
use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use clap::Command;
use crumbkit_durable::{Mode, PathLock, write_atomically};
use snapbox::assert::{Action, DEFAULT_ACTION_ENV};
use snapbox::{Assert, Data};

use crate::pages::Pages;
use crate::tree;

#[cfg(test)]
mod tests;

/// Checks that the `-h` and `--help` pages of `command` and of each visible
/// command below it match the golden files in `dir`, and panics, as a
/// failed test assertion does, naming every problem it finds.
///
/// A page is the plain text that `crumbkit::parse` prints for the command's
/// path and the flag with the tool record of `pages`, as a caller reads it on a
/// pipe, or what the tool's binary prints, as [`Pages`] chooses. The check
/// skips each hidden command and everything below it, because help never
/// shows them. The golden file of a page is named after the command's path
/// and the flag: `shelf-h.txt` and `shelf-help.txt` for the root
/// `shelf`, and `shelf-books-list-h.txt` and `shelf-books-list-help.txt`
/// for `shelf books list`.
///
/// `dir` holds only help golden files, because the check counts every
/// `.txt` file in it that no page produces as stale. A relative `dir`, such
/// as `tests/golden/help`, names a directory in the package that runs the
/// test, because cargo runs a package's unit and integration tests from the
/// package's manifest directory.
///
/// The check compares each page with snapbox, as other golden files are
/// compared (ADR-0004). With the environment variable `SNAPSHOTS=overwrite`,
/// it writes each missing file, rewrites each file whose content differs
/// and deletes each stale file. It finds two pages whose golden files have
/// one name, such as the pages of `shelf books list` and of a root command
/// `books-list`, naming both command lines, and compares or writes only the
/// first of the two. It finds a visible command whose `-h` or `--help`
/// prints no help page, such as one that turned its help flag off. Without
/// `SNAPSHOTS=overwrite`, it also finds:
///
/// - a page without a golden file;
/// - a page that differs from its golden file, with snapbox's report of
///   both texts;
/// - a stale golden file: a `.txt` file in `dir` that no page produces, such
///   as the page of a removed command.
///
/// ```no_run
/// use clap::Command;
/// use crumbkit::Tool;
/// use crumbkit_test::Pages;
///
/// let shelf = Command::new("shelf")
///     .about("Browse a small catalogue")
///     .subcommand(Command::new("books").subcommand(Command::new("list")));
///
/// crumbkit_test::check_help_goldens(&shelf, Pages::rendered(&Tool::new()), "tests/golden/help");
/// ```
///
/// # Parallel test processes
///
/// With `SNAPSHOTS=overwrite` the check is safe in test processes that run
/// at once, as cargo-nextest runs them. It writes each file through a
/// temporary file and a rename, so a reader sees the old file or the new
/// one in full, and it locks `dir` until it finishes, so two overwrites of
/// one directory run one after the other. The lock waits up to 120 seconds
/// and then reports a problem. Two different checks still need two
/// directories (ADR-0231).
///
/// # Panics
///
/// Panics when a page or a golden file breaks a rule above, or when
/// `SNAPSHOTS=overwrite` cannot write or delete a file. Each problem in the
/// message starts on a new line, and the problem of a page that differs
/// from its golden file continues with snapbox's report of both texts. The
/// message names each page by its command line, such as
/// `shelf books list -h`.
pub fn check_help_goldens(command: &Command, pages: Pages<'_>, dir: impl AsRef<Path>) {
    let assert = Assert::new().action_env(DEFAULT_ACTION_ENV);
    check(command, pages, dir.as_ref(), &assert);
}

/// Runs [`check_help_goldens`] with `assert`, whose action decides whether
/// the check compares or overwrites, so a test chooses the action without
/// setting `SNAPSHOTS`.
fn check(command: &Command, pages: Pages<'_>, dir: &Path, assert: &Assert) {
    let overwrite = assert.selected_action() == Action::Overwrite;
    let mut problems = Vec::new();
    let _lock = lock_directory(dir, overwrite, &mut problems);
    // Each golden file name with the command line of the page that has it.
    let mut names: Vec<(String, String)> = Vec::new();
    for (flag, suffix) in [("-h", "h"), ("--help", "help")] {
        for (visit, page) in crate::pages::pages(command, pages, flag, &mut problems) {
            let line = format!("{} {flag}", visit.path());
            let name = format!("{}-{suffix}.txt", visit.path().replace(' ', "-"));
            if let Some((_, other)) = names.iter().find(|(known, _)| *known == name) {
                problems.push(format!(
                    "{line}: the page's golden file name `{name}` is also the name for the page `{other}`"
                ));
                continue;
            }
            compare(assert, &line, "page", page, &dir.join(&name), &mut problems);
            names.push((name, line));
        }
    }
    let names: Vec<String> = names.into_iter().map(|(name, _)| name).collect();
    remove_stale(dir, &names, "txt", overwrite, "help page", &mut problems);
    tree::report("golden help rules", &problems);
}

/// Compares `actual`, the `what` of the command line `line`, such as its
/// page or its output, with the golden file `file` through `assert`, and
/// adds a problem to `problems` when the file is missing or differs. When
/// the action of `assert` is to overwrite, it writes or rewrites the file
/// instead, through [`write_golden`].
///
/// An overwrite compares the bytes itself and never lets snapbox write,
/// because snapbox truncates the file and then writes it, so a test process
/// that reads the file in between sees an empty or partial file.
pub(crate) fn compare(
    assert: &Assert,
    line: &str,
    what: &str,
    actual: String,
    file: &Path,
    problems: &mut Vec<String>,
) {
    if assert.selected_action() == Action::Overwrite {
        if fs::read(file).is_ok_and(|current| current == actual.as_bytes()) {
            return;
        }
        if let Err(problem) = write_golden(file, actual.as_bytes()) {
            problems.push(format!("{line}: {problem}"));
        }
        return;
    }
    if !file.is_file() {
        problems.push(format!(
            "{line}: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it",
            file.display()
        ));
        return;
    }
    let expected = Data::read_from(file, None).raw();
    if let Err(error) = assert.try_eq(Some(&line), Data::text(actual), expected) {
        problems.push(format!(
            "{line}: the {what} differs from the golden file `{}`:\n{error}",
            file.display()
        ));
    }
}

/// Writes `bytes` to the golden file `file`, and creates its directory when
/// it is missing. Every check that writes a golden file calls this, so a
/// reader of the file, such as a check in another test process, sees the old
/// file or the new one in full.
///
/// The write goes through a temporary file in the same directory, named
/// `.<file>.<pid>.<n>.tmp`, and a rename over `file`
/// ([`crumbkit_durable::write_atomically`]). The temporary name ends in
/// `tmp`, which no check treats as a golden file extension, so a stale scan
/// never deletes it or counts it.
pub(crate) fn write_golden(file: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = file.parent() {
        #[expect(
            clippy::disallowed_methods,
            reason = "the golden check owns its directory and creates it before the first golden file"
        )]
        let created = fs::create_dir_all(parent);
        created.map_err(|error| {
            format!(
                "the golden directory `{}` cannot be created: {error}",
                parent.display()
            )
        })?;
    }
    write_atomically(file, bytes, Mode::Umask).map_err(|error| {
        format!(
            "the golden file `{}` cannot be written: {error}: {}",
            file.display(),
            error.source
        )
    })
}

/// How long [`lock_directory`] waits for another check of the same
/// directory before it reports a problem.
const LOCK_WAIT: Duration = Duration::from_secs(120);

/// With `overwrite` set, locks `dir` for the caller's whole check and
/// returns the lock, which the caller keeps until the check ends, so
/// another test process that overwrites the same directory waits for this
/// one. Without `overwrite` it returns `None`, because a comparison only
/// reads. When the lock cannot be taken within [`LOCK_WAIT`], it adds a
/// problem to `problems` and returns `None`.
///
/// A check that deletes stale golden files decides what is stale from the
/// files that exist when it scans, and from the golden files that a skipped
/// example keeps. A second overwrite that writes a golden file between
/// those two reads would lose it to the scan. The lock makes the two
/// overwrites run one after the other, and each sees the files of the first.
///
/// The lock file is in the system's temporary directory, named by a hash of
/// the absolute path of `dir`, so the golden directory holds no extra file.
pub(crate) fn lock_directory(
    dir: &Path,
    overwrite: bool,
    problems: &mut Vec<String>,
) -> Option<PathLock> {
    if !overwrite {
        return None;
    }
    let file = lock_file(dir);
    match PathLock::acquire(&file, Instant::now() + LOCK_WAIT) {
        Ok(lock) => Some(lock),
        Err(error) => {
            problems.push(format!(
                "the golden directory `{}` cannot be locked for `SNAPSHOTS=overwrite`: {error}",
                dir.display()
            ));
            None
        }
    }
}

/// Returns the path of the lock file for the golden directory `dir`.
fn lock_file(dir: &Path) -> PathBuf {
    let absolute = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    let mut hasher = DefaultHasher::new();
    absolute.hash(&mut hasher);
    std::env::temp_dir().join(format!("crumbkit-test-{:016x}.lock", hasher.finish()))
}

/// Adds a problem to `problems` for each file in `dir` with the extension
/// `extension`, such as `txt`, whose name is not in `names`, saying that it
/// belongs to no `owner`, such as a help page, or deletes the file when
/// `overwrite` is set.
///
/// A directory that does not exist or cannot be read holds no stale file,
/// and each name then reports its own missing file.
pub(crate) fn remove_stale(
    dir: &Path,
    names: &[String],
    extension: &str,
    overwrite: bool,
    owner: &str,
    problems: &mut Vec<String>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut stale: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|found| found == extension))
        .filter(|path| {
            path.file_name()
                .is_none_or(|name| !names.iter().any(|known| name == known.as_str()))
        })
        .collect();
    stale.sort();
    for file in stale {
        if !overwrite {
            problems.push(format!(
                "the golden file `{}` belongs to no {owner}, and `SNAPSHOTS=overwrite` deletes it",
                file.display()
            ));
            continue;
        }
        #[expect(
            clippy::disallowed_methods,
            reason = "the golden check owns its directory, and SNAPSHOTS=overwrite deletes each stale golden file there"
        )]
        let removed = fs::remove_file(&file);
        if let Err(error) = removed {
            problems.push(format!(
                "the stale golden file `{}` cannot be deleted: {error}",
                file.display()
            ));
        }
    }
}
