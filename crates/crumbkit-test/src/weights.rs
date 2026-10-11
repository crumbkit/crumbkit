//! The path weight check, which keeps the bytes that an agent reads to find
//! and run each command within a budget, and the sums that it and
//! `cargo crumbkit weigh` share.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::Command;
use crumbkit::help::Guide;

use crate::guide::command_line;
use crate::pages::pages;
use crate::tree::{self, Visit};

#[cfg(test)]
mod tests;

/// crumbkit's default budget for a path, in bytes.
const PATH: u64 = 8_000;

/// The budgets of the paths through a tool's help, in bytes, which
/// [`check_path_weights`] checks, and the directory of the example golden
/// files that it reads the output weight from.
///
/// A path is what an agent reads to find and run a command: the `-h` page
/// of the root, of each command between the root and the command, and of
/// the command, and then the largest output of the command's examples.
/// [`PathBudgets::new`] holds crumbkit's default budget of 8,000 bytes for
/// every path, which [ADR-0025] records. A tool can lower the budget of
/// every path, or of single commands, and cannot raise a budget above the
/// default:
///
/// ```
/// use crumbkit_test::PathBudgets;
///
/// const BUDGETS: PathBudgets<'_> = PathBudgets::new("tests/golden/examples")
///     .path(4_000)
///     .nodes(&[("books list", 2_500)]);
/// ```
///
/// [ADR-0025]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0025-help-checks.md
#[derive(Clone, Copy, Debug)]
pub struct PathBudgets<'a> {
    /// The directory of the example golden files that
    /// [`check_examples`](crate::check_examples) writes.
    goldens: &'a str,
    /// The budget of each path.
    path: u64,
    /// The budgets of single commands, each named by its path below the
    /// root, which win over the budget of every path.
    nodes: &'a [(&'a str, u64)],
}

impl<'a> PathBudgets<'a> {
    /// Returns crumbkit's default budget for every path and no budget for a
    /// single command, with the example golden files in `goldens`, the
    /// directory that [`Examples`](crate::Examples) names.
    #[must_use]
    pub const fn new(goldens: &'a str) -> Self {
        Self {
            goldens,
            path: PATH,
            nodes: &[],
        }
    }

    /// Sets the budget of every path to `bytes`.
    #[must_use]
    pub const fn path(mut self, bytes: u64) -> Self {
        self.path = bytes;
        self
    }

    /// Sets the budgets of single commands, each a command's path below the
    /// root, with its names joined by single spaces, such as
    /// `"books list"`, and a number of bytes. A command's own budget wins
    /// over the budget of every path, and the first entry for a path wins
    /// over a later one.
    #[must_use]
    pub const fn nodes(mut self, nodes: &'a [(&'a str, u64)]) -> Self {
        self.nodes = nodes;
        self
    }
}

/// Checks that the path to `command` and to each visible command below it
/// fits its budget in `budgets`, and panics, as a failed test assertion
/// does, naming every problem it finds.
///
/// A path's weight is the sum of the sizes in bytes of the `-h` pages from
/// the root down to the command, as [`check_help_sizes`](crate::check_help_sizes)
/// reads them, and the command's output weight, which [`output_weight`]
/// reads from the golden files of the examples that `guide` gives the
/// command. A command without an example golden file has an output weight
/// of 0. `cargo crumbkit weigh` prints the same sums as `path_total`. The
/// check skips each hidden command and everything below it. It finds:
///
/// - a path whose weight is over its budget, naming the command, the
///   weight, the bytes of its pages and of its output, and the budget;
/// - a budget above crumbkit's default, because a tool can lower a budget
///   and cannot raise one;
/// - a command's budget whose path names no command or a hidden one;
/// - a golden file whose size cannot be read for a reason other than its
///   absence;
/// - a visible command whose `-h` prints no help page.
///
/// ```
/// use clap::Command;
/// use crumbkit::help::Guide;
/// use crumbkit_test::PathBudgets;
///
/// let shelf = Command::new("shelf")
///     .about("Browse a small catalogue")
///     .subcommand(Command::new("books").subcommand(Command::new("list")));
///
/// crumbkit_test::check_path_weights(&shelf, &Guide::new(), &PathBudgets::new("tests/golden/examples"));
/// ```
///
/// # Panics
///
/// Panics when a path, a budget or a golden file breaks a rule above. The
/// message lists each problem on its own line and names each command by its
/// path from `command`, such as `shelf books list`.
pub fn check_path_weights(command: &Command, guide: &Guide, budgets: &PathBudgets<'_>) {
    let mut problems = Vec::new();
    let above = |budget: u64| {
        format!(
            "budget of {budget} bytes is above crumbkit's default of {PATH} bytes, and a tool can lower a budget and cannot raise one"
        )
    };
    if budgets.path > PATH {
        problems.push(format!("the path {}", above(budgets.path)));
    }
    let pages = pages(command, guide, "-h", &mut problems);
    let visible = tree::walk_visible(command, &[], &mut problems);
    for &(path, budget) in budgets.nodes {
        match visible
            .iter()
            .find(|visit| visit.depth() > 0 && visit.below() == path)
        {
            Some(visit) if budget > PATH => {
                problems.push(format!("{}: the {}", visit.path(), above(budget)));
            }
            Some(_) => {}
            None => problems.push(format!(
                "the budget of `{path}` names no visible command below the root"
            )),
        }
    }
    let dir = Path::new(budgets.goldens);
    let rows: Vec<(usize, u64, Option<u64>)> = pages
        .iter()
        .map(|(visit, page)| {
            let output =
                output_weight(dir, &examples(command, guide, visit)).unwrap_or_else(|error| {
                    problems.push(format!("{}: {error}", visit.path()));
                    None
                });
            (visit.depth(), page.len() as u64, output)
        })
        .collect();
    for (((visit, _), &(_, _, output)), total) in pages.iter().zip(&rows).zip(path_totals(&rows)) {
        let own = budgets
            .nodes
            .iter()
            .find(|(path, _)| visit.depth() > 0 && *path == visit.below());
        let budget = own.map_or(budgets.path, |&(_, bytes)| bytes);
        if total > budget {
            let output = output.unwrap_or(0);
            problems.push(format!(
                "{}: the path reads {total} bytes, {} of -h pages and {output} of output, over its budget of {budget} bytes",
                visit.path(),
                total - output,
            ));
        }
    }
    tree::report("path weight rules", &problems);
}

