//! The walk of the recovery check through the chains of recoveries that
//! each start prints.

use std::fmt::Write as _;

use clap::Command;
use crumbkit::Spelling;
use crumbkit::form::{ReadFailure, Step};
use crumbkit::help::Guide;

use super::words::split;
use super::{Failing, Recoveries};
use crate::allow::{self, Allowances, Rule};
use crate::examples::is_crumbkit_warning;
use crate::exits;
use crate::failures::{self, Reader};
use crate::guide::command_line;
use crate::runs::{self, Place, Ran, TIMEOUT, WorkDir, Xdg, env_note, with_json};
use fill::{fill_word, path_below, value};

mod all;
pub(super) mod fill;
pub(super) mod known;

/// What the check has found so far: its settings, the problems, the report
/// and the codes that the chains provoked.
pub(super) struct Walk<'a> {
    /// The check's settings.
    pub(super) recoveries: &'a Recoveries<'a>,
    /// The check's spelling, which spells crumbkit's own codes with the
    /// tool's name (ADR-0079).
    pub(super) spelling: Spelling,
    /// The tool's command tree.
    pub(super) command: &'a Command,
    /// The tool's declared codes, spelled.
    pub(super) declared: Vec<String>,
    /// The spelled codes whose recoveries the check follows by name
    /// ([`Walk::is_known`]).
    pub(super) known: Vec<String>,
    /// The component of each declared code with the text that the
    /// spelling writes before the area of each of its codes, such as
    /// `shelf::`.
    pub(super) components: Vec<(String, String)>,
    /// crumbkit's usage code, as the tool spells it.
    pub(super) usage: String,
    /// The opt-outs of `recoveries/follow`, `recoveries/same-code`,
    /// `recoveries/field-value` and `recoveries/next-step`.
    pub(super) allowances: Allowances<'a>,
    /// The problems found, one line each.
    pub(super) problems: Vec<String>,
    /// The text of the golden report.
    pub(super) report: String,
    /// The spelled codes that a start or a step failed with.
    pub(super) provoked: Vec<String>,
    /// The number of failures whose codes the check did not know.
    pub(super) skipped: usize,
    /// The tool's failure reader, which reads each failed run first.
    pub(super) reader: Option<Reader<'a>>,
    /// The tool's guide, whose exits and renames give the exit code of a
    /// failed run.
    pub(super) guide: &'a Guide,
    /// The problem that stopped the walk: a failed run whose failure the
    /// tool's reader does not read, so no later run can be
    /// followed.
    pub(super) stopped: Option<String>,
}

/// The failure that offers a chain's next steps.
#[derive(Clone, Copy)]
struct Parent<'p> {
    /// The codes of the failure and of every earlier failure of its chain,
    /// the start's first, as the tool spells them.
    codes: &'p [String],
    /// The failure's fields that fill the steps' placeholders, each a name
    /// and its value's text.
    fields: &'p [(String, String)],
    /// The failure's list fields whose every item fills a placeholder, each
    /// a name and its items' text, which a `RunAll` step runs with.
    lists: &'p [(String, Vec<String>)],
    /// The number of values that the step being followed ran with, when it
    /// is a `RunAll` step. A failure of that run with the same code and
    /// fewer values left is progress.
    ran_all: Option<usize>,
    /// The path below the root of the command that failed, which an
    /// opt-out from `recoveries/field-value` names.
    path: &'p str,
}

/// The words of a filled call after the tool's name, and the name of each
/// placeholder that a field filled with the field's name.
type Filled = (Vec<String>, Vec<(String, String)>);

/// How a run ended: with code 0, or with the failures that it printed.
enum Outcome {
    /// The run exited with code 0.
    Succeeds,
    /// The run failed and printed `failures`, the first failure first, each
    /// with its code, its steps and its fields. `warned` says whether it
    /// wrote a `warning: <tool> left out` line, which a debug build writes for a
    /// recovery call that does not render.
    Fails {
        failures: Vec<ReadFailure>,
        warned: bool,
    },
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
    /// The variables that every run sets or removes.
    env: &'c [(&'c str, Option<&'c str>)],
    /// The words of each command line that the chain ran.
    ran: Vec<Vec<String>>,
}

