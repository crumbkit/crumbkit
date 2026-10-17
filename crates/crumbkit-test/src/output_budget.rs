//! The output budget check, which runs each `--json` example of a tool's
//! guide, and each text example of a headed leaf, again with half the bytes
//! it printed, and checks that the tool keeps crumbkit's contract for a
//! result that does not fit (ADR-0046, ADR-0062).

use std::path::Path;
use std::time::Duration;

use clap::Command;
use crumbkit::Spelling;
use crumbkit::help::Guide;

use crate::allow::{Allowances, Rule};
use crate::examples::{Examples, example_golden_name, read_stdin, setup_problem};
use crate::guide::command_line;
use crate::runs::{self, Place, TIMEOUT, WorkDir, Xdg};
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
/// - it exits with code 0 and prints one document whose `data`, or the
///   data key that the document's envelope declares, is one object, which a
///   document that always prints its first answer does, or an array of one
///   row, which a batch of one answer prints;
/// - it exits with the code that the guide gives the over-budget failure,
///   which is 1 unless an [`Exit`](crumbkit::help::Exit) lists the code,
///   and its last JSON document, on one line or
///   indented, is a failure document, a document with one in `failures`,
///   or an array that holds one, at the top or under the declared data key,
///   as a batch that prints its failures in input order prints it, with the
///   field `budget` set to the budget, or, for a document without the
///   envelope, standard error names the budget. A batch whose document
///   answers each key left out (`Document::left_out_per_key`) can print
///   another failure after its over-budget failures, and exits with the
///   code of its first failure in key order. When that failure is one of
///   the tool's own, the run must exit with the code that the guide gives
///   it. The check reads the failure's code in the `::`, `.` and `/`
///   spellings, and expects 1 for a code that it cannot match to an exit.
///
/// The check also runs each example without `--json` of a leaf whose
/// document is headed (ADR-0062). Such a run keeps the contract when it
/// prints at most the budget on standard output, exits with the code that
/// the guide gives the over-budget failure, and names the budget on standard
/// error. A run of a document that always prints its first answer may
/// instead print more than the budget and exit with code 0, because its one
/// answer prints whole.
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
            env.push((VARIABLE, &value));
            match run(examples, &env, &args, stdin, timeout) {
                Ok(runs::Ran::Exited(output)) => {
                    let ran = Ran {
                        code: output.status.code(),
                        stdout: &String::from_utf8_lossy(&output.stdout),
                        stderr: &String::from_utf8_lossy(&output.stderr),
                        data_key: leaf
                            .get_document()
                            .map_or("data", |document| document.get_envelope().data_key()),
                    };
                    let printed = output.stdout.len();
                    let failed =
                        if document.is_some_and(|document| document.answers_left_out_per_key()) {
                            first_exit(&ran, guide, path, budget, text).unwrap_or(failed)
                        } else {
                            failed
                        };
                    let problem = if text {
                        judge_text(&ran, printed, budget, failed, first_always)
                    } else {
                        judge(&ran, budget, failed)
                    };
                    if let Some(problem) = problem {
                        problems.push(format!("{line}: {problem}"));
                    }
                }
                Ok(runs::Ran::Stopped) => problems.push(format!(
                    "{line}: with an output budget of {budget} bytes, {}",
                    runs::stopped(timeout)
                )),
                Err(problem) => problems.push(format!("{line}: {problem}")),
            }
        }
    }
    problems
}

