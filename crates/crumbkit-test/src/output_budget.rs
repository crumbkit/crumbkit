//! The output budget check, which runs each `--json` example of a tool's
//! guide again with half the bytes it printed, and checks that the tool
//! keeps crumbkit's contract for a result that does not fit (ADR-0046).

use std::time::Duration;

use clap::Command;
use crumbkit::help::Guide;
use serde_json::Value;

use crate::allow::{Allowances, Rule};
use crate::examples::Examples;
use crate::guide::command_line;
use crate::runs::{self, Place, TIMEOUT, Xdg};
use crate::tree;

#[cfg(test)]
mod tests;

/// The environment variable that lowers the output budget of one run.
const VARIABLE: &str = "CRUMBKIT_OUTPUT_BUDGET";

/// Checks that each `--json` example in `guide` keeps the output budget's
/// contract when its budget is half the bytes it prints, and panics, as a
/// failed test assertion does, naming every example that breaks it.
///
/// The check runs each example as [`check_examples`](crate::check_examples)
/// does, skips an example that fails or prints fewer than 2 bytes, and runs
/// it again with the variable `CRUMBKIT_OUTPUT_BUDGET` set to half the bytes
/// it printed. The second run keeps the contract when one of these holds:
///
/// - it exits with code 0, prints at most the budget on standard output, and
///   marks the cut: a document holds `cut` with the budget, or, for a
///   document without the envelope, standard error says that output stops
///   at the budget;
/// - it exits with code 0 and prints one document whose `data` is one
///   object, which a document that always prints its first answer does;
/// - it exits with code 1 and its last line is a failure document, or a
///   document whose last entry of `failures` is one, with the field
///   `budget` set to the budget, or, for a document without the envelope,
///   standard error names the budget.
///
/// It skips the examples that `examples` skips through
/// [`Rule::Examples`](crate::Rule::Examples).
///
/// ```no_run
/// use clap::Command;
/// use crumbkit::help::{Example, Guide, Leaf};
/// use crumbkit_test::Examples;
///
/// const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"])
///     .examples(&[Example::new(&["--all", "--json"], "Print every book as JSON")])]);
///
/// // In an integration test, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// const EXAMPLES: Examples<'_> = Examples::new("/path/to/shelf", "tests/golden/examples");
///
/// let shelf = Command::new("shelf")
///     .subcommand(Command::new("books").subcommand(Command::new("list")));
///
/// crumbkit_test::check_output_budget(&shelf, &GUIDE, &EXAMPLES);
/// ```
///
/// # Panics
///
/// Panics when a run breaks the contract or cannot start, or when the check
/// cannot create a run's XDG directories. The message lists each problem
/// on its own line under the line `these examples break the output budget's
/// contract:`.
pub fn check_output_budget(command: &Command, guide: &Guide, examples: &Examples<'_>) {
    let problems = check(command, guide, examples, TIMEOUT);
    tree::report(
        "these examples break the output budget's contract",
        &problems,
    );
}

/// Runs [`check_output_budget`] with `timeout` for each run, and returns
/// its problems.
fn check(
    command: &Command,
    guide: &Guide,
    examples: &Examples<'_>,
    timeout: Duration,
) -> Vec<String> {
    let tool = command.get_name();
    let mut problems = Vec::new();
    let mut allowances = Allowances::new(examples.opt_outs(), &[Rule::Examples]);
    let visits = tree::walk(command);
    for leaf in guide.get_leaves() {
        let path = leaf.path();
        let below = path.join(" ");
        if !visits.iter().any(|visit| visit.below() == below)
            || allowances.excuses(Rule::Examples, &below)
        {
            continue;
        }
        for example in leaf.get_examples() {
            if !example.words().contains(&"--json") {
                continue;
            }
            let args: Vec<&str> = path.iter().chain(example.words()).copied().collect();
            let text = std::iter::once(tool)
                .chain(path.iter().copied())
                .collect::<Vec<_>>()
                .join(" ");
            let line = command_line(&text, example.words());
            let whole = match run(examples, examples.variables(), &args, timeout) {
                Ok(output) if output.status.success() => output.stdout.len(),
                Ok(_) => continue,
                Err(problem) => {
                    problems.push(format!("{line}: {problem}"));
                    continue;
                }
            };
            if whole < 2 {
                continue;
            }
            let budget = whole / 2;
            let value = budget.to_string();
            let mut env = examples.variables().to_vec();
            env.push((VARIABLE, &value));
            match run(examples, &env, &args, timeout) {
                Ok(output) => {
                    let ran = Ran {
                        code: output.status.code(),
                        stdout: &String::from_utf8_lossy(&output.stdout),
                        stderr: &String::from_utf8_lossy(&output.stderr),
                    };
                    if let Some(problem) = judge(&ran, budget) {
                        problems.push(format!("{line}: {problem}"));
                    }
                }
                Err(problem) => problems.push(format!("{line}: {problem}")),
            }
        }
    }
    problems
}

/// Runs the binary of `examples` once with `env` and `args` in fresh XDG
/// directories.
fn run(
    examples: &Examples<'_>,
    env: &[(&str, &str)],
    args: &[&str],
    timeout: Duration,
) -> Result<std::process::Output, String> {
    let xdg = Xdg::new("output-budget")?;
    runs::run(
        examples.binary(),
        env,
        &xdg,
        args,
        Place::default(),
        timeout,
    )
}

/// What one run with a lowered budget printed and how it ended.
pub(crate) struct Ran<'a> {
    /// The exit code, or `None` when a signal ended the run.
    pub(crate) code: Option<i32>,
    /// The run's standard output.
    pub(crate) stdout: &'a str,
    /// The run's standard error.
    pub(crate) stderr: &'a str,
}

/// Returns the problem of `ran`, a run whose output budget was `budget`
/// bytes, or `None` when it keeps the contract that
/// [`check_output_budget`] describes.
pub(crate) fn judge(ran: &Ran<'_>, budget: usize) -> Option<String> {
    let documents: Vec<Value> = ran
        .stdout
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let printed = ran.stdout.len();
    let names_budget = ran
        .stderr
        .contains(&format!("output stops at {budget} bytes"))
        || ran
            .stderr
            .contains(&format!("output budget of {budget} bytes"));
    match ran.code {
        Some(0) if printed > budget => {
            let one_row =
                matches!(documents.as_slice(), [document] if document["data"].is_object());
            (!one_row).then(|| {
                format!(
                    "with an output budget of {budget} bytes, the run printed {printed} bytes and exited with code 0, and a run past its budget prints a single answer or fails with exit code 1"
                )
            })
        }
        Some(0) => {
            let cut = documents
                .iter()
                .any(|document| document["cut"]["budget"] == budget);
            (!cut && !names_budget).then(|| {
                format!(
                    "with an output budget of {budget} bytes, the run printed a result without marking it as cut short, and a cut result holds `cut` with the budget"
                )
            })
        }
        Some(1) => {
            let failed = documents.last().is_some_and(|last| {
                last["fields"]["budget"] == budget
                    || last["failures"]
                        .as_array()
                        .and_then(|failures| failures.last())
                        .is_some_and(|failure| failure["fields"]["budget"] == budget)
            });
            (!failed && !names_budget).then(|| {
                format!(
                    "with an output budget of {budget} bytes, the run exited with code 1 without the over-budget failure, whose field `budget` names the budget"
                )
            })
        }
        code => Some(format!(
            "with an output budget of {budget} bytes, the run ended with {}, and a run past its budget exits with code 0 or 1",
            code.map_or_else(|| "a signal".to_owned(), |code| format!("exit code {code}"))
        )),
    }
}
