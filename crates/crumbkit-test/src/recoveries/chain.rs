//! The walk of the recovery check through the chains of recoveries that
//! each start prints.

use std::fmt::Write as _;
use std::time::Instant;

use clap::Command;
use crumbkit::form::{FailureForm, Step};
use crumbkit::help::split;

use super::{Failing, Recoveries};
use crate::allow::{Allowances, Rule};
use crate::guide::command_line;
use crate::runs::{self, Place, TIMEOUT, Xdg};

/// What the check has found so far: its settings, the problems, the report
/// and the codes that the chains provoked.
pub(super) struct Walk<'a> {
    /// The check's settings.
    pub(super) recoveries: &'a Recoveries<'a>,
    /// The tool's command tree.
    pub(super) command: &'a Command,
    /// The spelled codes whose recoveries the check follows: the declared
    /// codes and crumbkit's own codes.
    pub(super) known: Vec<String>,
    /// The component of each declared code, which
    /// [`Spelling::split`](crumbkit::Spelling::split) reads back. It is
    /// empty for a custom spelling, which cannot be read back.
    pub(super) components: Vec<String>,
    /// crumbkit's usage code, as the tool spells it.
    pub(super) usage: String,
    /// The opt-outs of `recoveries/follow` and `recoveries/same-code`.
    pub(super) allowances: Allowances<'a>,
    /// The problems found, one line each.
    pub(super) problems: Vec<String>,
    /// The text of the golden report.
    pub(super) report: String,
    /// The spelled codes that a start or a step failed with.
    pub(super) provoked: Vec<String>,
    /// The number of failures whose codes the check did not know.
    pub(super) skipped: usize,
    /// The tool's failure form, which reads each failed run first.
    pub(super) form: Option<&'a dyn FailureForm>,
    /// The problem that stopped the walk: a failed run whose failure the
    /// tool's form does not read, so no later run can be followed.
    pub(super) stopped: Option<String>,
}

/// How a run ended: with code 0, or with a failure document's code and
/// steps.
enum Outcome {
    /// The run exited with code 0.
    Succeeds,
    /// The run failed with this code and offers these steps, the recovery
    /// first.
    Fails(String, Vec<Step>),
}

/// The settings that every run of one chain shares.
struct Chain<'c> {
    /// The command line of the start, which names each of its problems.
    start: String,
    /// The XDG directories of every run of the chain.
    xdg: Xdg,
    /// The working directory and standard input of every run.
    place: Place<'c>,
    /// The words of each command line that the chain ran.
    ran: Vec<Vec<String>>,
}

