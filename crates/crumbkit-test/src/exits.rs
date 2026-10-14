//! The exit code check, which a tool's test runs over the exit codes that
//! its guide declares.

use std::process::ExitStatus;

use clap::Command;
use crumbkit::help::{Guide, Rename};
use crumbkit::{Code, Spelling};

use crate::allow::{Allow, Allowances, Rule, command_line};
use crate::tree;

#[cfg(test)]
mod tests;

/// Checks the exit codes that `guide` declares with
/// [`Guide::exits`](crumbkit::help::Guide::exits) and the renames it
/// declares with [`Guide::codes`](crumbkit::help::Guide::codes), and
/// panics, as a failed test assertion does, naming every problem it finds
/// (ADR-0002).
///
/// `<tool> help exit-codes` and `commands list` describe the declared codes,
/// so an agent can tell what a run's exit code means. The check finds:
///
/// - the code 0, which means that the run succeeded;
/// - the code 1 or 2, which already mean crumbkit's default failure and a
///   refused command line;
/// - a code from 126 to 255, which a shell also reports for its own
///   reasons, such as 127 when it cannot find the tool and 130 when Ctrl-C
///   ends it;
/// - a code that the guide declares more than once;
/// - a meaning that is empty, only whitespace or more than one line;
/// - a command path of a code that names no command below `command`;
/// - a failure code that [`Exit::failures`](crumbkit::help::Exit::failures) lists for two codes, where one
///   of crumbkit's codes counts as the tool's code that the guide prints in
///   its place;
/// - one of crumbkit's codes that the guide renames twice, and a code of
///   the tool that two renames print.
///
/// The problems spell failure codes with [`Spelling::COLONS`]. An exit may
/// list one of crumbkit's failure codes, such as
/// [`CLI_USAGE`](crumbkit::codes::CLI_USAGE), to give that failure the
/// tool's own exit code.
///
/// A tool can opt out of the rule for codes from 126 with an [`Allow`] of
/// [`Rule::ShellExitCode`], and of the rule for 1 and 2 with an [`Allow`] of
/// [`Rule::SharedExit`], each with a stated reason, such as a tool that
/// passes on the code of a child process ([ADR-0036]). An opt-out covers a
/// code when it covers every command that returns the code, so a code that
/// every command returns needs an opt-out with an empty path. The check
/// fails on such an opt-out that names no command or excuses no problem.
/// The rule for 0 has no opt-out, because a caller reads 0 as success.
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
/// Panics when a code, a rename or an opt-out breaks a rule above. Each
/// problem in the message starts on a new line and names the code.
///
/// [ADR-0036]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/adoption/0036-opt-outs.md
pub fn check_exit_codes(command: &Command, guide: &Guide, allow: &[Allow]) {
    let mut allowances = Allowances::new(allow, &[Rule::ShellExitCode, Rule::SharedExit]);
    let mut problems = Vec::new();
    let exits = guide.get_exits();
    for (index, exit) in exits.iter().enumerate() {
        let code = exit.code();
        if code == 0 {
            problems.push(
                "the exit code 0 means that the run succeeded, so a caller cannot tell a failure from a success"
                    .to_owned(),
            );
        }
        if (code == 1 || code == 2)
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
        if code >= 126 && !excused(&mut allowances, Rule::ShellExitCode, exit.get_commands()) {
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
            listed(guide, index, failure, &mut problems);
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
    renamed(guide.get_codes(), &mut problems);
    allowances.report(command, "exit code rules", problems);
}

/// Pushes a problem when an earlier exit of `guide` than the one at
/// `index` lists `failure`, or a code that the guide prints as the same
/// code. Only the second exit that lists a code reports it, so a code that
/// three exits list is one problem.
fn listed(guide: &Guide, index: usize, failure: &'static Code, problems: &mut Vec<String>) {
    let exits = guide.get_exits();
    let code = exits[index].code();
    let printed = guide.printed(failure);
    let text = Spelling::COLONS.spell(printed);
    let mut earlier = exits[..index].iter().filter(|exit| {
        exit.get_failures()
            .iter()
            .any(|&listed| guide.printed(listed) == printed)
    });
    if let Some(first) = earlier.next()
        && earlier.next().is_none()
    {
        problems.push(format!(
            "the exit code {code} lists the failure code {text}, which the exit code {} lists already, so a run that fails with it cannot exit with both",
            first.code()
        ));
    }
}

/// Pushes a problem for each rename in `renames` whose crumbkit code an
/// earlier rename names, because a run prints only the first, and for each
/// rename that prints the code of an earlier rename of another crumbkit
/// code, because a caller cannot tell the two failures apart.
fn renamed(renames: &[Rename], problems: &mut Vec<String>) {
    for (index, rename) in renames.iter().enumerate() {
        let crumbkit = Spelling::COLONS.spell(rename.crumbkit());
        let tool = Spelling::COLONS.spell(rename.tool());
        let earlier = &renames[..index];
        if let Some(first) = earlier
            .iter()
            .find(|earlier| earlier.crumbkit() == rename.crumbkit())
        {
            problems.push(format!(
                "the guide renames {crumbkit} twice, and a run prints only the first rename, {}",
                Spelling::COLONS.spell(first.tool())
            ));
        } else if let Some(first) = earlier
            .iter()
            .find(|earlier| earlier.tool() == rename.tool())
        {
            problems.push(format!(
                "the guide renames {crumbkit} to {tool}, which it gives {} already, so a caller cannot tell the two failures apart",
                Spelling::COLONS.spell(first.crumbkit())
            ));
        }
    }
}

/// crumbkit's code of a `--json` that a command whose child has no JSON
/// form refuses. crumbkit-child declares it, and crumbkit-test does not
/// depend on crumbkit-child, so the check names it here.
const CHILD_NO_JSON: Code = Code::new("crumbkit", "child", "no_json");

/// Returns the problem when a failed run of the command at `path`, whose
/// first failure the tool spells `printed` with `spelling`, ends with
/// `status`. The problem starts with a verb, and the caller puts the run's
/// name before it, such as its command line.
///
/// The run must exit as [`Guide::failure_exit`] decides for its first
/// failure (ADR-0037):
///
/// - with the code of the exit of `guide` that lists the failure's code,
///   or one of crumbkit's codes that the guide prints as it;
/// - with the code that the guide gives a refused command line, 2 unless an
///   exit lists `crumbkit::cli::usage`, for `crumbkit::cli::usage` and
///   `crumbkit::child::no_json`;
/// - with 1 for any other failure, unless the guide's leaf at `path`
///   declares with [`Leaf::passes_on_exit`](crumbkit::help::Leaf::passes_on_exit)
///   that the command passes on a program's exit code, which may be any
///   code but 0.
///
/// A run whose failure no exit lists never exits with the code of an exit
/// that lists failures, because a caller reads that code as one of those
/// failures.
pub(crate) fn exit_problem(
    guide: &Guide,
    spelling: Spelling,
    path: &[String],
    printed: &str,
    status: ExitStatus,
) -> Option<String> {
    let is = |code: &Code| spelling.spell(guide.printed(code)) == printed;
    let exits = guide.get_exits();
    let listed = exits
        .iter()
        .flat_map(|exit| exit.get_failures().iter().copied())
        .find(|&code| is(code));
    let code = status.code();
    if let Some(listed) = listed {
        let expected = guide.failure_exit(listed);
        return (code != Some(i32::from(expected))).then(|| {
            format!(
                "fails with {printed}, which the guide's exit code {expected} lists, and it ends with {status}"
            )
        });
    }
    if is(&crumbkit::codes::CLI_USAGE) || is(&CHILD_NO_JSON) {
        let expected = guide.failure_exit(&crumbkit::codes::CLI_USAGE);
        return (code != Some(i32::from(expected))).then(|| {
            format!(
                "fails with {printed}, which refuses the command line, and it ends with {status}, where the guide gives a refused command line the exit code {expected}"
            )
        });
    }
    if let Some(code) = code
        && exits
            .iter()
            .any(|exit| i32::from(exit.code()) == code && !exit.get_failures().is_empty())
    {
        return Some(format!(
            "fails with {printed}, which no exit code of the guide lists, and it exits with {code}, which the guide declares for the failure codes that it lists"
        ));
    }
    let passes = guide
        .get_leaves()
        .iter()
        .any(|leaf| leaf.path().iter().eq(path.iter()) && leaf.get_passes_on_exit().is_some());
    // A failed run never exits with 0, because the check reads a failure
    // only from a run that did not succeed.
    if code == Some(1) || (passes && code.is_some()) {
        return None;
    }
    Some(format!(
        "fails with {printed}, which no exit code of the guide lists, and it ends with {status}, where such a failure exits with code 1. A command that passes on the exit code of a program it runs declares it with `Leaf::passes_on_exit`"
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
