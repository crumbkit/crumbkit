//! The opt-out report, which pins a tool's opt-outs from crumbkit's
//! conventions, with their reasons and costs, in golden files.

use std::fmt::Write as _;
use std::path::Path;

use clap::Command;
use serde::Serialize;
use snapbox::Assert;
use snapbox::assert::DEFAULT_ACTION_ENV;

use crate::allow::{Allow, command_line};
use crate::budget::Departure;
use crate::goldens::compare;
use crate::{Budgets, PathBudgets, tree};

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
/// `command` is the tool's root command, `allow` holds every opt-out that
/// the tool passes to the checks ([ADR-0036]), `budgets` holds the budgets
/// that it passes to [`check_help_sizes`](crate::check_help_sizes), and
/// `paths` holds the budgets that it passes to
/// [`check_path_weights`](crate::check_path_weights), or is `None` for a
/// tool that does not run that check. The report lists each opt-out in the
/// order of `allow`, with the rule's id, the command line it applies to,
/// the tool's reason and what skipping the rule costs an agent. Then it
/// lists each budget that the tool set, with the budget, crumbkit's default
/// and, for a budget above the default, the extra bytes and tokens at 4
/// bytes a token that each read can cost.
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
/// `applies_to`, `reason`, `costs` and `budget`, which is `null` for an
/// opt-out and holds `bytes`, `default`, `extra_bytes` and `extra_tokens`
/// for a budget.
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
/// use crumbkit_test::{Allow, Budgets, Rule};
///
/// const ALLOW: &[Allow] = &[Allow::new(
///     Rule::RootLeaf,
///     "version",
///     "People type `shelf version` from habit.",
/// )];
///
/// let shelf = Command::new("shelf").subcommand(Command::new("version"));
///
/// crumbkit_test::check_opt_outs(&shelf, ALLOW, &Budgets::new(), None, "tests/golden");
/// ```
///
/// # Panics
///
/// Panics when a golden file is missing or differs from the report, when a
/// command's budget names no visible command below the root, or when
/// `SNAPSHOTS=overwrite` cannot write a file. Each problem in the
/// message starts on a new line.
///
/// [ADR-0036]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0036-opt-outs.md
pub fn check_opt_outs(
    command: &Command,
    allow: &[Allow],
    budgets: &Budgets,
    paths: Option<&PathBudgets<'_>>,
    dir: impl AsRef<Path>,
) {
    let assert = Assert::new().action_env(DEFAULT_ACTION_ENV);
    check(command, allow, budgets, paths, dir.as_ref(), &assert);
}

/// Runs [`check_opt_outs`] with `assert`, whose action decides whether the
/// check compares or overwrites, so a test chooses the action without
/// setting `SNAPSHOTS`.
fn check(
    command: &Command,
    allow: &[Allow],
    budgets: &Budgets,
    paths: Option<&PathBudgets<'_>>,
    dir: &Path,
    assert: &Assert,
) {
    let mut problems = Vec::new();
    let mut entries: Vec<Entry> = allow
        .iter()
        .map(|allow| Entry {
            rule: allow.rule().id(),
            applies_to: command_line(command, allow.path()),
            reason: allow.reason(),
            costs: allow
                .rule()
                .costs()
                .iter()
                .map(|&cost| cost.to_owned())
                .collect(),
            budget: None,
        })
        .collect();
    let help = budgets.departures(command, &mut problems);
    entries.extend(help.into_iter().map(|departure| {
        let cost = format!(
            "Each read of the page can cost {} bytes more than crumbkit's default, about {} tokens.",
            departure.extra_bytes(),
            departure.extra_tokens()
        );
        Entry::of(departure, cost)
    }));
    let path = paths.map(|paths| paths.departures(command, &mut problems));
    entries.extend(path.into_iter().flatten().map(|departure| {
        let cost = format!(
            "An agent that finds and runs the command can read {} bytes more than crumbkit's default, about {} tokens.",
            departure.extra_bytes(),
            departure.extra_tokens()
        );
        Entry::of(departure, cost)
    }));
    let line = command.get_name();
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
    costs: Vec<String>,
    /// The budget that the entry sets, or `None` for an opt-out.
    budget: Option<BudgetEntry>,
}

impl Entry {
    /// Returns the entry of `departure`, whose cost is `raised` when the
    /// budget is above crumbkit's default.
    fn of(departure: Departure, raised: String) -> Self {
        let cost = if departure.extra_bytes() > 0 {
            raised
        } else {
            "None, because the budget is at or below crumbkit's default.".to_owned()
        };
        Self {
            rule: departure.rule,
            budget: Some(BudgetEntry {
                bytes: departure.bytes,
                default: departure.default,
                extra_bytes: departure.extra_bytes(),
                extra_tokens: departure.extra_tokens(),
            }),
            applies_to: departure.applies_to,
            reason: departure.reason,
            costs: vec![cost],
        }
    }
}

/// A budget as the report shows it.
#[derive(Serialize)]
struct BudgetEntry {
    /// The budget, in bytes.
    bytes: u64,
    /// crumbkit's default for the same pages or paths, in bytes.
    default: u64,
    /// The bytes by which the budget is above the default, or 0.
    extra_bytes: u64,
    /// The extra bytes in tokens at 4 bytes a token, rounded up.
    extra_tokens: u64,
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
        let _ = writeln!(text, "\n{}: {}", entry.rule, entry.applies_to);
        if let Some(budget) = &entry.budget {
            let _ = writeln!(
                text,
                "  Budget: {} bytes, where crumbkit's default is {} bytes.",
                budget.bytes, budget.default
            );
        }
        let _ = writeln!(text, "  Reason: {}", entry.reason);
        for cost in &entry.costs {
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