impl Walk<'_> {
    /// Runs `start` and follows the steps of its failure, unless an opt-out
    /// from `recoveries/follow` covers its command.
    pub(super) fn start(&mut self, start: &Failing<'_>) {
        let tool = self.command.get_name();
        let line = command_line(tool, start.words);
        if self.stopped.is_some() {
            return;
        }
        if self
            .allowances
            .excuses(Rule::Recoveries, &path_below(self.command, start.words))
        {
            return;
        }
        let code = self.recoveries.spelling.spell(start.code);
        if !self.known.contains(&code) {
            self.problems.push(format!(
                "{line}: the start's code {code} is not a declared code"
            ));
            return;
        }
        let xdg = match Xdg::new("recoveries") {
            Ok(xdg) => xdg,
            Err(problem) => {
                self.problems.push(format!("{line}: {problem}"));
                return;
            }
        };
        let _ = writeln!(self.report, "{line}");
        let mut chain = Chain {
            start: line,
            xdg,
            place: start.place(self.recoveries.examples.working_dir()),
            ran: Vec::new(),
        };
        let words: Vec<String> = start.words.iter().map(|&word| word.to_owned()).collect();
        match self.run(&mut chain, "the start", &words) {
            Some(Outcome::Succeeds) => self.problems.push(format!(
                "{}: the start must fail with {code}, and it exits with code 0",
                chain.start
            )),
            Some(Outcome::Fails(failed, steps)) => {
                self.provoked.push(failed.clone());
                if failed == code {
                    let _ = writeln!(self.report, "  fails with {code}");
                    self.follow(&mut chain, &code, &steps, 1);
                } else {
                    self.problems.push(format!(
                        "{}: the start must fail with {code}, and it fails with {failed}",
                        chain.start
                    ));
                }
            }
            None => {}
        }
        self.report.push('\n');
    }

    /// Follows each of `steps`, which a failure with the code `parent`
    /// offers, as the `depth`th step of `chain`, writing each to the report
    /// indented by `depth`.
    fn follow(&mut self, chain: &mut Chain<'_>, parent: &str, steps: &[Step], depth: u8) {
        let indent = "  ".repeat(usize::from(depth));
        for step in steps {
            if self.stopped.is_some() {
                return;
            }
            let call = match step {
                Step::Run(call) => call,
                Step::Retry => {
                    let _ = writeln!(self.report, "{indent}retry");
                    continue;
                }
                Step::Hint(_) => {
                    let _ = writeln!(self.report, "{indent}hint");
                    continue;
                }
                _ => {
                    let _ = writeln!(self.report, "{indent}no next step");
                    continue;
                }
            };
            let Some(words) = self.fill(chain, call) else {
                continue;
            };
            let refs: Vec<&str> = words.iter().map(String::as_str).collect();
            let filled = command_line(self.command.get_name(), &refs);
            if filled == *call {
                let _ = writeln!(self.report, "{indent}run {call}");
            } else {
                let _ = writeln!(self.report, "{indent}run {call} as {filled}");
            }
            let path = path_below(self.command, &refs);
            let stopped = if self.allowances.excuses(Rule::Recoveries, &path) {
                Some("an opt-out from recoveries/follow covers it".to_owned())
            } else if depth > self.recoveries.steps {
                Some(format!(
                    "the chain reached Recoveries::steps, {}",
                    self.recoveries.steps
                ))
            } else if chain.ran.contains(&with_json(&words)) {
                Some("the chain ran it already".to_owned())
            } else {
                None
            };
            if let Some(reason) = stopped {
                let _ = writeln!(self.report, "{indent}  not run, because {reason}");
                continue;
            }
            let label = format!("the step `{call}`");
            match self.run(chain, &label, &words) {
                Some(Outcome::Succeeds) => {
                    let _ = writeln!(self.report, "{indent}  exits 0");
                }
                Some(Outcome::Fails(code, _)) if code == self.usage => {
                    let _ = writeln!(self.report, "{indent}  refused by the parser with {code}");
                    self.problems.push(format!(
                        "{}: the parser refuses {label} with {code}",
                        chain.start
                    ));
                }
                Some(Outcome::Fails(code, next)) => {
                    self.provoked.push(code.clone());
                    if !self.known.contains(&code) {
                        if let Some((component, _, _)) = self.recoveries.spelling.split(&code)
                            && self.components.iter().any(|known| known == component)
                        {
                            let _ = writeln!(
                                self.report,
                                "{indent}  fails with {code}, which no declared code spells"
                            );
                            self.problems.push(format!(
                                "{}: {label} fails with {code}, whose component {component} is a declared component, and no declared code spells it",
                                chain.start
                            ));
                            continue;
                        }
                        self.skipped += 1;
                        let _ = writeln!(
                            self.report,
                            "{indent}  fails with {code}, which no declared code spells"
                        );
                        continue;
                    }
                    if code == parent && !self.allowances.excuses(Rule::SameCode, &path) {
                        self.problems.push(format!(
                            "{}: {label} fails again with {code}, so an agent that follows the recovery learns nothing new",
                            chain.start
                        ));
                    }
                    let _ = writeln!(self.report, "{indent}  fails with {code}");
                    self.follow(chain, &code, &next, depth + 1);
                }
                None => {}
            }
        }
    }

    /// Returns the words of `call` after the tool's name, with each
    /// placeholder replaced by its value, or adds the problem and returns
    /// `None` when the call cannot be read, does not start with the tool's
    /// name or has a placeholder without a value.
    fn fill(&mut self, chain: &Chain<'_>, call: &str) -> Option<Vec<String>> {
        let tool = self.command.get_name();
        let problem = match split(call) {
            None => format!("the step `{call}` is not a command line that crumbkit quotes"),
            Some(words) if words.first().map(String::as_str) != Some(tool) => {
                format!("the step `{call}` must start with the tool's name, {tool}")
            }
            Some(words) => {
                let mut filled = Vec::with_capacity(words.len() - 1);
                let mut missing = None;
                for word in words.into_iter().skip(1) {
                    match placeholder(&word) {
                        None => filled.push(word),
                        Some(name) => {
                            match self.recoveries.values.iter().find(|(key, _)| *key == name) {
                                Some(&(_, value)) => filled.push(value.to_owned()),
                                None => missing = Some(word),
                            }
                        }
                    }
                }
                let Some(word) = missing else {
                    return Some(filled);
                };
                format!(
                    "the step `{call}` has the placeholder {}, and no value of Recoveries::values fills it",
                    word.trim_end_matches("...")
                )
            }
        };
        self.problems.push(format!("{}: {problem}", chain.start));
        None
    }

    /// Runs `words`, with `--json` added when they lack it, as one run of
    /// `chain`, and returns how it ended, or adds the problem and returns
    /// `None` when the run cannot start, does not end, writes a crumbkit
    /// warning or fails with a failure that the check cannot read. A failure
    /// that the tool's form does not read stops the walk. `label` names the
    /// run in a problem, such as "the start".
    fn run(&mut self, chain: &mut Chain<'_>, label: &str, words: &[String]) -> Option<Outcome> {
        let args = with_json(words);
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let started = Instant::now();
        let result = runs::run(
            self.recoveries.examples.binary(),
            self.recoveries.examples.variables(),
            &chain.xdg,
            &refs,
            chain.place,
            TIMEOUT,
        );
        chain.ran.push(args);
        let output = match result {
            Ok(output) => output,
            Err(problem) => {
                self.problems.push(format!("{}: {problem}", chain.start));
                return None;
            }
        };
        if !output.status.success() && started.elapsed() >= TIMEOUT {
            self.problems.push(format!(
                "{}: {label} did not end within {} seconds, so the check stopped it",
                chain.start,
                TIMEOUT.as_secs()
            ));
            return None;
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        for warning in stderr
            .lines()
            .filter(|line| line.starts_with("warning: crumbkit "))
        {
            self.problems.push(format!(
                "{}: {label} writes a warning to standard error: {warning}",
                chain.start
            ));
        }
        if output.status.success() {
            return Some(Outcome::Succeeds);
        }
        let Some(failure) = runs::read_failure(self.form, &output.stdout, &output.stderr) else {
            match self.form {
                Some(form) => {
                    self.stopped = Some(format!(
                        "{}: {label} ends with {}, and the failure form {} reads no failure from it, so the check cannot follow the tool's recoveries. The tool implements FailureForm::read, or adds an opt-out from `{}` that names the root, with a reason",
                        chain.start,
                        output.status,
                        form.document(),
                        Rule::Recoveries.id()
                    ));
                }
                None => self.problems.push(format!(
                    "{}: {label} ends with {} and prints no failure on standard output",
                    chain.start, output.status
                )),
            }
            return None;
        };
        Some(Outcome::Fails(
            failure.code().to_owned(),
            failure.steps().to_vec(),
        ))
    }
}

