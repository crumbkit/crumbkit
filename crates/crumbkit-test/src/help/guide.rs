//! The tool record check of the guide topics that `Topic::guide` declares.

use clap::Command;
use crumbkit::Tool;
use crumbkit::help::{Step, Topic};

use super::{command_line, push_view_flag_problems};

/// Adds a problem to `problems` for each way that the guide `name` with
/// `steps` breaks a guide rule: no steps, more than `Topic::MAX_STEPS`
/// steps, a step with no commands or more than `Step::MAX_COMMANDS`, a step
/// whose explanation is empty or holds a line break, and a command that
/// clap refuses when it parses the tool's name and the command's words
/// against `parser`, a copy of the tool's tree that the check may parse
/// with. A `--help` or `--version` command counts as refused, as in a leaf's
/// examples. A command that types a view flag that the leaf at its path in
/// `tool` does not offer, or that adds nothing to the leaf's JSON, fails as
/// an example does. The check skips a command whose path names no leaf of
/// the tool record, so it accepts a bare group word that prints help.
pub(super) fn check_guide(
    command: &Command,
    tool: &Tool,
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
        check_step(command, tool, name, index + 1, step, parser, problems);
    }
}

/// Adds a problem to `problems` for each rule that step `number` of the
/// guide `name` breaks.
fn check_step(
    command: &Command,
    tool: &Tool,
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
    for words in commands {
        if let Some(word) = words.iter().find(|word| word.contains(['\n', '\r'])) {
            problems.push(format!(
                "the guide `{name}` step {number} has a command word with a line break, so help cannot print the command on one line: `{}`",
                word.escape_debug()
            ));
        }
        let args = std::iter::once(tool_name).chain(words.iter().copied());
        let line = command_line(tool_name, words);
        match parser.try_get_matches_from_mut(args) {
            Ok(matches) => {
                let mut path = Vec::new();
                let mut level = &matches;
                while let Some((word, next)) = level.subcommand() {
                    path.push(word);
                    level = next;
                }
                let leaf = tool
                    .get_help()
                    .get_leaves()
                    .iter()
                    .find(|leaf| leaf.path() == path.as_slice());
                if let Some(leaf) = leaf {
                    let prefix = format!("the guide `{name}` step {number}");
                    push_view_flag_problems(&prefix, "command", &line, leaf, &matches, problems);
                }
            }
            Err(error) => problems.push(format!(
                "the guide `{name}` step {number}: clap refuses the command `{line}` with the error kind {:?}",
                error.kind()
            )),
        }
    }
}
