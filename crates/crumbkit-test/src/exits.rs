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

/// Returns the problem when a failed run whose first failure the tool
/// spells `printed` with `spelling` ends with `status`, which the run
/// names in the problem after its own text, such as a command line.
///
/// The run must exit with the code of the exit of `guide` that lists the
/// failure's code, or one of crumbkit's codes that the guide prints as it.
/// A run whose failure no exit lists must not exit with the code of an
/// exit that lists failures, because a caller reads that code as one of
/// those failures. It may exit with any other code, so a tool can pass on
/// the code of a child process.
pub(crate) fn exit_problem(
    guide: &Guide,
    spelling: Spelling,
    printed: &str,
    status: ExitStatus,
) -> Option<String> {
    let exits = guide.get_exits();
    let mapped = exits.iter().find(|exit| {
        exit.get_failures()
            .iter()
            .any(|&code| spelling.spell(guide.printed(code)) == printed)
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
