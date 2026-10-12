//! The opt-out report, which pins a tool's opt-outs from crumbkit's
//! conventions, with their reasons and costs, in golden files.

use std::fmt::Write as _;
use std::path::Path;

use clap::Command;
use serde::Serialize;
use snapbox::Assert;
use snapbox::assert::DEFAULT_ACTION_ENV;

use crate::allow::{Allow, command_line};
use crate::goldens::compare;
use crate::tree;

#[cfg(test)]
mod tests;

/// The id of the JSON report's schema, which a reader checks before it
/// reads the entries.
const SCHEMA: &str = "crumbkit/opt-outs/v1";

/// Checks that the report of the tool's opt-outs from crumbkit's
/// conventions matches the golden files `opt-outs.txt` and
/// `opt-outs.json` in `dir`, and panics, as a failed test assertion does,
/// naming every problem it finds.
///
/// `command` is the tool's root command and `allow` holds every opt-out
/// that the tool passes to the checks ([ADR-0036]). The report lists each
/// opt-out in the order of `allow`, with the rule's id, the command line it
/// applies to, the tool's reason and what skipping the rule costs an agent.
/// A reviewer reads a change to a tool's opt-outs as a change to these two
/// files, and an agent reads `opt-outs.json` to learn where the tool
/// departs from crumbkit's conventions.
///
/// The text file starts with a line that counts the opt-outs:
///
/// ```text
/// shelf has 1 opt-out from crumbkit's conventions.
///
/// grammar/root-leaf: shelf version
///   Reason: People type `shelf version` from habit.
///   Cost: Each such leaf adds a line to the root help, which an agent reads first.
/// ```
///
/// The JSON file holds the same entries under `data`, with the schema id
/// `crumbkit/opt-outs/v1` under `schema`. Each entry has the fields `rule`,
/// `applies_to`, `reason` and `costs`.
///
/// The check compares each file with snapbox, as other golden files are
/// compared (ADR-0004). With the environment variable `SNAPSHOTS=overwrite`,
/// it writes each missing file and rewrites each file that differs. It
/// leaves every other file in `dir` alone, so `dir` can be a directory of
/// other golden files, such as `tests/golden`. A relative `dir` names a
/// directory in the package that runs the test.
///
/// ```no_run
/// use clap::Command;
/// use crumbkit_test::{Allow, Rule};
///
/// const ALLOW: &[Allow] = &[Allow::new(
///     Rule::RootLeaf,
///     "version",
///     "People type `shelf version` from habit.",
/// )];
///
/// let shelf = Command::new("shelf").subcommand(Command::new("version"));
///
/// crumbkit_test::check_opt_outs(&shelf, ALLOW, "tests/golden");
/// ```
///
/// # Panics
///
/// Panics when a golden file is missing or differs from the report, or
/// when `SNAPSHOTS=overwrite` cannot write a file. Each problem in the
/// message starts on a new line.
///
/// [ADR-0036]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0036-opt-outs.md
pub fn check_opt_outs(command: &Command, allow: &[Allow], dir: impl AsRef<Path>) {
    let assert = Assert::new().action_env(DEFAULT_ACTION_ENV);
    check(command, allow, dir.as_ref(), &assert);
}

/// Runs [`check_opt_outs`] with `assert`, whose action decides whether the
/// check compares or overwrites, so a test chooses the action without
/// setting `SNAPSHOTS`.
fn check(command: &Command, allow: &[Allow], dir: &Path, assert: &Assert) {
    let entries: Vec<Entry> = allow
        .iter()
        .map(|allow| Entry {
            rule: allow.rule().id(),
            applies_to: command_line(command, allow.path()),
            reason: allow.reason(),
            costs: allow.rule().costs(),
        })
        .collect();
    let line = command.get_name();
    let mut problems = Vec::new();
    compare(
        assert,
        line,
        "opt-out report",
        text(line, &entries),
        &dir.join("opt-outs.txt"),
        &mut problems,
    );
    compare(
        assert,
        line,
        "opt-out report",
        json(&entries),
        &dir.join("opt-outs.json"),
        &mut problems,
    );
    tree::report("opt-out report rules", &problems);
}

/// One opt-out as the report shows it.
#[derive(Serialize)]
struct Entry {
    /// The rule's id, such as `grammar/root-leaf`.
    rule: &'static str,
    /// The command line that the opt-out covers, such as `shelf version`.
    applies_to: String,
    /// The tool's reason.
    reason: &'static str,
    /// What skipping the rule costs an agent, one sentence each.
    costs: &'static [&'static str],
}

/// The JSON report: the schema id and the entries.
#[derive(Serialize)]
struct Report<'a> {
    /// The id of the report's schema.
    schema: &'static str,
    /// The opt-outs, in the tool's order.
    data: &'a [Entry],
}

/// Returns the text report of `entries` for the tool `tool`.
fn text(tool: &str, entries: &[Entry]) -> String {
    let mut text = match entries.len() {
        0 => format!("{tool} has no opt-outs from crumbkit's conventions.\n"),
        1 => format!("{tool} has 1 opt-out from crumbkit's conventions.\n"),
        count => format!("{tool} has {count} opt-outs from crumbkit's conventions.\n"),
    };
    for entry in entries {
        // Writing to a `String` cannot fail.
        let _ = write!(
            text,
            "\n{}: {}\n  Reason: {}\n",
            entry.rule, entry.applies_to, entry.reason
        );
        for cost in entry.costs {
            let _ = writeln!(text, "  Cost: {cost}");
        }
    }
    text
}

/// Returns the JSON report of `entries`, pretty-printed and ending with a
/// line break.
fn json(entries: &[Entry]) -> String {
    let report = Report {
        schema: SCHEMA,
        data: entries,
    };
    // Serializing strings and lists of strings cannot fail, and an empty
    // text would show as a differing file.
    serde_json::to_string_pretty(&report).unwrap_or_default() + "\n"
}
