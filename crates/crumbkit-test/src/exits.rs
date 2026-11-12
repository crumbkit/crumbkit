//! The exit code check, which a tool's test runs over the exit codes that
//! its tool record declares.

use std::process::ExitStatus;

use clap::Command;
use crumbkit::tool::Exit;
use crumbkit::{Code, Spelling, Tool};

use crate::allow::{Allow, Allowances, Rule, command_line};
use crate::tree;

#[cfg(test)]
mod tests;

/// Checks the exit codes that `tool` declares with
/// [`Failures::exits`](crumbkit::tool::Failures::exits), and panics, as a
/// failed test assertion does, naming every problem it finds (ADR-0002).
///
/// `<tool> help exit-codes` and `commands list` describe the declared codes,
/// so an agent can tell what a run's exit code means. The check finds:
///
/// - the code 0, which means that the run succeeded;
/// - the code 1 or 2, which already mean crumbkit's default failure and a
///   refused command line;
/// - a single code from 126 to 128 or from 160 to 255, which a shell also
///   reports for its own reasons, such as 127 when it cannot find the tool.
///   A code from 129 to 159 passes, because a shell reports a run that a
///   signal ended as 128 plus the signal's number, such as 130 after
///   Ctrl-C, and a tool that catches the signal exits with the same code;
/// - a range from [`Exit::range`](crumbkit::tool::Exit::range) that starts
///   below 3, because 0, 1 and 2 keep their meanings, or that ends before
///   it starts. A range may run up to 255, because its codes come from
///   another program;
/// - a range that lists failure codes, because a range names no single
///   code for a run that fails with one to exit with;
/// - a code that the tool record declares more than once, and a range that
///   holds a code that another declaration holds;
/// - a meaning that is empty, only whitespace or more than one line;
/// - a command path of a code that names no command below `command`;
/// - a failure code that [`Exit::failures`](crumbkit::tool::Exit::failures)
///   lists for two codes.
///
/// The problems spell failure codes with the tool's name in crumbkit's own
/// codes, as the tool prints them. An exit may
/// list one of crumbkit's failure codes, such as
/// [`CLI_USAGE`](crumbkit::codes::CLI_USAGE), to give that failure the tool's
/// own exit code.
///
/// A tool can opt out of the rule for single codes from 126 with an
/// [`Allow`] of [`Rule::ShellExitCode`], and of the rule for 1 and 2 with an [`Allow`] of
/// [`Rule::SharedExit`], each with a stated reason, such as a tool that
/// passes on the code of a child process ([opt-outs docs]). An opt-out covers a
/// code when it covers every command that returns the code, so a code that
/// every command returns needs an opt-out with an empty path. The check
/// fails on such an opt-out that names no command or excuses no problem.
/// The rule for 0 has no opt-out, because a caller reads 0 as success, and
/// the rules for ranges and overlaps have none, because a range already
/// spans the codes that the shell reports, and a code with two meanings
/// cannot be read.
///
/// ```
/// use clap::Command;
/// use crumbkit::Tool;
/// use crumbkit::tool::{Exit, Failures};
///
/// const TOOL: Tool = Tool::new()
///     .failures(Failures::new().exits(&[Exit::new(3, "The catalogue was locked.")]));
///
/// crumbkit_test::check_exit_codes(&Command::new("shelf"), &TOOL, &[]);
/// ```
///
/// # Panics
///
/// Panics when a code or an opt-out breaks a rule above. Each
/// problem in the message starts on a new line and names the code.
///
/// [opt-outs docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/adoption/opt-outs.md
pub fn check_exit_codes(command: &Command, tool: &Tool, allow: &[Allow]) {
    let mut allowances = Allowances::new(allow, &[Rule::ShellExitCode, Rule::SharedExit]);
    let mut problems = Vec::new();
    let spelling = Spelling::COLONS.for_tool(tool.tool_name(command));
    let exits = tool.get_failures().get_exits();
    for (index, exit) in exits.iter().enumerate() {
        let code = exit.code();
        let name = name(exit);
        if !single(exit) {
            ranged(exit, &name, &mut problems);
        } else if code == 0 {
            problems.push(
                "the exit code 0 means that the run succeeded, so a caller cannot tell a failure from a success"
                    .to_owned(),
            );
        }
        if single(exit)
            && (code == 1 || code == 2)
            && !excused(&mut allowances, Rule::SharedExit, exit.get_commands())
        {
            let meaning = if code == 1 {
                "crumbkit's default failure ended the run"
            } else {
                "crumbkit refused the command line"
            };
            problems.push(format!(
                "the exit code {code} already means that {meaning}, so it has two meanings"
            ));
        }
        if single(exit)
            && is_shell_code(code)
            && !excused(&mut allowances, Rule::ShellExitCode, exit.get_commands())
        {
            problems.push(format!(
                "the exit code {code} is one that a shell reports for its own reasons, so a caller cannot tell the tool's {code} from the shell's"
            ));
        }
        overlaps(&exits[..index], exit, &name, &mut problems);
        // Only the second declaration of a single code reports it, so a
        // code that three declarations share is one problem.
        let earlier = exits[..index]
            .iter()
            .filter(|earlier| single(exit) && single(earlier) && earlier.code() == code)
            .count();
        if earlier == 1 {
            problems.push(format!(
                "the exit code {code} is declared more than once, so a caller cannot tell which meaning applies"
            ));
        }
        let meaning = exit.meaning();
        if meaning.trim().is_empty() {
            problems.push(format!("{name} has no meaning"));
        } else if meaning.contains(['\n', '\r']) {
            problems.push(format!(
                "the meaning of {name} is more than one line, and help and `commands list` print it on one"
            ));
        }
        for &failure in exit.get_failures() {
            listed(tool, spelling, index, failure, &mut problems);
        }
        for path in exit.get_commands() {
            if tree::find(command, path.iter().copied()).is_none() {
                problems.push(format!(
                    "{name} names `{}`, which is no command",
                    command_line(command, &path.join(" "))
                ));
            }
        }
    }
    allowances.report(command, "exit code rules", problems);
}