/// Returns `words` with `--json` added at the end when they lack it.
fn with_json(words: &[String]) -> Vec<String> {
    let mut args = words.to_vec();
    if !args.iter().any(|word| word == "--json") {
        args.push("--json".to_owned());
    }
    args
}

/// Returns the name between the angle brackets of `word` when it is a
/// placeholder, such as `ID` for `<ID>` or `<ID>...`.
//
// RUST-10: `crumbkit::help::split` returns plain words, so the check cannot
// tell a quoted value from a placeholder. A call whose real value is such
// text, such as `'<ID>'`, reads as a placeholder. A call that kept its
// placeholders apart from its values replaces this if a tool needs one.
fn placeholder(word: &str) -> Option<&str> {
    let name = word
        .strip_suffix("...")
        .unwrap_or(word)
        .strip_prefix('<')?
        .strip_suffix('>')?;
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '-');
    valid.then_some(name)
}

/// Returns the path below `root` of the deepest command that `words` name
/// from their start, with its names joined by single spaces, such as
/// `books show` for `books show 7`.
fn path_below<S: AsRef<str>>(root: &Command, words: &[S]) -> String {
    let mut command = root;
    let mut names = Vec::new();
    for word in words {
        let Some(sub) = command.find_subcommand(word.as_ref()) else {
            break;
        };
        names.push(sub.get_name());
        command = sub;
    }
    names.join(" ")
}
