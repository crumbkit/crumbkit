//! The opt-out report, which pins a tool's opt-outs from crumbkit's
//! conventions, with their reasons and costs, in golden files.

use std::fmt::Write as _;
use std::path::Path;

use clap::Command;
use crumbkit::help::Guide;
use crumbkit::{JsonLayout, Output};
use serde::Serialize;
use snapbox::Assert;
use snapbox::assert::DEFAULT_ACTION_ENV;

use crate::allow::Allow;
use crate::budget::Departure;
use crate::goldens::compare;
use crate::{Budgets, PathBudgets, tree};

mod customisations;
#[cfg(test)]
mod tests;

use self::customisations::{Customisation, customisations};

/// The id of the JSON report's schema, which a reader checks before it
/// reads the entries. The report names the tool in place of `crumbkit`,
/// such as `shelf/opt-outs/v1` (ADR-0079).
const SCHEMA: &str = "crumbkit/opt-outs/v1";

/// Checks that the report of the tool's opt-outs from crumbkit's
/// conventions matches the golden files `opt-outs.txt` and
/// `opt-outs.json` in `dir`, and panics, as a failed test assertion does,
/// naming every problem it finds.
///
/// `command` is the tool's root command, `guide` is the tool's guide, whose
/// output budget and JSON layout the report lists, `allow` holds every opt-out that
/// the tool passes to the checks ([opt-outs docs]), `budgets` holds the budgets
/// that it passes to [`check_help_sizes`](crate::check_help_sizes), and
/// `paths` holds the budgets that it passes to
/// [`check_path_weights`](crate::check_path_weights), or is `None` for a
/// tool that does not run that check. The report lists each opt-out in the
/// order of `allow`, with the rule's id, the command line it applies to,
/// the tool's reason and what skipping the rule costs an agent. Then it
/// lists each budget that the tool set, with the budget, crumbkit's default
/// and, for a budget above the default, the extra bytes and tokens at 4
/// bytes a token that each read can cost. When the guide sets an output
/// budget, the report ends with it beside crumbkit's default of 1048576
/// bytes, under the rule `output-budget/run` (ADR-0046). No opt-out removes
/// the output budget. When the guide sets a
/// [`JsonLayout`](crumbkit::JsonLayout), the report then lists it under the
/// rule `json-layout/pretty`, with its reason and its cost (ADR-0052).
/// When the guide turns terminal detection off with
/// `Guide::without_terminal_detection`, the report then lists it under the
/// rule `terminal-detection/off`, with its reason and its costs (ADR-0049).
/// After the opt-outs, the report lists the tool's customisations: the
/// declarations that change what a run prints and keep crumbkit's
/// behaviour, so they have no reason and no cost (ADR-0056). They are each
/// code rename, each exit that lists one of
/// crumbkit's codes or its rename, each document whose name is not its
/// command's path, each leaf that passes on a program's exit code, each
/// leaf that declares refusal alternatives or a refusal hint, the guide's
/// refusal alternatives and refusal hint, the guide's
/// `Guide::dashed_forms` declaration, and with it each command named in
/// another style that also answers to its dashed form, such as
/// `reading_lists` for `reading-lists`, and the root verbs that the tool
/// declares on the guide or with [`Allow::root_verbs`], with their reason.
/// The report spells codes with colons. The section is left out when the
/// tool declares nothing.
/// A reviewer reads a change to a tool's opt-outs as a change to these two
/// files, and an agent reads `opt-outs.json` to learn where the tool
/// departs from crumbkit's conventions.
///
/// The text file starts with a line that counts the opt-outs and, when the
/// tool sets any, the budgets apart from them, such as
/// `shelf has 1 opt-out from the conventions and sets 2 budgets.`:
///
/// ```text
/// shelf has 1 opt-out from the conventions.
///
/// grammar/root-leaf: shelf version
///   Reason: People type `shelf version` from habit.
///   Cost: Each such leaf adds a line to the root help, which an agent reads first.
/// ```
///
/// The JSON file holds the same entries under `data`, with the schema id
/// `<tool>/opt-outs/v1` under `schema`. Each entry has the fields `rule`,
/// `applies_to`, `reason`, `costs` and `budget`, which is `null` for an
/// opt-out and holds `bytes`, `default`, `extra_bytes` and `extra_tokens`
/// for a budget. When the tool declares a customisation, the JSON file
/// also holds a `customisations` array, whose entries have the fields
/// `declaration`, `applies_to` and `declares`, and `reason` for the root
/// verbs. The text line that counts the customisations says that they need
/// no reason unless the tool declares root verbs.
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
/// use crumbkit::help::Guide;
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
/// crumbkit_test::check_opt_outs(&shelf, &Guide::new(), ALLOW, &Budgets::new(), None, "tests/golden");
/// ```
///
/// # Panics
///
/// Panics when a golden file is missing or differs from the report, when a
/// command's budget names no visible command below the root, or when
/// `SNAPSHOTS=overwrite` cannot write a file. Each problem in the
/// message starts on a new line.
///
/// [opt-outs docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/adoption/opt-outs.md
pub fn check_opt_outs(
    command: &Command,
    guide: &Guide,
    allow: &[Allow],
    budgets: &Budgets,
    paths: Option<&PathBudgets<'_>>,
    dir: impl AsRef<Path>,
) {
    let assert = Assert::new().action_env(DEFAULT_ACTION_ENV);
    check(command, guide, allow, budgets, paths, dir.as_ref(), &assert);
}

