//! The lines that `parse` adds to the help of the commands on the typed
//! path, the goal headings it applies to them, and the sections of the
//! deepest command's leaf.

use clap::Command;
use clap::builder::styling::Style;

use crate::help::{self, Guide, Leaf, Topic, leaf, topic};

/// Adds crumbkit's help lines to the root and to each command on `path`, and
/// applies to each of those commands the groups of `guide` that name it. A
/// guided group off `path` keeps clap's help, so the work grows with the
/// typed depth. The root's lines name `topics`.
///
/// The deepest command on `path` also gets the sections of the leaf of
/// `guide` whose path is `path`, the first when two leaves share it. Its
/// `--help` describes the leaf's columns only when `long_help` is set,
/// because that builds the row type's JSON Schema.
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
    long_help: bool,
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
    let leaf = guide.get_leaves().iter().find(|leaf| leaf.path() == path);

    let mut text = command.get_name().to_owned();
    let mut node = command;
    let mut depth = 0;
    loop {
        apply_groups(node, guide, &path[..depth], header, literal);
        let sections = match leaf {
            Some(leaf) if depth == path.len() => {
                leaf_sections(leaf, &text, long_help, header, literal)
            }
            _ => Sections::default(),
        };
        let topics = if depth == 0 { topics.as_str() } else { "" };
        set_help_lines(node, &text, &flags, topics, sections);
        let Some(name) = path.get(depth) else {
            break;
        };
        let Some(subcommand) = node.find_subcommand_mut(name) else {
            break;
        };
        text.push(' ');
        text.push_str(name);
        node = subcommand;
        depth += 1;
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

/// The sections that a leaf adds to its command's help before crumbkit's
/// lines, and whether its `-h` points at its columns.
#[derive(Default)]
struct Sections {
    /// The sections of `-h`, in order.
    short: Vec<String>,
    /// The sections of `--help`, in order.
    long: Vec<String>,
    /// Whether the leaf has a document, so `-h` ends with a `Next:` line
    /// that names the `--help` that lists its columns.
    columns: bool,
}

/// Returns the sections of `leaf`, whose command line starts with `text`:
/// its examples in `-h` and `--help`, and in `--help` the sections that
/// describe its columns when `long_help` is set.
fn leaf_sections(
    leaf: &Leaf,
    text: &str,
    long_help: bool,
    header: Style,
    literal: Style,
) -> Sections {
    let short = leaf::examples(leaf, text, false, header, literal);
    let mut long: Vec<String> = leaf::examples(leaf, text, true, header, literal)
        .into_iter()
        .collect();
    if long_help {
        long.extend(leaf::columns(leaf, header, literal));
    }
    Sections {
        short: short.into_iter().collect(),
        long,
        columns: leaf.has_document(),
    }
}

/// Sets the text that ends the help of `node`, whose path is `text`: the
/// leaf's `sections`, and then crumbkit's lines. The lines are the global
/// flags line in `-h` when `flags` is not empty, the topics line in `-h`
/// and `--help` when `topics` is not empty, and a `Next:` line. The `Next:`
/// line opens a subcommand in `-h` and `--help` when `node` has a visible
/// subcommand, and names `--help` in `-h` when the leaf has columns. Only
/// the root gets `topics`. A blank line separates each section from the
/// next and from the lines.
fn set_help_lines(node: &mut Command, text: &str, flags: &str, topics: &str, sections: Sections) {
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
    if sections.columns {
        short.push(format!("Next: {text} --help explains each column."));
    }
    let short = join(sections.short, &short);
    if short.is_empty() {
        return;
    }
    // `--help` falls back to `after_help` when `after_long_help` is unset, so
    // a command without a `Next:` line gets an empty `after_long_help`, and
    // clap trims the blank lines it leaves.
    *node = std::mem::take(node)
        .after_help(short)
        .after_long_help(join(sections.long, &long));
}

/// Returns `sections` and then `lines` as one text: a blank line between
/// sections, and each of `lines` on its own line after them.
fn join(mut sections: Vec<String>, lines: &[String]) -> String {
    if !lines.is_empty() {
        sections.push(lines.join("\n"));
    }
    sections.join("\n\n")
}

/// Reports whether `node` has a subcommand that its help lists. A hidden
/// subcommand does not count. clap's own `has_visible_subcommands` is
/// private to clap.
pub(super) fn has_visible_subcommands(node: &Command) -> bool {
    node.get_subcommands()
        .any(|subcommand| !subcommand.is_hide_set())
}
