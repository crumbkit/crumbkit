//! The walk of the recovery check through the chains of recoveries that
//! each start prints.

use std::fmt::Write as _;
use std::time::Instant;

use clap::Command;
use crumbkit::form::Step;

use super::words::split;
use super::{Failing, Recoveries};
use crate::allow::{self, Allowances, Rule};
use crate::failures::{self, Reader};
use crate::guide::command_line;
use crate::runs::{self, Place, TIMEOUT, WorkDir, Xdg};
use fill::{fill_word, path_below, value, with_json};

pub(super) mod fill;

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
    /// The opt-outs of `recoveries/follow`, `recoveries/same-code` and
    /// `recoveries/field-value`.
    pub(super) allowances: Allowances<'a>,
    /// The problems found, one line each.
    pub(super) problems: Vec<String>,
    /// The text of the golden report.
    pub(super) report: String,
    /// The spelled codes that a start or a step failed with.
    pub(super) provoked: Vec<String>,
    /// The number of failures whose codes the check did not know.
    pub(super) skipped: usize,
    /// The tool's failure form or failure reader, which reads each failed
    /// run first.
    pub(super) reader: Option<Reader<'a>>,
    /// The problem that stopped the walk: a failed run whose failure the
    /// tool's form or reader does not read, so no later run can be
    /// followed.
    pub(super) stopped: Option<String>,
}

/// The failure that offers a chain's next steps.
struct Parent<'p> {
    /// The failure's code, as the tool spells it.
    code: &'p str,
    /// The failure's fields, which fill the steps' placeholders.
    fields: &'p [(String, String)],
    /// The path below the root of the command that failed, which an
    /// opt-out from `recoveries/field-value` names.
    path: &'p str,
}

/// The words of a filled call after the tool's name, and the name of each
/// placeholder that a field filled with the field's name.
type Filled = (Vec<String>, Vec<(String, String)>);

/// How a run ended: with code 0, or with a failure document's code and
/// steps.
enum Outcome {
    /// The run exited with code 0.
    Succeeds,
    /// The run failed with this code, offers these steps, the recovery
    /// first, and names these fields, which fill the steps' placeholders.
    Fails(String, Vec<Step>, Vec<(String, String)>),
}

