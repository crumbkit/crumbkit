//! The lines that `parse` adds to the help of the commands on the typed
//! path, and the goal headings it applies to them.

use clap::Command;
use clap::builder::styling::Style;

use crate::help::{self, Guide, Topic, topic};

/// Adds crumbkit's help lines to the root and to each command on `path`, and
/// applies to each of those commands the groups of `guide` that name it. A
/// guided group off `path` keeps clap's help, so the work grows with the
/// typed depth. The root's lines name `topics`.
///
/// It hides the root's visible global flags from `-h` first, so the copies
/// that clap gives each subcommand are hidden too. It then builds the root
/// alone with `Command::render_usage`, because `Arg`'s `Display` reads an
/// option's number of values, which clap sets in the build. The parse that
/// follows builds only the commands it reaches, and `Command::build` would
/// build the whole tree.
pub(super) fn add_help_lines(
    command: &mut Command,
    guide: &Guide,
    topics: &[Topic],
    path: &[String],
) {
    *command = std::mem::take(command).mut_args(|arg| {
        if arg.is_global_set() && !arg.is_hide_set() {
            arg.hide_short_help(true)
        } else {
            arg
        }
    });
    let _ = command.render_usage();
    let flags: Vec<String> = command
        .get_arguments()
        .filter(|arg| arg.is_global_set() && !arg.is_hide_set())
        .map(ToString::to_string)
        .collect();
    let flags = flags.join(", ");
    let topics = topic::names(topics).join(", ");
    // Every guided page prints with the root's styles, as in `help::extend`.
    let styles = command.get_styles();
    let header = *styles.get_header();
    let literal = *styles.get_literal();

    let mut text = command.get_name().to_owned();
    let mut node = command;
    apply_groups(node, guide, &[], header, literal);
    set_help_lines(node, &text, &flags, &topics);
    for (depth, name) in path.iter().enumerate() {
        let Some(subcommand) = node.find_subcommand_mut(name) else {
            break;
        };
        text.push(' ');
        text.push_str(name);
        node = subcommand;
        apply_groups(node, guide, &path[..=depth], header, literal);
        set_help_lines(node, &text, &flags, "");
    }
}

/// Applies to `node`, the command at `path`, each group of `guide` whose
/// path is `path`, in guide order, as [`help::extend`] applies them.
fn apply_groups(node: &mut Command, guide: &Guide, path: &[String], header: Style, literal: Style) {
    for group in guide.get_groups() {
        if group.path() == path {
            help::apply(node, group, header, literal);
        }
    }
}

/// Sets the lines that end the help of `node`, whose path is `text`: the
/// global flags line in `-h` when `flags` is not empty, the topics line in
/// `-h` and `--help` when `topics` is not empty, and the `Next:` line in
/// `-h` and `--help` when `node` has a visible subcommand. Only the root
/// gets `topics`.
fn set_help_lines(node: &mut Command, text: &str, flags: &str, topics: &str) {
    let mut short = Vec::new();
    let mut long = Vec::new();
    if !flags.is_empty() {
        short.push(format!(
            "Global flags: {flags}. `{text} --help` describes them."
        ));
    }
    if !topics.is_empty() {
        let line = format!("Topics: {topics}. `{text} help <TOPIC>` prints one.");
        short.push(line.clone());
        long.push(line);
    }
    if has_visible_subcommands(node) {
        let value_name = node.get_subcommand_value_name().unwrap_or("COMMAND");
        let next = format!("Next: {text} <{value_name}> -h");
        short.push(next.clone());
        long.push(next);
    }
    if short.is_empty() {
        return;
    }
    // `--help` falls back to `after_help` when `after_long_help` is unset, so
    // a command without a `Next:` line gets an empty `after_long_help`, and
    // clap trims the blank lines it leaves.
    *node = std::mem::take(node)
        .after_help(short.join("\n"))
        .after_long_help(long.join("\n"));
}

/// Reports whether `node` has a subcommand that its help lists. A hidden
/// subcommand does not count. clap's own `has_visible_subcommands` is
/// private to clap.
pub(super) fn has_visible_subcommands(node: &Command) -> bool {
    node.get_subcommands()
        .any(|subcommand| !subcommand.is_hide_set())
}
