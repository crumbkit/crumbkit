//! Groups, whose help lists their subcommands under goal headings, and the
//! help template that [`extend`](super::extend) and
//! [`parse`](crate::parse) give a guided command.

use std::fmt::Write as _;

use clap::Command;
use clap::builder::styling::Style;

use super::list::{list, names};

/// A command whose help lists its subcommands under goal headings.
///
/// Its `-h` and `--help` list each goal's subcommands with their
/// descriptions, one line each. A group built with [`Group::compact`]
/// lists only each goal's names in `-h`, on one line per goal, and keeps the
/// descriptions for `--help`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group {
    /// The names of the subcommands from the root to the command.
    path: &'static [&'static str],
    /// The goals, in the order the help page lists them.
    goals: &'static [Goal],
    /// Whether `-h` lists each goal's names on one line.
    compact: bool,
}

impl Group {
    /// Returns the group of the command at `path`, the real names of its
    /// subcommands from the root, as [`Call::new`](crate::Call::new) names a
    /// command. An empty path names the root. Its help lists `goals` in
    /// order.
    #[must_use]
    pub const fn new(path: &'static [&'static str], goals: &'static [Goal]) -> Self {
        Self {
            path,
            goals,
            compact: false,
        }
    }

    /// Returns the group with a compact `-h`, which lists each goal's
    /// subcommand names on one line after its heading and a colon, such as
    /// `Read:    list, show`, and leaves the descriptions to `--help`. A
    /// group with many subcommands keeps its `-h` short this way, as a
    /// leaf's `-h` leaves its columns to `--help`.
    ///
    /// ```
    /// use crumbkit::help::{Goal, Group};
    ///
    /// const BOOKS: Group = Group::new(
    ///     &["books"],
    ///     &[
    ///         Goal::new("Read", &["list", "show"]),
    ///         Goal::new("Change", &["remove"]),
    ///     ],
    /// )
    /// .compact();
    ///
    /// assert!(BOOKS.is_compact());
    /// ```
    #[must_use]
    pub const fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Returns the names of the subcommands from the root to the command.
    #[must_use]
    pub const fn path(&self) -> &'static [&'static str] {
        self.path
    }

    /// Returns the goals, in the order the help page lists them.
    #[must_use]
    pub const fn goals(&self) -> &'static [Goal] {
        self.goals
    }

    /// Reports whether [`Group::compact`] made the group's `-h` list only
    /// names.
    #[must_use]
    pub const fn is_compact(&self) -> bool {
        self.compact
    }
}

/// A heading on a group's help page and the subcommands listed under it,
/// such as `Read` with `list` and `show`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Goal {
    /// The heading, which prints without a colon in the full list and with
    /// one in a compact group's `-h`.
    pub(super) heading: &'static str,
    /// The real names of the subcommands, in the order the heading lists
    /// them.
    pub(super) commands: &'static [&'static str],
}

impl Goal {
    /// Returns the goal that lists `commands` under `heading`. `commands`
    /// holds each subcommand's real name, never an alias.
    #[must_use]
    pub const fn new(heading: &'static str, commands: &'static [&'static str]) -> Self {
        Self { heading, commands }
    }

    /// Returns the heading.
    #[must_use]
    pub const fn heading(&self) -> &'static str {
        self.heading
    }

    /// Returns the real names of the subcommands, in the order the heading
    /// lists them.
    #[must_use]
    pub const fn commands(&self) -> &'static [&'static str] {
        self.commands
    }
}

/// Gives `node`, the command at the path of `group`, a help template that
/// holds the headed list of `group`'s goals, with `header` as the style of
/// each heading and `literal` as the style of each name. It leaves `node`
/// unchanged when the list would be empty. [`extend`](super::extend) and
/// [`parse`](crate::parse) both apply a group through it.
///
/// The command's own `before_help` and `before_long_help` stay as the tool
/// set them, and the template prints them at the top of the page, as clap's
/// default template does. A compact group and a list that holds `{` are the
/// exceptions, which the comments below describe.
pub(crate) fn apply(node: &mut Command, group: &Group, header: Style, literal: Style) {
    let Some(list) = list(node, group.goals, header, literal) else {
        return;
    };
    // clap's template prints `before_help` in `-h` and `before_long_help`
    // in `--help`, and it has no other place whose text differs between
    // the two.
    //
    // RUST-10: a compact group puts its names in `before_help` and its full
    // list in `before_long_help`, after the usage line, so the command loses
    // its own `before_help` and `before_long_help`, as a list with `{`
    // does below. With clap's `wrap_help` feature, clap wraps both lists at
    // the terminal's width, and a wrapped line continues at column 0. A
    // template tag for each kind of help, if clap adds one, replaces this.
    if group.compact {
        let names = names(node, group.goals, header, literal).unwrap_or_default();
        *node = std::mem::take(node)
            .help_template(template(header, ""))
            .before_help(names)
            .before_long_help(list);
        return;
    }
    // clap expands each `{tag}` in a template and drops the text after a
    // `{` without a closing `}`, and it expands no tags in `before_help`.
    //
    // RUST-10: a list that holds `{`, from a heading or a subcommand's
    // description, goes in `before_help` instead of the template, so it
    // prints as written and the command loses its own `before_help` and
    // `before_long_help`. Clearing `before_long_help` keeps clap's `-h`
    // line from saying `(see more with '--help')` on a page whose `--help`
    // shows nothing more. A template tag that clap leaves to crumbkit
    // replaces this fallback if clap adds one.
    if list.contains('{') {
        *node = std::mem::take(node)
            .help_template(template(header, ""))
            .before_help(list)
            .before_long_help(None::<&str>);
    } else {
        *node = std::mem::take(node).help_template(template(header, &list));
    }
}

/// Returns the help template of a guided command, with `list` after the
/// usage line and `header` as the style of its `Options:` heading.
///
/// It is clap's default template with the guide's list in place of
/// `{all-args}`. An empty `list` leaves its place to `{before-help}`, which
/// then holds the list, as [`apply`] describes.
//
// RUST-10: the template copies the layout of clap's default template, and
// shelf's golden files fail when clap changes that layout. It handles
// options only: a guided command shows no `Arguments:` section for its
// positional arguments, and `{options}` lists an argument that has its own
// help heading under `Options:` (clap's help_template.rs). A template that
// writes each of clap's sections replaces it when a guided command needs
// positional arguments or help headings.
fn template(header: Style, list: &str) -> String {
    let mut template = String::with_capacity(list.len() + 160);
    if list.is_empty() {
        template.push_str("{about-with-newline}\n{usage-heading} {usage}\n\n{before-help}");
    } else {
        template.push_str("{before-help}{about-with-newline}\n{usage-heading} {usage}\n\n");
        template.push_str(list);
        template.push_str("\n\n");
    }
    // Writing to a `String` cannot fail.
    let _ = write!(
        template,
        "{header}Options:{header:#}\n{{options}}{{after-help}}"
    );
    template
}