/// Runs [`check_opt_outs`] with `assert`, whose action decides whether the
/// check compares or overwrites, so a test chooses the action without
/// setting `SNAPSHOTS`.
fn check(
    command: &Command,
    guide: &Guide,
    allow: &[Allow],
    budgets: &Budgets,
    paths: Option<&PathBudgets<'_>>,
    dir: &Path,
    assert: &Assert,
) {
    let mut problems = Vec::new();
    let mut entries: Vec<Entry> = allow
        .iter()
        .filter(|allow| allow.declared_root_verbs().is_none())
        .map(|allow| Entry {
            rule: allow.rule().id(),
            applies_to: allow.line(command),
            reason: allow.reason(),
            costs: allow.costs().iter().map(|&cost| cost.to_owned()).collect(),
            budget: None,
        })
        .collect();
    let help = budgets.departures(command, &mut problems);
    entries.extend(help.into_iter().map(|departure| {
        let cost = format!(
            "Each read of the page can cost {} bytes more than the default, about {} tokens.",
            departure.extra_bytes(),
            departure.extra_tokens()
        );
        Entry::of(departure, cost)
    }));
    let path = paths.map(|paths| paths.departures(command, &mut problems));
    entries.extend(path.into_iter().flatten().map(|departure| {
        let cost = format!(
            "An agent that finds and runs the command can read {} bytes more than the default, about {} tokens.",
            departure.extra_bytes(),
            departure.extra_tokens()
        );
        Entry::of(departure, cost)
    }));
    if let Some(departure) = output_budget(command, guide) {
        let cost = format!(
            "A run can print {} bytes more than the default, about {} tokens.",
            departure.extra_bytes(),
            departure.extra_tokens()
        );
        entries.push(Entry::of(departure, cost));
    }
    if let JsonLayout::Pretty { reason } = guide.get_json_layout() {
        entries.push(Entry {
            rule: "json-layout/pretty",
            applies_to: format!("every run of {}", command.get_name()),
            reason,
            costs: vec![JSON_LAYOUT_COST.to_owned()],
            budget: None,
        });
    }
    if let Some(reason) = guide.get_without_terminal_detection() {
        entries.push(Entry {
            rule: "terminal-detection/off",
            applies_to: format!("every run of {}", command.get_name()),
            reason,
            costs: TERMINAL_DETECTION_COSTS
                .iter()
                .map(|&cost| cost.to_owned())
                .collect(),
            budget: None,
        });
    }
    let declared = customisations(command, guide, allow);
    let line = command.get_name();
    compare(
        assert,
        line,
        "opt-out report",
        text(line, &entries, &declared),
        &dir.join("opt-outs.txt"),
        &mut problems,
    );
    compare(
        assert,
        line,
        "opt-out report",
        json(
            &guide.tool_name(command).schema_id(SCHEMA),
            &entries,
            &declared,
        ),
        &dir.join("opt-outs.json"),
        &mut problems,
    );
    tree::report("opt-out report rules", &problems);
}

/// What an indented JSON layout costs an agent, which the report lists
/// under the rule `json-layout/pretty`.
const JSON_LAYOUT_COST: &str = "Every JSON document that is not a stream line is indented by two spaces, which made the documents of the example tool shelf 20 to 62 percent larger, so an agent reads more tokens for each result and fewer rows fit in the output budget.";

