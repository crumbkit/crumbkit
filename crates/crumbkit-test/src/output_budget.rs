//! The output budget check, which runs each `--json` example of a tool's
//! guide, and each text example of a headed leaf, again with half the bytes
//! it printed, and checks that the tool keeps crumbkit's contract for a
//! result that does not fit (ADR-0046, ADR-0062).

use std::path::Path;
use std::time::Duration;

use clap::Command;
use crumbkit::help::Guide;

use crate::allow::{Allowances, Rule};
use crate::examples::{Examples, example_golden_name, read_stdin, setup_problem};
use crate::guide::command_line;
use crate::runs::{self, Place, TIMEOUT, Xdg};
use crate::tree;

#[cfg(test)]
mod tests;

/// The end of the name of the environment variable that lowers the output
/// budget of one run, whose name starts with the tool's name, such as
/// `SHELF_OUTPUT_BUDGET` (ADR-0079).
const VARIABLE: &str = "OUTPUT_BUDGET";

/// Checks that each `--json` example in `guide` keeps the output budget's
/// contract when its budget is half the bytes it prints, and panics, as a
/// failed test assertion does, naming every example that breaks it.
///
/// The check runs each example as [`check_examples`](crate::check_examples)
/// does, skips an example that fails or prints fewer than 2 bytes, and runs
/// it again with the tool's variable `<TOOL>_OUTPUT_BUDGET`, such as
/// `SHELF_OUTPUT_BUDGET`, set to half the bytes it printed. The second run keeps the contract when one of these holds:
///
/// - it exits with code 0, prints at most the budget on standard output, and
///   marks the cut: a document holds `cut` with the budget, or, for a
///   document without the envelope, standard error says that output stops
///   at the budget;
/// - it exits with code 0, its document always prints its first answer,
///   and it prints one document whose `data` is one object, or an array of
///   one row, which a batch of one answer prints;
/// - it exits with the code that the guide gives the over-budget failure,
///   which is 1 unless an [`Exit`](crumbkit::help::Exit) lists the code,
///   and its last JSON document, on one line or
///   indented, is a failure document, or a document whose last entry of
///   `failures` is one, with the field `budget` set to the budget, or, for
///   a document without the envelope, standard error names the budget.
///
/// The check also runs each example without `--json` of a leaf whose
/// document is headed (ADR-0062). Such a run keeps the contract when it
/// exits with the code that the guide gives the over-budget failure and
/// names the budget on standard error, and prints within the budget unless
/// its document always prints its first answer. It also keeps the contract
/// when it exits with code 0 and prints one answer past the budget, with at
/// most one `==> KEY <==` header, from a document that always prints its
/// first answer (ADR-0084).
///
/// Each run starts in the directory of
/// [`Examples::current_dir`](crate::Examples::current_dir) and reads the
/// example's `.stdin` file, as in `check_examples`. The check fails without
/// running any example when that directory does not exist. It skips the
/// examples that `examples` skips through
/// [`Rule::Examples`](crate::Rule::Examples) and
/// [`Allow::example`](crate::Allow::example).
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
    let names = guide.tool_name(command);
    let variable = names.variable(VARIABLE);
    let mut problems = Vec::new();
    if let Some(problem) = setup_problem(examples) {
        problems.push(problem);
        return problems;
    }
    let mut allowances = Allowances::new(examples.opt_outs(), &[Rule::Examples]);
    let visits = tree::walk(command);
    for leaf in guide.get_leaves() {
        let path = leaf.path();
        let below = path.join(" ");
        let failed = guide.failure_exit(path, &crumbkit::codes::OUTPUT_OVER_BUDGET);
        if !visits.iter().any(|visit| visit.below() == below)
            || allowances.excuses(Rule::Examples, &below)
        {
            continue;
        }
        let document = leaf.get_document();
        let headed = document.is_some_and(|document| document.is_headed());
        let first_always = document.is_some_and(|document| document.always_prints_first());
        for example in leaf.get_examples() {
            let text = !example.words().contains(&"--json");
            if text && !headed {
                continue;
            }
            let args: Vec<&str> = path.iter().chain(example.words()).copied().collect();
            let called = std::iter::once(tool)
                .chain(path.iter().copied())
                .collect::<Vec<_>>()
                .join(" ");
            let line = command_line(&called, example.words());
            if allowances.excuses_example(&below, example.words()) {
                continue;
            }
            let dir = Path::new(examples.goldens());
            let stdin = match read_stdin(dir, &example_golden_name(&line)) {
                Ok(stdin) => stdin,
                Err(problem) => {
                    problems.push(format!("{line}: {problem}"));
                    continue;
                }
            };
            let stdin = stdin.as_deref();
            let whole = match run(examples, examples.variables(), &args, stdin, timeout) {
                Ok(runs::Ran::Exited(output)) if output.status.success() => output.stdout.len(),
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
            env.push((&variable, &value));
            match run(examples, &env, &args, stdin, timeout) {
                Ok(runs::Ran::Exited(output)) => {
                    let ran = Ran {
                        code: output.status.code(),
                        stdout: &String::from_utf8_lossy(&output.stdout),
                        stderr: &String::from_utf8_lossy(&output.stderr),
                    };
                    let printed = output.stdout.len();
                    let problem = if text {
                        judge_text(&ran, printed, budget, failed, first_always)
                    } else {
                        judge(&ran, budget, failed, first_always)
                    };
                    if let Some(problem) = problem {
                        problems.push(format!("{line}: {problem}"));
                    }
                }
                Ok(runs::Ran::Stopped) => problems.push(format!(
                    "{line}: with an output budget of {}, {}",
                    byte_count(budget),
                    runs::stopped(timeout)
                )),
                Err(problem) => problems.push(format!("{line}: {problem}")),
            }
        }
    }
    problems
}