/// Runs the binary of `examples` once with `env`, `args` and `stdin` in
/// fresh XDG directories, in the working directory of `examples` or a new
/// copy of its fresh fixture.
fn run(
    examples: &Examples<'_>,
    env: &[(&str, &str)],
    args: &[&str],
    stdin: Option<&[u8]>,
    timeout: Duration,
) -> Result<runs::Ran, String> {
    let xdg = Xdg::new("output-budget", examples.xdg_fixture())?;
    let work = WorkDir::new("output-budget-dir", examples.working_dir())?;
    let place = Place {
        dir: work.path(),
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
    /// The key that holds the result in the leaf's document: `data`, or the
    /// data key that the document's envelope declares.
    pub(crate) data_key: &'a str,
}

/// Returns the exit code of the first failure in key order of `ran`, a run
/// of the command at `path` whose output budget was `budget`, when that
/// failure is one of the tool's own, or `None` when it is an over-budget
/// failure or the run printed no failure. A text run, which `text` marks,
/// prints its failures on standard error, and a JSON run prints them in its
/// last document. A code that no exit of `guide` lists in a built-in
/// spelling exits with 1.
fn first_exit(
    ran: &Ran<'_>,
    guide: &Guide,
    path: &[&str],
    budget: usize,
    text: bool,
) -> Option<u8> {
    let code = if text {
        first_text_code(ran.stderr, budget)?
    } else {
        first_json_code(ran, budget)?
    };
    let exit = guide
        .get_exits()
        .iter()
        .flat_map(crumbkit::help::Exit::get_failures)
        .find(|listed| {
            let printed = guide.printed(path, listed);
            [Spelling::COLONS, Spelling::DOTS, Spelling::SLASHES]
                .iter()
                .any(|spelling| spelling.spell(printed) == code)
        })
        .map_or(1, |listed| guide.failure_exit(path, listed));
    Some(exit)
}

/// Returns the code of the first failure on standard error `stderr`, or
/// `None` when it is the over-budget failure of `budget` bytes or there is
/// none. Each failure starts with an `error:` line and names its code on a
/// `code:` line.
fn first_text_code(stderr: &str, budget: usize) -> Option<String> {
    let mut lines = stderr
        .lines()
        .skip_while(|line| !line.starts_with("error:"));
    let error = lines.next()?;
    if error.contains(&format!("output budget of {budget} bytes")) {
        return None;
    }
    lines
        .take_while(|line| !line.starts_with("error:"))
        .find_map(|line| line.trim_start().strip_prefix("code:"))
        .map(|code| code.trim().to_owned())
}

/// Returns the code of the failure with the lowest index in the last JSON
/// document of `ran`, or `None` when it is the over-budget failure of
/// `budget` bytes or there is none. The failures are a failure document,
/// the document's `failures`, or the failures among the rows of an array in
/// input order, at the top or under the data key.
fn first_json_code(ran: &Ran<'_>, budget: usize) -> Option<String> {
    let documents = runs::documents(ran.stdout);
    let last = documents.last()?;
    let first = if last.get("code").is_some() {
        last
    } else {
        last["failures"]
            .as_array()
            .or_else(|| last[ran.data_key].as_array())
            .or_else(|| last.as_array())?
            .iter()
            .filter(|failure| failure.get("code").is_some())
            .min_by_key(|failure| failure["index"].as_u64().unwrap_or(u64::MAX))?
    };
    if first["fields"]["budget"] == budget {
        return None;
    }
    first["code"].as_str().map(str::to_owned)
}

/// Returns the problem of `ran`, a run whose output budget was `budget`
/// bytes and whose over-budget failure exits with `failed`, or `None` when
/// it keeps the contract that [`check_output_budget`] describes.
pub(crate) fn judge(ran: &Ran<'_>, budget: usize, failed: u8) -> Option<String> {
    let documents = runs::documents(ran.stdout);
    let printed = ran.stdout.len();
    let names_budget = ran
        .stderr
        .contains(&format!("output stops at {budget} bytes"))
        || ran
            .stderr
            .contains(&format!("output budget of {budget} bytes"));
    match ran.code {
        Some(0) if printed > budget => {
            // A batch always prints its first answer, so a batch of one
            // answer prints it whole in an array of one row.
            let one_row = matches!(documents.as_slice(), [document]
                if document[ran.data_key].is_object()
                    || document[ran.data_key].as_array().is_some_and(|rows| rows.len() == 1));
            (!one_row).then(|| {
                format!(
                    "with an output budget of {budget} bytes, the run printed {printed} bytes and exited with code 0, and a run past its budget prints a single answer or fails with exit code {failed}"
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
        Some(code) if code == i32::from(failed) => {
            let over = documents.last().is_some_and(|last| {
                let failures = last["failures"]
                    .as_array()
                    .or_else(|| last[ran.data_key].as_array())
                    .or_else(|| last.as_array());
                // A batch that answers each key left out prints its
                // over-budget failures among its other failures.
                last["fields"]["budget"] == budget
                    || failures.is_some_and(|failures| {
                        failures
                            .iter()
                            .any(|failure| failure["fields"]["budget"] == budget)
                    })
            });
            (!over && !names_budget).then(|| {
                format!(
                    "with an output budget of {budget} bytes, the run exited with code {failed} without the over-budget failure, whose field `budget` names the budget"
                )
            })
        }
        code => Some(format!(
            "with an output budget of {budget} bytes, the run ended with {}, and a run past its budget exits with code 0 or {failed}",
            code.map_or_else(|| "a signal".to_owned(), |code| format!("exit code {code}"))
        )),
    }
}

/// Returns the problem of `ran`, a run of a text example of a headed leaf
/// that printed `printed` bytes with an output budget of `budget` bytes and
/// whose over-budget failure exits with `failed`, or `None` when it keeps
/// the contract that [`check_output_budget`] describes for such a run.
/// `first_always` is set when the leaf's document always prints its first
/// answer.
pub(crate) fn judge_text(
    ran: &Ran<'_>,
    printed: usize,
    budget: usize,
    failed: u8,
    first_always: bool,
) -> Option<String> {
    let names_budget = ran
        .stderr
        .contains(&format!("output budget of {budget} bytes"));
    match ran.code {
        _ if printed > budget && !first_always => Some(format!(
            "with an output budget of {budget} bytes, the run printed {printed} bytes, and a headed document prints more than its budget only for a first answer that its document always prints"
        )),
        Some(0) if printed > budget => None,
        Some(code) if code == i32::from(failed) && names_budget => None,
        Some(0) => Some(format!(
            "with an output budget of {budget} bytes, the run exited with code 0 without the over-budget failure, whose message names the budget on standard error"
        )),
        Some(code) if code == i32::from(failed) => Some(format!(
            "with an output budget of {budget} bytes, the run exited with code {failed} without the over-budget failure, whose message names the budget on standard error"
        )),
        code => Some(format!(
            "with an output budget of {budget} bytes, the run ended with {}, and a run past its budget exits with code 0 or {failed}",
            code.map_or_else(|| "a signal".to_owned(), |code| format!("exit code {code}"))
        )),
    }
}