/// What turning terminal detection off costs, which the report lists
/// under the rule `terminal-detection/off`.
const TERMINAL_DETECTION_COSTS: &[&str] = &[
    "A picker never asks for a missing argument and `Output::confirm` never asks, even at a terminal, so a person must type every argument and `--yes` as a script does.",
    "Progress never prints, help prints without clap's colours at a terminal, and a table's cells print without their styles.",
    "A text table never fits the terminal's width, so the terminal wraps each line of a table that is wider than the terminal.",
];

/// Returns the output budget that `guide` sets as the report lists it, or
/// `None` when the tool keeps crumbkit's default.
fn output_budget(command: &Command, guide: &Guide) -> Option<Departure> {
    let (bytes, reason) = guide.get_output_budget()?;
    Some(Departure {
        rule: "output-budget/run",
        applies_to: format!("every run of {}", command.get_name()),
        reason,
        bytes: u64::try_from(bytes).unwrap_or(u64::MAX),
        default: u64::try_from(Output::DEFAULT_BUDGET).unwrap_or(u64::MAX),
    })
}

/// One entry of the report as it shows it: an opt-out, or a budget that the
/// tool set.
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
            "None, because the budget is at or below the default.".to_owned()
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

/// The JSON report: the schema id, the entries and the declarations.
#[derive(Serialize)]
struct Report<'a> {
    /// The id of the report's schema, such as `shelf/opt-outs/v1`.
    schema: &'a str,
    /// The opt-outs and the budgets, in the order the report lists them.
    data: &'a [Entry],
    /// The declarations, which the report leaves out when there are none.
    #[serde(skip_serializing_if = "<[Customisation]>::is_empty")]
    customisations: &'a [Customisation],
}

/// Returns the text report of `entries` and of the declarations `declared`
/// for the tool `tool`.
fn text(tool: &str, entries: &[Entry], declared: &[Customisation]) -> String {
    let budgets = entries
        .iter()
        .filter(|entry| entry.budget.is_some())
        .count();
    let opt_outs = match entries.len() - budgets {
        0 => "no opt-outs".to_owned(),
        1 => "1 opt-out".to_owned(),
        count => format!("{count} opt-outs"),
    };
    let mut text = match budgets {
        0 => format!("{tool} has {opt_outs} from the conventions.\n"),
        1 => format!("{tool} has {opt_outs} from the conventions and sets 1 budget.\n"),
        count => {
            format!("{tool} has {opt_outs} from the conventions and sets {count} budgets.\n")
        }
    };
    for entry in entries {
        // Writing to a `String` cannot fail.
        let _ = writeln!(text, "\n{}: {}", entry.rule, entry.applies_to);
        if let Some(budget) = &entry.budget {
            let _ = writeln!(
                text,
                "  Budget: {} bytes, where the default is {} bytes.",
                budget.bytes, budget.default
            );
        }
        let _ = writeln!(text, "  Reason: {}", entry.reason);
        for cost in &entry.costs {
            let _ = writeln!(text, "  Cost: {cost}");
        }
    }
    // A customisation with a reason, such as root verbs, makes the line
    // leave out that customisations need no reason.
    let reasoned = declared
        .iter()
        .any(|customisation| customisation.reason.is_some());
    match (declared.len(), reasoned) {
        (0, _) => {}
        (1, false) => {
            let _ = writeln!(
                text,
                "\n{tool} declares 1 customisation, which needs no reason."
            );
        }
        (1, true) => {
            let _ = writeln!(text, "\n{tool} declares 1 customisation.");
        }
        (count, false) => {
            let _ = writeln!(
                text,
                "\n{tool} declares {count} customisations, which need no reason."
            );
        }
        (count, true) => {
            let _ = writeln!(text, "\n{tool} declares {count} customisations.");
        }
    }
    for customisation in declared {
        let _ = writeln!(
            text,
            "\n{}: {}\n  Declares: {}",
            customisation.declaration, customisation.applies_to, customisation.declares
        );
        if let Some(reason) = customisation.reason {
            let _ = writeln!(text, "  Reason: {reason}");
        }
    }
    text
}

/// Returns the JSON report with the id `schema` of `entries` and of the
/// declarations `declared`, pretty-printed and ending with a line break.
fn json(schema: &str, entries: &[Entry], declared: &[Customisation]) -> String {
    let report = Report {
        schema,
        data: entries,
        customisations: declared,
    };
    // Serializing strings and lists of strings cannot fail, and an empty
    // text would show as a differing file.
    serde_json::to_string_pretty(&report).unwrap_or_default() + "\n"
}