/// Returns whether `exit` declares one code, as [`Exit::new`] does, or is a
/// range whose first and last codes are the same.
fn single(exit: &Exit) -> bool {
    exit.codes().start() == exit.codes().end()
}

/// Returns whether a shell reports the single code `code` for its own
/// reasons: 126 to 128, and 160 and above. The codes from 129 to 159 are
/// 128 plus a signal's number, which a tool that catches the signal also
/// exits with.
const fn is_shell_code(code: u8) -> bool {
    matches!(code, 126..=128 | 160..=u8::MAX)
}

/// Returns the words that name `exit` in a problem: `the exit code 124` for
/// a single code, and `the exit range 124-125` for a range.
fn name(exit: &Exit) -> String {
    let codes = exit.codes();
    if single(exit) {
        format!("the exit code {}", codes.start())
    } else {
        format!("the exit range {}-{}", codes.start(), codes.end())
    }
}

/// Pushes a problem when `exit`, a range named `name`, starts below 3, ends
/// before it starts, or lists failure codes.
fn ranged(exit: &Exit, name: &str, problems: &mut Vec<String>) {
    let codes = exit.codes();
    if *codes.start() < 3 {
        problems.push(format!(
            "{name} starts below 3, and 0, 1 and 2 already mean success, crumbkit's default failure and a refused command line"
        ));
    }
    if codes.start() > codes.end() {
        problems.push(format!("{name} ends before it starts, so it holds no code"));
    }
    if !exit.get_failures().is_empty() {
        problems.push(format!(
            "{name} lists failure codes, and a run that fails with one exits with 1, because a range names no single code to exit with"
        ));
    }
}

/// Pushes a problem for each of `earlier` that holds a code that `exit`,
/// named `name`, also holds, when one of the two is a range. Only the later
/// of two declarations reports their overlap, so each pair is one problem.
/// Two single codes that are the same are the problem of a code declared
/// more than once, which [`check_exit_codes`] reports.
fn overlaps(earlier: &[Exit], exit: &Exit, name: &str, problems: &mut Vec<String>) {
    let codes = exit.codes();
    for other in earlier {
        if single(exit) && single(other) {
            continue;
        }
        let theirs = other.codes();
        if codes.start().max(theirs.start()) <= codes.end().min(theirs.end()) {
            problems.push(format!(
                "{name} overlaps {}, so a caller cannot tell which meaning applies",
                self::name(other)
            ));
        }
    }
}