impl Walk<'_> {
    /// Runs `start` and follows the steps of its failure, unless an opt-out
    /// from `recoveries/follow` covers its command.
    pub(super) fn start(&mut self, start: &Failing<'_>) {
        let tool = self.command.get_name();
        let line = command_line(tool, start.words) + &env_note(start.own_env());
        if self.stopped.is_some() {
            return;
        }
        if self
            .allowances
            .excuses(Rule::Recoveries, &path_below(self.command, start.words))
        {
            return;
        }
        let code = self.spelling.spell(start.code);
        // A start names its code in Rust, so crumbkit's own codes are those
        // whose component is `crumbkit`.
        let owned = start.code.component() == "crumbkit";
        if !owned && !self.known.contains(&code) {
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
            env: start.own_env(),
            ran: Vec::new(),
        };
        let words: Vec<String> = start.words.iter().map(|&word| word.to_owned()).collect();
        match self.run(&mut chain, "the start", &words) {
            Some(Outcome::Succeeds) => self.problems.push(format!(
                "{}: the start must fail with {code}, and it exits with code 0",
                chain.start
            )),
            Some(Outcome::Fails { failures, warned }) => {
                if failures.iter().any(|failure| failure.code() == code) {
                    let path = path_below(self.command, start.words);
                    let mark = chain.ran.len();
                    for failure in &failures {
                        chain.ran.truncate(mark);
                        if !warned {
                            self.next_step(&chain, "the start", &path, failure);
                        }
                        self.branch(&mut chain, None, "the start", &path, failure, 0);
                    }
                } else {
                    let failed: Vec<&str> = failures.iter().map(ReadFailure::code).collect();
                    self.provoked
                        .extend(failed.iter().map(|&failed| failed.to_owned()));
                    self.problems.push(format!(
                        "{}: the start must fail with {code}, and it fails with {}",
                        chain.start,
                        failed.join(", ")
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
            let (call, values) = match step {
                Step::Run(call) => (call, None),
                Step::RunAll { call, field } => {
                    let list = parent.lists.iter().find(|(name, _)| name == field);
                    let Some((_, values)) = list.filter(|(_, values)| !values.is_empty()) else {
                        self.problems.push(format!(
                            "{}: the step `{call}` runs with every value of the failure's field {field}, which is not a list that holds a value",
                            chain.start
                        ));
                        continue;
                    };
                    (call, Some(values.as_slice()))
                }
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
            let Some((words, used)) = self.fill(chain, call, parent.fields, values) else {
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
                Some(Outcome::Fails { failures, warned }) => {
                    // Each failure starts its own branch, which may run a
                    // command line that a sibling branch ran.
                    let mark = chain.ran.len();
                    for failure in &failures {
                        chain.ran.truncate(mark);
                        if !warned {
                            self.next_step(chain, &label, &path, failure);
                        }
                        let ran = Parent {
                            ran_all: values.map(<[String]>::len),
                            ..*parent
                        };
                        self.branch(chain, Some(&ran), &label, &path, failure, depth);
                    }
                }
                None => {}
            }
        }
    }

    /// Follows `failure`, one failure that the run `label` of the command at
    /// `path` printed, as a branch of `chain` at `depth`, the start's
    /// failures at 0. `parent` is the failure whose step ran, or `None` for
    /// the start. It reports a refusal by the parser, an undeclared code and
    /// a code that an earlier failure of the chain had, and then follows the
    /// failure's steps.
    fn branch(
        &mut self,
        chain: &mut Chain<'_>,
        parent: Option<&Parent<'_>>,
        label: &str,
        path: &str,
        failure: &ReadFailure,
        depth: u8,
    ) {
        let indent = "  ".repeat(usize::from(depth) + 1);
        let code = failure.code();
        self.provoked.push(code.to_owned());
        // A start may fail with the usage code on purpose, so only a step
        // that the parser refuses is a problem.
        if code == self.usage && parent.is_some() {
            let _ = writeln!(self.report, "{indent}refused by the parser with {code}");
            self.problems.push(format!(
                "{}: the parser refuses {label} with {code}",
                chain.start
            ));
            return;
        }
        if !self.is_known(code) {
            self.unknown(chain, label, code, &indent);
            return;
        }
        let earlier = parent.map_or(&[][..], |parent| parent.codes);
        // A run with every value of a list field that leaves fewer values
        // out makes progress, because a batch always answers its first key.
        let progress = parent
            .and_then(|parent| parent.ran_all)
            .is_some_and(|ran| all::fewer_left(failure, ran));
        if earlier.iter().any(|seen| seen == code)
            && !progress
            && !self.allowances.excuses(Rule::SameCode, path)
        {
            self.problems.push(format!(
                "{}: {label} fails again with {code}, so an agent that follows the recovery learns nothing new",
                chain.start
            ));
        }
        let _ = writeln!(self.report, "{indent}fails with {code}");
        let mut codes = earlier.to_vec();
        codes.push(code.to_owned());
        let fields: Vec<(String, String)> = failure
            .fields()
            .iter()
            .filter_map(|(name, value)| Some((name.clone(), failures::placeholder_text(value)?)))
            .collect();
        let lists = all::lists(failure);
        let next = Parent {
            codes: &codes,
            fields: &fields,
            lists: &lists,
            ran_all: None,
            path,
        };
        self.follow(chain, &next, failure.steps(), depth + 1);
    }

    /// Adds a problem when `failure`, which the run `label` of the command
    /// at `path` printed, has one of the tool's declared codes and offers no
    /// `run`, `retry` or `hint` step, unless an opt-out from
    /// `recoveries/next-step` covers the command. A `none` step with a
    /// reason is still no next step, because the opt-out is where a tool
    /// declares that exception.
    fn next_step(&mut self, chain: &Chain<'_>, label: &str, path: &str, failure: &ReadFailure) {
        let code = failure.code();
        let offers = failure.steps().iter().any(|step| {
            matches!(
                step,
                Step::Run(_) | Step::RunAll { .. } | Step::Retry | Step::Hint(_)
            )
        });
        if offers
            || !self.declared.iter().any(|declared| declared == code)
            || self.allowances.excuses(Rule::NextStep, path)
        {
            return;
        }
        self.problems.push(format!(
            "{}: {label} fails with {code}, whose failure offers no run, retry or hint step, so an agent has no next step. The failure can offer a recovery, or an opt-out from `{}` names `{}` with a reason",
            chain.start,
            Rule::NextStep.id(),
            allow::command_line(self.command, path)
        ));
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
    ///
    /// `values` holds every value of the list field that a `RunAll` step
    /// runs its call with. They replace the first word of the call that is
    /// one placeholder, such as `<ID>...`, as one word each, before any other
    /// source, and they are no finding of `recoveries/field-value`, because
    /// the step names the field they come from. crumbkit refuses to render a
    /// call from `Call::all` with another placeholder, so that word is the
    /// argument that takes the values.
    fn fill(
        &mut self,
        chain: &Chain<'_>,
        call: &str,
        fields: &[(String, String)],
        values: Option<&[String]>,
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
                let mut values = values;
                let mut filled = Vec::with_capacity(words.len());
                let mut missing = None;
                for word in words.iter().skip(1) {
                    match values {
                        Some(all) if all::is_placeholder(word) => {
                            filled.extend(all.iter().cloned());
                            values = None;
                        }
                        _ => match fill_word(word, &mut lookup) {
                            Ok(word) => filled.push(word),
                            Err(placeholder) => {
                                missing = Some(placeholder);
                                break;
                            }
                        },
                    }
                }
                match missing {
                    None => return Some((filled, used)),
                    Some(missing) => format!(
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
        let path = crumbkit::command_path(self.command, self.guide, &refs);
        let result = runs::run(
            self.recoveries.examples.binary(),
            self.recoveries.examples.variables(),
            &chain.xdg,
            &refs,
            Place {
                dir: chain.dir.path(),
                stdin: chain.stdin,
                env: chain.env,
            },
            TIMEOUT,
        );
        chain.ran.push(args);
        let output = match result {
            Ok(Ran::Exited(output)) => output,
            Ok(Ran::Stopped) => {
                self.problems.push(format!(
                    "{}: {label} did not end within {} seconds, so the check stopped it",
                    chain.start,
                    TIMEOUT.as_secs()
                ));
                return None;
            }
            Err(problem) => {
                self.problems.push(format!("{}: {problem}", chain.start));
                return None;
            }
        };
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tool = self.guide.tool_name(self.command);
        let mut warned = false;
        for warning in stderr
            .lines()
            .filter(|line| is_crumbkit_warning(line, tool))
        {
            warned = true;
            self.problems.push(format!(
                "{}: {label} writes a warning to standard error: {warning}",
                chain.start
            ));
        }
        if output.status.success() {
            return Some(Outcome::Succeeds);
        }
        let failures = failures::read_failures(tool, self.reader, &output.stdout, &output.stderr);
        if failures.is_empty() {
            match self.reader {
                Some(_) => {
                    self.stopped = Some(format!(
                        "{}: {label} ends with {}, and the tool's FailureReader reads no failure from it, so the check cannot follow the tool's recoveries. The tool's FailureReader reads the failure, or the tool adds an opt-out from `{}` that names the root, with a reason",
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
        }
        // The first failure of a run decides its exit code (ADR-0037).
        let first = failures.first().map(ReadFailure::code);
        if let Some(problem) = first.and_then(|first| {
            let spelling = self.spelling;
            exits::exit_problem(self.guide, spelling, &path, first, output.status)
        }) {
            self.problems
                .push(format!("{}: {label} {problem}", chain.start));
        }
        Some(Outcome::Fails { failures, warned })
    }
}
