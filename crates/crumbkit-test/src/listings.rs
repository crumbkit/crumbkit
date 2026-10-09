//! The listings check, which gives every `list` command crumbkit's paging
//! flags and every `show` command its many ids.

use clap::{Arg, ArgAction, Command};

use crate::tree::{self, takes_values};

#[cfg(test)]
mod tests;

/// The most values that a `show` command's ids argument may accept.
const MOST_IDS: usize = 100;

/// Checks the arguments of every `list` and `show` command in the tree of
/// `command`, and panics, as a failed test assertion does, naming every
/// problem it finds.
///
/// A `list` or `show` command is one with that name and no subcommands. The
/// check walks `command` and every subcommand below it, including hidden
/// ones, and skips each command that `exempt` names and everything below it.
/// An exemption is a command's path below `command`, such as `"books list"`,
/// and an exemption that names no command is a problem. The check finds:
///
/// - a `list` command without a `--limit` argument that takes a value or
///   without an `--all` flag, which flattening crumbkit's `Page` gives it;
/// - a `show` command without a required positional argument whose
///   `num_args` takes at least 1 and at most 100 values and accepts more than
///   1, such as `num_args = 1..=100`.
///
/// ```
/// use clap::{Arg, ArgAction, Args, Command};
/// use crumbkit::Page;
///
/// let books = Command::new("books")
///     .subcommand(Page::<25>::augment_args(Command::new("list")))
///     .subcommand(
///         Command::new("show").arg(
///             Arg::new("ids")
///                 .required(true)
///                 .num_args(1..=100)
///                 .action(ArgAction::Append),
///         ),
///     );
///
/// crumbkit_test::check_listings(&books, &[]);
/// ```
///
/// # Panics
///
/// Panics when the tree breaks a rule above. The message lists each problem
/// on its own line and names each command by its path from `command`, such
/// as `shelf books list`.
pub fn check_listings(command: &Command, exempt: &[&str]) {
    let mut problems = Vec::new();
    for visit in tree::walk(command, exempt, &mut problems) {
        let path = visit.path();
        let command = visit.command();
        if command.has_subcommands() {
            continue;
        }
        match command.get_name() {
            "list" => {
                if !command.get_arguments().any(is_limit) {
                    problems.push(format!(
                        "{path}: a `list` command has a `--limit` argument that takes a value, as flattening crumbkit's `Page` gives it"
                    ));
                }
                if !command.get_arguments().any(is_all) {
                    problems.push(format!(
                        "{path}: a `list` command has an `--all` flag, as flattening crumbkit's `Page` gives it"
                    ));
                }
            }
            "show" if !command.get_arguments().any(is_ids) => problems.push(format!(
                "{path}: a `show` command has a required positional argument that takes at least 1 and at most {MOST_IDS} values and accepts more than 1"
            )),
            _ => {}
        }
    }
    tree::report("listing rules", &problems);
}

/// Reports whether `arg` is `--limit` and takes a value.
fn is_limit(arg: &Arg) -> bool {
    arg.get_long() == Some("limit") && takes_values(arg)
}

/// Reports whether `arg` is `--all` and is a flag that sets `true`.
fn is_all(arg: &Arg) -> bool {
    arg.get_long() == Some("all") && matches!(arg.get_action(), ArgAction::SetTrue)
}

/// Reports whether `arg` is a required positional argument whose `num_args`
/// takes at least 1 and at most [`MOST_IDS`] values and accepts more than 1.
fn is_ids(arg: &Arg) -> bool {
    arg.is_positional()
        && arg.is_required_set()
        && arg.get_num_args().is_some_and(|range| {
            range.min_values() >= 1 && range.max_values() > 1 && range.max_values() <= MOST_IDS
        })
}
