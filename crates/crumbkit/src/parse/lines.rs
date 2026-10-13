//! The lines that `parse` adds to the help of the commands on the typed
//! path, the goal headings it applies to them, and the sections of the
//! deepest command's leaf.

use std::fmt::Write as _;

use clap::Command;
use clap::builder::StyledStr;
use clap::builder::styling::Style;

use crate::expect;
use crate::help::{self, Effects, Guide, Leaf, ViewFlag, leaf, topic};

/// Adds crumbkit's help lines to the root and to each command on `path`, and
/// applies to each of those commands the groups of `guide` that name it. A
/// guided group off `path` keeps clap's help, so the work grows with the
/// typed depth. When the tool `has_topics`, the lines of the guide's root
/// name the guide's topics and `exit-codes`. When `aliases` is set, the
/// lines of the clap root name the aliases that it returns for the clap
/// root.
///
/// The deepest command on `path` also gets the sections of the leaf of
/// `guide` whose path is `path`, the first when two leaves share it. Its
/// `--help` describes the leaf's columns only when `long_help` is set,
/// because that builds the row type's JSON Schema. Its help also leaves out
/// each global view flag that the leaf does not offer, and so does its
/// `Global flags:` line.
///
/// Each command on the path, the root included, hides its own visible global
/// flags from `-h` before clap builds it, so the copies that clap gives each
/// subcommand below it are hidden too. It then builds that command alone
/// with `Command::render_usage`, because `Arg`'s `Display` reads an option's
/// number of values, which clap sets in the build. The `Global flags:` line
/// of a command names the visible global flags of the root and of each
/// command on the path down to it, the root's first. The parse that follows
/// builds only the commands it reaches, which are the same commands, and
/// `Command::build` would build the whole tree.
pub(super) fn add_help_lines(
    command: &mut Command,
    guide: &Guide,
    has_topics: bool,
    aliases: Option<fn(&Command) -> Vec<String>>,
    path: &[String],
    long_help: bool,
) {
    let root = guide.root_in(command);
    let tool = command.get_name();
    let aliases = match aliases.map(|names| names(command)) {
        Some(names) if !names.is_empty() => format!(
            "Personal aliases: {}. `{tool} aliases list` shows what each runs.",
            names.join(", ")
        ),
        _ => String::new(),
    };
    let topics = if has_topics {
        topic::names(guide.get_topics()).join(", ")
    } else {
        String::new()
    };
    // Every guided page prints with the root's styles, as in `help::extend`.
    let styles = command.get_styles();
    let header = *styles.get_header();
    let literal = *styles.get_literal();
    let leaf = guide.get_leaves().iter().find(|leaf| leaf.path() == path);

    let mut text = command.get_name().to_owned();
    // The ids and the text of the global flags of the commands so far.
    let mut flags: Vec<(String, String)> = Vec::new();
    let mut node = command;
    let mut depth = 0;
    loop {
        let at_leaf = leaf.filter(|_| depth == path.len());
        if let Some(leaf) = at_leaf {
            hide_unoffered(node, leaf);
        }
        add_global_flags(node, &mut flags);
        let flag_list: Vec<&str> = flags
            .iter()
            .filter(|(id, _)| at_leaf.is_none_or(|leaf| offers(leaf, id)))
            .map(|(_, flag)| flag.as_str())
            .collect();
        apply_groups(node, guide, &path[..depth], header, literal);
        let sections = match at_leaf {
            Some(leaf) => leaf_sections(leaf, node, &text, long_help, header, literal),
            None => Sections::default(),
        };
        let topics = if path[..depth] == *root {
            topics.as_str()
        } else {
            ""
        };
        let lines = Lines {
            flags: &flag_list.join(", "),
            topics,
            aliases: if depth == 0 { aliases.as_str() } else { "" },
        };
        set_help_lines(node, &text, &lines, sections);
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

/// Hides the visible global flags of `node` from `-h`, builds `node` alone,
/// and adds to `flags` the id and the text of each visible global flag of
/// `node` whose id `flags` does not hold yet, in definition order.
///
/// The build copies the global flags of `node` into each of its
/// subcommands, so those copies are hidden too. A global flag of a command
/// above `node` is already in `flags`, from that command.
fn add_global_flags(node: &mut Command, flags: &mut Vec<(String, String)>) {
    *node = std::mem::take(node).mut_args(|arg| {
        if arg.is_global_set() && !arg.is_hide_set() {
            arg.hide_short_help(true)
        } else {
            arg
        }
    });
    let _ = node.render_usage();
    for arg in node.get_arguments() {
        let id = arg.get_id().as_str();
        let listed = flags.iter().any(|(listed, _)| listed == id);
        if arg.is_global_set() && !arg.is_hide_set() && !listed {
            flags.push((id.to_owned(), arg.to_string()));
        }
    }
}

/// Hides from the help of `node`, the command of `leaf`, each global view
/// flag that the leaf does not offer (ADR-0016). An argument that is not
/// global keeps its help, so a tool's own `--long` on the command stays.
fn hide_unoffered(node: &mut Command, leaf: &Leaf) {
    *node = std::mem::take(node).mut_args(|arg| {
        if arg.is_global_set() && !offers(leaf, arg.get_id().as_str()) {
            arg.hide(true)
        } else {
            arg
        }
    });
}

/// Reports whether `leaf` offers the flag whose argument id is `id`. Every
/// flag that is not a view flag counts as offered.
fn offers(leaf: &Leaf, id: &str) -> bool {
    ViewFlag::from_id(id).is_none_or(|flag| leaf.offers(flag))
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
    /// Whether the leaf has a document with columns, so `-h` ends with a
    /// `Next:` line that names the `--help` that lists them.
    columns: bool,
    /// The line that names the leaf's effects, which `-h` and `--help`
    /// print first among crumbkit's lines, or `None` when the leaf declares
    /// none.
    effects: Option<&'static str>,
    /// The line that names how the leaf guards what it writes, which `-h`
    /// and `--help` print after the effects line, or `None` when it has
    /// none.
    writes: Option<String>,
    /// The `Output:` line that `-h` and `--help` print after the effects
    /// line, which [`Leaf::output_line`] returns, or `None` when the leaf
    /// has none.
    output: Option<&'static str>,
    /// The `Problems:` line that `-h` and `--help` print after the
    /// `Output:` line, which [`Leaf::problems_line`] returns, or `None`
    /// when the leaf's rows name no problem field.
    problems: Option<String>,
}

/// Returns the sections of `leaf`, whose command is `node` and whose
/// command line starts with `text`:
/// its examples in `-h` and `--help`, and in `--help` the sections that
/// describe its columns when `long_help` is set.
fn leaf_sections(
    leaf: &Leaf,
    node: &Command,
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
        columns: leaf.has_columns(),
        effects: leaf.get_effects().map(Effects::line),
        writes: leaf
            .get_effects()
            .and_then(|effects| effects.writes_line(expect::takes_expect(node))),
        output: leaf.output_line(),
        problems: leaf.problems_line(),
    }
}

/// The text of crumbkit's lines for one command, each empty when the
/// command has no such line.
struct Lines<'a> {
    /// The visible global flags of the command and of the commands above it.
    flags: &'a str,
    /// The names of the topics, which only the guide's root has.
    topics: &'a str,
    /// The whole line that names a person's aliases, which only the clap
    /// root has.
    aliases: &'a str,
}

