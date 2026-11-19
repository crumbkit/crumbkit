//! The tool record check of the guide topics that `Topic::guide` declares.

use clap::Command;
use crumbkit::help::{Step, Topic};

use super::command_line;

/// Adds a problem to `problems` for each way that the guide `name` with
/// `steps` breaks a guide rule: no steps, more than `Topic::MAX_STEPS`
/// steps, a step with no commands or more than `Step::MAX_COMMANDS`, a step
/// whose explanation is empty or holds a line break, and a command that
/// clap refuses when it parses the tool's name and the command's words
/// against `parser`, a copy of the tool's tree that the check may parse
/// with. A `--help` or `--version` command counts as refused, as in a leaf's
/// examples.
pub(super) fn check_guide(
    command: &Command,
    name: &str,
    steps: &[Step],
    parser: &mut Command,
    problems: &mut Vec<String>,
) {
    if steps.is_empty() {
        problems.push(format!(
            "the guide `{name}` has no steps, so `{} help {name}` prints an empty page",
            command.get_name()
        ));
    }
    if steps.len() > Topic::MAX_STEPS {
        problems.push(format!(
            "the guide `{name}` has {} steps, over the limit of {}; split it into two guides",
            steps.len(),
            Topic::MAX_STEPS
        ));
    }
    for (index, step) in steps.iter().enumerate() {
        check_step(command, name, index + 1, step, parser, problems);
    }
}

/// Adds a problem to `problems` for each rule that step `number` of the
/// guide `name` breaks.
fn check_step(
    command: &Command,
    name: &str,
    number: usize,
    step: &Step,
    parser: &mut Command,
    problems: &mut Vec<String>,
) {
    let about = step.about();
    if about.trim().is_empty() {
        problems.push(format!(
            "the guide `{name}` step {number} has an empty explanation"
        ));
    } else if about.contains(['\n', '\r']) {
        problems.push(format!(
            "the guide `{name}` step {number} has an explanation with a line break, so help cannot print it on one line: `{}`",
            about.escape_debug()
        ));
    }
    let commands = step.commands();
    if commands.is_empty() {
        problems.push(format!(
            "the guide `{name}` step {number} has no commands, so it shows nothing to run"
        ));
    }
    if commands.len() > Step::MAX_COMMANDS {
        problems.push(format!(
            "the guide `{name}` step {number} has {} commands, over the limit of {}; split it into two steps",
            commands.len(),
            Step::MAX_COMMANDS
        ));
    }
    let tool_name = command.get_name();
    // RUST-10: a guide command is parsed against the tool's whole tree, so
    // the check does less than it does for a leaf's examples. It does not
    // refuse a view flag that the command does not offer, because a guide
    // command names no leaf, and it accepts a group word that prints help.
    // A leaf's example covers those through `Leaf::examples`.
    for words in commands {
        if let Some(word) = words.iter().find(|word| word.contains(['\n', '\r'])) {
            problems.push(format!(
                "the guide `{name}` step {number} has a command word with a line break, so help cannot print the command on one line: `{}`",
                word.escape_debug()
            ));
        }
        let args = std::iter::once(tool_name).chain(words.iter().copied());
        if let Err(error) = parser.try_get_matches_from_mut(args) {
            problems.push(format!(
                "the guide `{name}` step {number}: clap refuses the command `{}` with the error kind {:?}",
                command_line(tool_name, words),
                error.kind()
            ));
        }
    }
}
