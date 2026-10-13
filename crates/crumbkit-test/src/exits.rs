//! The exit code check, which a tool's test runs over the exit codes that
//! its guide declares.

use std::process::ExitStatus;

use clap::Command;
use crumbkit::help::{Exit, Guide};
use crumbkit::{Code, Spelling};

use crate::allow::{Allow, Allowances, Rule, command_line};
use crate::tree;

#[cfg(test)]
mod tests;

/// Checks the exit codes that `guide` declares with
/// [`Guide::exits`](crumbkit::help::Guide::exits), and panics, as a failed
/// test assertion does, naming every problem it finds (ADR-0002).
///
/// `<tool> help exit-codes` and `commands list` describe the declared codes,
/// so an agent can tell what a run's exit code means. The check finds:
///
/// - a code from 0 to 2, which crumbkit owns: 0 for success, 1 for a
///   failure and 2 for a refused command line;
/// - a code from 126 to 255, which a shell also reports for its own
///   reasons, such as 127 when it cannot find the tool and 130 when Ctrl-C
///   ends it;
/// - a code that the guide declares more than once;
/// - a meaning that is empty, only whitespace or more than one line;
/// - a command path of a code that names no command below `command`;
/// - a failure code that [`Exit::failures`] lists for two codes, or that
///   belongs to crumbkit, which decides the exit code of its own failures.
///   The problem spells the failure code with [`Spelling::COLONS`].
///
/// A tool can opt out of the rule for codes from 126 with an [`Allow`] of
/// [`Rule::ShellExitCode`] and a stated reason, such as a tool that passes
/// on the code of a child process ([ADR-0036]). The opt-out covers a code
/// when it covers every command that returns the code, so a code that every
/// command returns needs an opt-out with an empty path. The check fails on
/// such an opt-out that names no command or excuses no problem. The rule for
/// crumbkit's codes has no opt-out, because crumbkit's own meaning of each
/// code would contradict the tool's.
///
/// ```
/// use clap::Command;
/// use crumbkit::help::{Exit, Guide};
///
/// const GUIDE: Guide = Guide::new().exits(&[Exit::new(3, "The catalogue was locked.")]);
///
/// crumbkit_test::check_exit_codes(&Command::new("shelf"), &GUIDE, &[]);
/// ```
///
/// # Panics
///
/// Panics when a code or an opt-out breaks a rule above. Each problem in
/// the message starts on a new line and names the code.
///
/// [ADR-0036]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/adoption/0036-opt-outs.md
pub fn check_exit_codes(command: &Command, guide: &Guide, allow: &[Allow]) {
    let mut allowances = Allowances::new(allow, &[Rule::ShellExitCode]);
    let mut problems = Vec::new();
    let tool = command.get_name();
    let exits = guide.get_exits();
    for (index, exit) in exits.iter().enumerate() {
        let code = exit.code();
        if code <= 2 {
            problems.push(format!(
                "the exit code {code} is one of crumbkit's codes 0, 1 and 2, which `{tool} help exit-codes` already describes"
            ));
        }
        if code >= 126 && !excused(&mut allowances, exit.get_commands()) {
            problems.push(format!(
                "the exit code {code} is one that a shell reports for its own reasons, so a caller cannot tell the tool's {code} from the shell's"
            ));
        }
        // Only the second declaration of a code reports it, so a code that
        // three declarations share is one problem.
        let earlier = exits[..index]
            .iter()
            .filter(|earlier| earlier.code() == code)
            .count();
        if earlier == 1 {
            problems.push(format!(
                "the exit code {code} is declared more than once, so a caller cannot tell which meaning applies"
            ));
        }
        let meaning = exit.meaning();
        if meaning.trim().is_empty() {
            problems.push(format!("the exit code {code} has no meaning"));
        } else if meaning.contains(['\n', '\r']) {
            problems.push(format!(
                "the meaning of the exit code {code} is more than one line, and help and `commands list` print it on one"
            ));
        }
        for &failure in exit.get_failures() {
            listed(exits, index, failure, &mut problems);
        }
        for path in exit.get_commands() {
            if tree::find(command, path.iter().copied()).is_none() {
                problems.push(format!(
                    "the exit code {code} names `{}`, which is no command",
                    command_line(command, &path.join(" "))
                ));
            }
        }
    }
    allowances.report(command, "exit code rules", problems);
}

/// Pushes a problem when `failure`, which the exit at `index` of `exits`
/// lists, belongs to crumbkit, or when an earlier exit lists it. Only the
/// second exit that lists a code reports it, so a code that three exits
/// list is one problem.
fn listed(exits: &[Exit], index: usize, failure: &Code, problems: &mut Vec<String>) {
    let code = exits[index].code();
    let text = Spelling::COLONS.spell(failure);
    if failure.component() == "crumbkit" {
        problems.push(format!(
            "the exit code {code} lists the failure code {text}, which belongs to crumbkit, and crumbkit decides the exit code of its own failures"
        ));
    }
    let mut earlier = exits[..index]
        .iter()
        .filter(|exit| exit.get_failures().contains(&failure));
    if let Some(first) = earlier.next()
        && earlier.next().is_none()
    {
        problems.push(format!(
            "the exit code {code} lists the failure code {text}, which the exit code {} lists already, so a run that fails with it cannot exit with both",
            first.code()
        ));
    }
}

/// Returns the problem when a failed run whose first failure the tool
/// spells `printed` with `spelling` ends with `status`, which the run
/// names in the problem after its own text, such as a command line.
///
/// The run must exit with the code of the exit in `exits` that lists the
/// failure's code. A run whose failure no exit lists must not exit with
/// the code of an exit that lists failures, because a caller reads that
/// code as one of those failures. It may exit with any other code, so a
/// tool can pass on the code of a child process.
pub(crate) fn exit_problem(
    exits: &[Exit],
    spelling: Spelling,
    printed: &str,
    status: ExitStatus,
) -> Option<String> {
    let mapped = exits.iter().find(|exit| {
        exit.get_failures()
            .iter()
            .any(|&code| spelling.spell(code) == printed)
    });
    match (mapped, status.code()) {
        (Some(exit), Some(code)) if code == i32::from(exit.code()) => None,
        (Some(exit), _) => Some(format!(
            "fails with {printed}, which the guide's exit code {} lists, and it ends with {status}",
            exit.code()
        )),
        (None, Some(code))
            if exits
                .iter()
                .any(|exit| i32::from(exit.code()) == code && !exit.get_failures().is_empty()) =>
        {
            Some(format!(
                "fails with {printed}, which no exit code of the guide lists, and it exits with {code}, which the guide declares for the failure codes that it lists"
            ))
        }
        (None, _) => None,
    }
}

/// Reports whether an opt-out from [`Rule::ShellExitCode`] covers each
/// command path in `paths`, or the root when `paths` is empty, because a
/// code without paths can come from every command. It asks about every
/// path, so each opt-out that covers one is recorded as used.
fn excused(allowances: &mut Allowances<'_>, paths: &[&[&str]]) -> bool {
    if paths.is_empty() {
        return allowances.excuses(Rule::ShellExitCode, "");
    }
    let mut all = true;
    for path in paths {
        all &= allowances.excuses(Rule::ShellExitCode, &path.join(" "));
    }
    all
}