/// The settings that every run of one chain shares.
struct Chain<'c> {
    /// The command line of the start, which names each of its problems.
    start: String,
    /// The XDG directories of every run of the chain.
    xdg: Xdg,
    /// The working directory of every run, which is one copy of a fresh
    /// fixture for the whole chain.
    dir: WorkDir,
    /// The standard input of every run.
    stdin: Option<&'c [u8]>,
    /// The start's own value of each placeholder.
    values: &'c [(&'c str, &'c str)],
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
        let fixture = start.fixture(self.recoveries.examples.xdg_fixture());
        let xdg = match Xdg::new("recoveries", fixture) {
            Ok(xdg) => xdg,
            Err(problem) => {
                self.problems.push(format!("{line}: {problem}"));
                return;
            }
        };
        let dir = match WorkDir::new(
            "recoveries-dir",
            start.work_dir(self.recoveries.examples.working_dir()),
        ) {
            Ok(dir) => dir,
            Err(problem) => {
                self.problems.push(format!("{line}: {problem}"));
                return;
            }
        };
        let _ = writeln!(self.report, "{line}");
        let mut chain = Chain {
            start: line,
            xdg,
            dir,
            stdin: start.stdin_bytes(),
            values: start.own_values(),
            ran: Vec::new(),
        };
        let words: Vec<String> = start.words.iter().map(|&word| word.to_owned()).collect();
        match self.run(&mut chain, "the start", &words) {
            Some(Outcome::Succeeds) => self.problems.push(format!(
                "{}: the start must fail with {code}, and it exits with code 0",
                chain.start
            )),
            Some(Outcome::Fails(failed, steps, fields)) => {
                self.provoked.push(failed.clone());
                if failed == code {
                    let _ = writeln!(self.report, "  fails with {code}");
                    let path = path_below(self.command, start.words);
                    let parent = Parent {
                        code: &code,
                        fields: &fields,
                        path: &path,
                    };
                    self.follow(&mut chain, &parent, &steps, 1);
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

    /// Follows each of `steps`, which the failure `parent` offers, as the
    /// `depth`th step of `chain`, writing each to the report indented by
    /// `depth`.
    fn follow(&mut self, chain: &mut Chain<'_>, parent: &Parent<'_>, steps: &[Step], depth: u8) {
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
            let Some((words, used)) = self.fill(chain, call, parent.fields) else {
                continue;
            };
            let refs: Vec<&str> = words.iter().map(String::as_str).collect();
            let filled = command_line(self.command.get_name(), &refs);
            if filled == *call {
                let _ = writeln!(self.report, "{indent}run {call}");
            } else {
                let _ = writeln!(self.report, "{indent}run {call} as {filled}");
            }
            self.report_fields(chain, parent, call, &used, &indent);
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
                Some(Outcome::Fails(code, ..)) if code == self.usage => {
                    let _ = writeln!(self.report, "{indent}  refused by the parser with {code}");
                    self.problems.push(format!(
                        "{}: the parser refuses {label} with {code}",
                        chain.start
                    ));
                }
                Some(Outcome::Fails(code, next, next_fields)) => {
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
                    if code == parent.code && !self.allowances.excuses(Rule::SameCode, &path) {
                        self.problems.push(format!(
                            "{}: {label} fails again with {code}, so an agent that follows the recovery learns nothing new",
                            chain.start
                        ));
                    }
                    let _ = writeln!(self.report, "{indent}  fails with {code}");
                    let next_parent = Parent {
                        code: &code,
                        fields: &next_fields,
                        path: &path,
                    };
                    self.follow(chain, &next_parent, &next, depth + 1);
                }
                None => {}
            }
        }
    }

    /// Writes each placeholder of `call` that a field of `parent` filled,
    /// as `used` names them, to the report indented by `indent`, and adds
    /// a problem for each unless an opt-out from `recoveries/field-value`
    /// covers the command whose failure printed `call`.
    fn report_fields(
        &mut self,
        chain: &Chain<'_>,
        parent: &Parent<'_>,
        call: &str,
        used: &[(String, String)],
        indent: &str,
    ) {
        for (placeholder, field) in used {
            let _ = writeln!(
                self.report,
                "{indent}  <{placeholder}> from the failure's field {field}"
            );
            if !self.allowances.excuses(Rule::FieldValue, parent.path) {
                self.problems.push(format!(
                    "{}: the step `{call}` takes `<{placeholder}>` from the failure's field {field}, so an agent must substitute the value itself. The failure's recovery can carry the value with Call::value, or an opt-out from `{}` names `{}` with a reason",
                    chain.start,
                    Rule::FieldValue.id(),
                    allow::command_line(self.command, parent.path)
                ));
            }
        }
    }

    /// Returns the words of `call` after the tool's name, with each
    /// placeholder replaced by its value from the start's values, `fields`
    /// or the shared values, and the name of each placeholder that a field
    /// filled with the field's name. It adds the problem and returns `None`
    /// when the call cannot be read, does not start with the tool's name or
    /// has a placeholder without a value.
    fn fill(
        &mut self,
        chain: &Chain<'_>,
        call: &str,
        fields: &[(String, String)],
    ) -> Option<Filled> {
        let tool = self.command.get_name();
        let problem = match split(call) {
            None => format!("the step `{call}` is not a command line that crumbkit quotes"),
            Some(words) if words.first().map(String::as_str) != Some(tool) => {
                format!("the step `{call}` must start with the tool's name, {tool}")
            }
            Some(words) => {
                let shared = self.recoveries.values;
                let mut used: Vec<(String, String)> = Vec::new();
                let mut lookup = |name: &str| {
                    let (value, field) = value(chain.values, fields, shared, name)?;
                    if let Some(field) = field
                        && !used.iter().any(|(seen, _)| seen == name)
                    {
                        used.push((name.to_owned(), field.to_owned()));
                    }
                    Some(value)
                };
                let filled: Result<Vec<String>, String> = words
                    .iter()
                    .skip(1)
                    .map(|word| fill_word(word, &mut lookup))
                    .collect();
                match filled {
                    Ok(filled) => return Some((filled, used)),
                    Err(missing) => format!(
                        "the step `{call}` has the placeholder {missing}, and no value of Failing::values, the fields of the failure or Recoveries::values fills it"
                    ),
                }
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
            Place {
                dir: chain.dir.path(),
                stdin: chain.stdin,
            },
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
        // The first failure is the one the run fails with.
        let Some(failure) = failures::read_failures(self.reader, &output.stdout, &output.stderr)
            .into_iter()
            .next()
        else {
            match self.reader {
                Some(reader) => {
                    let (name, remedy) = reader.name_and_remedy();
                    self.stopped = Some(format!(
                        "{}: {label} ends with {}, and {name} reads no failure from it, so the check cannot follow the tool's recoveries. {remedy}, or the tool adds an opt-out from `{}` that names the root, with a reason",
                        chain.start,
                        output.status,
                        Rule::Recoveries.id()
                    ));
                }
                None => self.problems.push(format!(
                    "{}: {label} ends with {} and prints no failure that the check can read",
                    chain.start, output.status
                )),
            }
            return None;
        };
        let code = failure.code().to_owned();
        let steps = failure.steps().to_vec();
        Some(Outcome::Fails(code, steps, failure.fields().to_vec()))
    }
}