/// Runs the binary of `examples` once with `env`, `args` and `stdin` in
/// fresh XDG directories, in the working directory of `examples`.
fn run(
    examples: &Examples<'_>,
    env: &[(&str, &str)],
    args: &[&str],
    stdin: Option<&[u8]>,
    timeout: Duration,
) -> Result<runs::Ran, String> {
    let xdg = Xdg::new("output-budget", examples.xdg_fixture())?;
    let place = Place {
        dir: examples.working_dir(),
        stdin,
        env: &[],
    };
    runs::run(examples.binary(), env, &xdg, args, place, timeout)
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
/// bytes and whose over-budget failure exits with `failed`, or `None` when
/// it keeps the contract that [`check_output_budget`] describes.
/// `first_always` reports whether the run's document always prints its
/// first answer.
pub(crate) fn judge(
    ran: &Ran<'_>,
    budget: usize,
    failed: u8,
    first_always: bool,
) -> Option<String> {
    let documents = runs::documents(ran.stdout);
    let printed = ran.stdout.len();
    let bytes = byte_count(budget);
    let names_budget = ran
        .stderr
        .contains(&format!("output stops at {}", byte_count(budget)))
        || names_budget(ran, budget);
    match ran.code {
        Some(0) if printed > budget => {
            // A document that always prints its first answer prints a
            // single result whole, or a batch of one answer in an array of
            // one row.
            let one_row = matches!(documents.as_slice(), [document]
                if document["data"].is_object()
                    || document["data"].as_array().is_some_and(|rows| rows.len() == 1));
            (!(one_row && first_always)).then(|| {
                format!(
                    "with an output budget of {bytes}, the run printed {printed} bytes and exited with code 0, and a run past its budget prints a single answer of a document that always prints its first answer or fails with exit code {failed}"
                )
            })
        }
        Some(0) => {
            let cut = documents
                .iter()
                .any(|document| document["cut"]["budget"] == budget);
            (!cut && !names_budget).then(|| {
                format!(
                    "with an output budget of {bytes}, the run printed a result without marking it as cut short, and a cut result holds `cut` with the budget"
                )
            })
        }
        Some(code) if code == i32::from(failed) => {
            let over = documents.last().is_some_and(|last| {
                last["fields"]["budget"] == budget
                    || last["failures"]
                        .as_array()
                        .and_then(|failures| failures.last())
                        .is_some_and(|failure| failure["fields"]["budget"] == budget)
            });
            (!over && !names_budget).then(|| {
                format!(
                    "with an output budget of {bytes}, the run exited with code {failed} without the over-budget failure, whose field `budget` names the budget"
                )
            })
        }
        code => Some(ended(code, &bytes, failed)),
    }
}

/// Returns the problem of `ran`, a run of a text example of a headed leaf
/// that printed `printed` bytes with an output budget of `budget` bytes and
/// whose over-budget failure exits with `failed`, or `None` when it keeps
/// the contract that [`check_output_budget`] describes for such a run.
/// `first_always` reports whether the leaf's document always prints its
/// first answer.
pub(crate) fn judge_text(
    ran: &Ran<'_>,
    printed: usize,
    budget: usize,
    failed: u8,
    first_always: bool,
) -> Option<String> {
    let bytes = byte_count(budget);
    match ran.code {
        Some(0) if printed > budget => {
            // A headed batch prints a header before each answer when it
            // has more than one key, and no header for one key. The judge
            // does not know which of the example's words are keys, so it
            // counts every line shaped as a header, and an answer whose
            // bytes hold such a line counts as more than one answer.
            let answers = ran
                .stdout
                .lines()
                .filter(|line| line.starts_with("==> ") && line.ends_with(" <=="))
                .count()
                .max(1);
            (answers > 1 || !first_always).then(|| {
                format!(
                    "with an output budget of {bytes}, the run printed {answers} answer{} in {printed} bytes and exited with code 0, and a run past its budget prints one answer of a document that always prints its first answer or fails with exit code {failed}",
                    if answers == 1 { "" } else { "s" }
                )
            })
        }
        Some(code) if code == i32::from(failed) && printed > budget && !first_always => {
            Some(format!(
                "with an output budget of {bytes}, the run printed {printed} bytes and exited with code {failed}, and a run whose document does not always print its first answer prints within its budget"
            ))
        }
        Some(code) if code == i32::from(failed) && names_budget(ran, budget) => None,
        Some(0) => Some(format!(
            "with an output budget of {bytes}, the run exited with code 0 without the over-budget failure, whose message names the budget on standard error"
        )),
        Some(code) if code == i32::from(failed) => Some(format!(
            "with an output budget of {bytes}, the run exited with code {failed} without the over-budget failure, whose message names the budget on standard error"
        )),
        code => Some(ended(code, &bytes, failed)),
    }
}

/// Returns the problem of a run with an output budget of `bytes` that ended
/// with `code`, which is neither 0 nor `failed`. `code` is `None` when a
/// signal ended the run.
fn ended(code: Option<i32>, bytes: &str, failed: u8) -> String {
    format!(
        "with an output budget of {bytes}, the run ended with {}, and a run past its budget exits with code 0 or {failed}",
        code.map_or_else(|| "a signal".to_owned(), |code| format!("exit code {code}"))
    )
}

/// Returns whether the standard error of `ran` names an output budget of
/// `budget` bytes as the over-budget failure's message does.
fn names_budget(ran: &Ran<'_>, budget: usize) -> bool {
    ran.stderr
        .contains(&format!("output budget of {}", byte_count(budget)))
}

/// Returns `bytes` as crumbkit's messages count bytes, such as `1 byte` or
/// `400 bytes`. crumbkit keeps its own copy private, because no tool needs
/// it, so the check words the count itself.
fn byte_count(bytes: usize) -> String {
    if bytes == 1 {
        "1 byte".to_owned()
    } else {
        format!("{bytes} bytes")
    }
}