/// Returns the command lines of the examples that `guide` gives the command
/// that `visit` reached in `command`, as help prints them.
fn examples(command: &Command, guide: &Guide, visit: &Visit<'_>) -> Vec<String> {
    let mut lines = Vec::new();
    for leaf in guide.get_leaves() {
        if leaf.path().join(" ") != visit.below() || visit.depth() == 0 {
            continue;
        }
        let text = std::iter::once(command.get_name())
            .chain(leaf.path().iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        lines.extend(
            leaf.get_examples()
                .iter()
                .map(|example| command_line(&text, example.words())),
        );
    }
    lines
}

/// Returns the path total of each of `rows`: the `-h` bytes of the row's
/// command and of each command above it, up to the root, plus the row's
/// output bytes.
///
/// Each row holds a command's depth below the root, which is 0 for the
/// root, the bytes of its `-h` page, and its output weight. The rows hold
/// one command each in depth-first order from the root, as
/// `commands list` lists them, so the commands above a row are the nearest
/// earlier row at each smaller depth. A sum that would overflow stays at
/// `u64::MAX`.
///
/// ```
/// let rows = [(0, 500, None), (1, 300, None), (2, 200, Some(150))];
///
/// assert_eq!(crumbkit_test::path_totals(&rows), [500, 800, 1_150]);
/// ```
#[must_use]
pub fn path_totals(rows: &[(usize, u64, Option<u64>)]) -> Vec<u64> {
    // `above[depth]` holds the sum of the `-h` bytes from the root down to
    // the latest row at that depth.
    let mut above: Vec<u64> = Vec::new();
    rows.iter()
        .map(|&(depth, short_help, output)| {
            above.truncate(depth);
            let pages = above
                .last()
                .copied()
                .unwrap_or(0)
                .saturating_add(short_help);
            above.push(pages);
            pages.saturating_add(output.unwrap_or(0))
        })
        .collect()
}

/// Returns the size in bytes of the largest golden file in `dir` of the
/// examples whose command lines are `examples`, or `None` when no example
/// has a golden file there.
///
/// Each example's golden file has the name that
/// [`example_golden_name`](crate::example_golden_name) gives its command
/// line, which is the name that [`check_examples`](crate::check_examples)
/// writes it under. A missing file counts as no golden file, so a tool
/// whose examples are not pinned has no output weight.
///
/// # Errors
///
/// Returns an [`UnreadableGolden`] when a golden file's size cannot be read
/// for a reason other than its absence, such as a `dir` that is a file.
pub fn output_weight(dir: &Path, examples: &[String]) -> Result<Option<u64>, UnreadableGolden> {
    let mut largest = None;
    for line in examples {
        let path = dir.join(crate::example_golden_name(line));
        match fs::metadata(&path) {
            // `None` is less than every `Some`, so the first size wins over
            // no size.
            Ok(metadata) => largest = largest.max(Some(metadata.len())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(UnreadableGolden { path, error }),
        }
    }
    Ok(largest)
}

/// The error of [`output_weight`] when a golden file's size cannot be read.
#[derive(Debug)]
pub struct UnreadableGolden {
    /// The golden file.
    path: PathBuf,
    /// The error that reading its size returned.
    error: io::Error,
}

impl UnreadableGolden {
    /// Returns the path of the golden file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the path of the golden file and the error that reading its
    /// size returned.
    #[must_use]
    pub fn into_parts(self) -> (PathBuf, io::Error) {
        (self.path, self.error)
    }
}

impl fmt::Display for UnreadableGolden {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the golden file {} cannot be read: {}",
            self.path.display(),
            self.error
        )
    }
}

impl std::error::Error for UnreadableGolden {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
