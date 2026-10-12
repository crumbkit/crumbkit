//! The help golden check, which pins each command's `-h` and `--help` pages
//! in golden files.

use std::fs;
use std::path::Path;

use clap::Command;
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
/// path and the flag with the guide of `pages`, as a caller reads it on a
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
/// use crumbkit::help::Guide;
/// use crumbkit_test::Pages;
///
/// let shelf = Command::new("shelf")
///     .about("Browse a small catalogue")
///     .subcommand(Command::new("books").subcommand(Command::new("list")));
///
/// crumbkit_test::check_help_goldens(&shelf, Pages::rendered(&Guide::new()), "tests/golden/help");
/// ```
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
/// instead.
pub(crate) fn compare(
    assert: &Assert,
    line: &str,
    what: &str,
    actual: String,
    file: &Path,
    problems: &mut Vec<String>,
) {
    if assert.selected_action() != Action::Overwrite && !file.is_file() {
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