/// Pushes a problem when an earlier exit of `tool` than the one at `index`
/// lists `failure`. Only the second exit that lists a code reports it, so a
/// code that three exits list is one problem. The problem spells the code
/// with `spelling`.
fn listed(
    tool: &Tool,
    spelling: Spelling,
    index: usize,
    failure: &'static Code<'static>,
    problems: &mut Vec<String>,
) {
    let exits = tool.get_failures().get_exits();
    let code = exits[index].code();
    let mut earlier = exits[..index]
        .iter()
        .filter(|exit| exit.get_failures().contains(&failure));
    let Some(first) = earlier.next() else {
        return;
    };
    if earlier.next().is_none() {
        problems.push(format!(
            "the exit code {code} lists the failure code {}, which the exit code {} lists already, so a run that fails with it cannot exit with both",
            spelling.spell(failure),
            first.code()
        ));
    }
}

/// Returns the problem when a failed run of the command at `path`, whose
/// first failure the tool spells `printed` with `spelling`, ends with
/// `status`. The problem starts with a verb, and the caller puts the run's
/// name before it, such as its command line.
///
/// The run must exit as [`Tool::failure_exit`] decides for its first
/// failure (ADR-0037):
///
/// - with the code of the exit of `tool` that lists the failure's code;
/// - with the code that the tool record gives a refused command line, 2 unless
///   an exit lists `<tool>::cli::usage`, for `<tool>::cli::usage`;
/// - with 1 for any other failure, unless the tool record's leaf at `path`
///   declares with [`Leaf::passes_on_exit`](crumbkit::help::Leaf::passes_on_exit)
///   that the command passes on a program's exit code, which may be any
///   code but 0.
///
/// A run whose failure no exit lists never exits with the code of an exit
/// that lists failures, because a caller reads that code as one of those
/// failures.
pub(crate) fn exit_problem(
    tool: &Tool,
    spelling: Spelling,
    path: &[String],
    printed: &str,
    status: ExitStatus,
) -> Option<String> {
    let path: Vec<&str> = path.iter().map(String::as_str).collect();
    let is = |code: &Code<'_>| spelling.spell(code) == printed;
    let exits = tool.get_failures().get_exits();
    // The failures of a range never decide a run's exit, so they count as
    // unlisted.
    let listed = exits
        .iter()
        .filter(|exit| single(exit))
        .flat_map(|exit| exit.get_failures().iter().copied())
        .find(|&code| is(code));
    let code = status.code();
    if let Some(listed) = listed {
        let expected = tool.failure_exit(*listed);
        return (code != Some(i32::from(expected))).then(|| {
            format!(
                "fails with {printed}, which the tool record's exit code {expected} lists, and it ends with {status}"
            )
        });
    }
    if is(&crumbkit::codes::CLI_USAGE) {
        let expected = tool.failure_exit(crumbkit::codes::CLI_USAGE);
        return (code != Some(i32::from(expected))).then(|| {
            format!(
                "fails with {printed}, which refuses the command line, and it ends with {status}, where the tool record gives a refused command line the exit code {expected}"
            )
        });
    }
    if let Some(code) = code
        && exits.iter().any(|exit| {
            single(exit) && i32::from(exit.code()) == code && !exit.get_failures().is_empty()
        })
    {
        return Some(format!(
            "fails with {printed}, which no exit code of the tool record lists, and it exits with {code}, which the tool record declares for the failure codes that it lists"
        ));
    }
    let passes = tool
        .get_help()
        .get_leaves()
        .iter()
        .any(|leaf| leaf.path() == path.as_slice() && leaf.get_passes_on_exit().is_some());
    // A failed run never exits with 0, because the check reads a failure
    // only from a run that did not succeed.
    if code == Some(1) || (passes && code.is_some()) {
        return None;
    }
    Some(format!(
        "fails with {printed}, which no exit code of the tool record lists, and it ends with {status}, where such a failure exits with code 1. A command that passes on the exit code of a program it runs declares it with `Leaf::passes_on_exit`"
    ))
}

/// Reports whether an opt-out from `rule` covers each command path in
/// `paths`, or the root when `paths` is empty, because a code without paths
/// can come from every command. It asks about every path, so each opt-out
/// that covers one is recorded as used.
fn excused(allowances: &mut Allowances<'_>, rule: Rule, paths: &[&[&str]]) -> bool {
    if paths.is_empty() {
        return allowances.excuses(rule, "");
    }
    let mut all = true;
    for path in paths {
        all &= allowances.excuses(rule, &path.join(" "));
    }
    all
}