/// Sets the text that ends the help of `node`, whose path is `text`: the
/// command's own `after_help` or `after_long_help`, the leaf's `sections`,
/// and then crumbkit's `lines`. The lines are the leaf's effects line in
/// `-h` and `--help` when it declares effects, its `Writes:` line in `-h`
/// and `--help` when it guards what it writes, the `Output:` line in `-h`
/// and `--help` when the leaf prints a stream or bytes, the `Problems:`
/// line in `-h` and `--help` when its rows name a problem field, the global flags
/// line in `-h`, the topics line and the aliases line in `-h` and `--help`, and a
/// `Next:` line, each when it has text. The `Next:` line opens a
/// subcommand in `-h` and `--help` when `node` has a visible subcommand, and
/// names `--help` in `-h` when the leaf has columns. A blank line separates
/// the command's own text, each section and the lines.
fn set_help_lines(node: &mut Command, text: &str, lines: &Lines<'_>, sections: Sections) {
    let Lines {
        flags,
        topics,
        aliases,
    } = *lines;
    let mut short = Vec::new();
    let mut long = Vec::new();
    if let Some(effects) = sections.effects {
        short.push(effects.to_owned());
        long.push(effects.to_owned());
    }
    if let Some(writes) = sections.writes {
        short.push(writes.clone());
        long.push(writes);
    }
    if let Some(output) = sections.output {
        short.push(output.to_owned());
        long.push(output.to_owned());
    }
    if let Some(problems) = sections.problems {
        short.push(problems.clone());
        long.push(problems);
    }
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
    if !aliases.is_empty() {
        short.push(aliases.to_owned());
        long.push(aliases.to_owned());
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
    if short.is_empty() && sections.short.is_empty() {
        return;
    }
    // `--help` falls back to `after_help` when `after_long_help` is unset, so
    // the tool's own long text is its `after_help` when it has no
    // `after_long_help`.
    let own_short = own_text(node.get_after_help(), sections.short.first());
    let own_long = own_text(
        node.get_after_long_help().or_else(|| node.get_after_help()),
        sections.long.first(),
    );
    let short = after(own_short, &join(sections.short, &short));
    // A command without a `Next:` line and without text of its own gets an
    // empty `after_long_help`, so `--help` does not fall back to the
    // `after_help` set here, and clap trims the blank lines it leaves.
    let long = after(own_long, &join(sections.long, &long));
    *node = std::mem::take(node).after_help(short).after_long_help(long);
}

/// Returns a copy of `own`, one of the command's own `after_help` texts, or
/// `None` when it is unset or empty or equals `examples`, the first of the
/// leaf's sections. [`help::extend`] writes a leaf's examples section in
/// place of the command's `after_help` and `after_long_help`, so that text
/// is the section that `parse` adds, and it prints once.
fn own_text(own: Option<&StyledStr>, examples: Option<&String>) -> Option<StyledStr> {
    let own = own?;
    let rewritten = examples.is_some_and(|examples| *own == StyledStr::from(examples));
    if *own == StyledStr::new() || rewritten {
        None
    } else {
        Some(own.clone())
    }
}

/// Returns `own`, the command's own text, and then `text`, crumbkit's
/// sections and lines, with a blank line between them. The text keeps the
/// styles of both, because it is written as it is.
fn after(own: Option<StyledStr>, text: &str) -> StyledStr {
    let Some(mut own) = own else {
        return StyledStr::from(text.to_owned());
    };
    if !text.is_empty() {
        // Writing to a `StyledStr` cannot fail.
        let _ = write!(own, "\n\n{text}");
    }
    own
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
