//! The filters check, which makes every option that accepts many values
//! accept them both repeated and separated by commas.

use clap::{Arg, ArgAction, Command};

use crate::tree::{self, argument_name, takes_values};

#[cfg(test)]
mod tests;

/// Checks every option that takes values in the tree of `command`, and
/// panics, as a failed test assertion does, naming every problem it finds.
///
/// The check looks at each argument that takes values and is not positional,
/// so boolean flags, counted flags and positional arguments are out of its
/// scope. It walks `command` and every subcommand below it, including hidden
/// ones, and skips each command that `exempt` names and everything below it.
/// An exemption is a command's path below `command`, such as `"books list"`,
/// and an exemption that names no command is a problem. The check finds an
/// option that accepts many values, because its action is
/// `ArgAction::Append` or its `num_args` accepts more than one value, unless
/// its action is `ArgAction::Append` and its value delimiter is `,`. Such an
/// option accepts both `--author a --author b` and `--author a,b`, as
/// crumbkit's `--fields` does.
///
/// ```
/// use clap::{Arg, ArgAction, Command};
///
/// let list = Command::new("list").arg(
///     Arg::new("author")
///         .long("author")
///         .value_delimiter(',')
///         .action(ArgAction::Append),
/// );
///
/// crumbkit_test::check_filters(&list, &[]);
/// ```
///
/// # Panics
///
/// Panics when the tree breaks the rule above. The message lists each
/// problem on its own line and names each command by its path from
/// `command`, such as `shelf books list`.
pub fn check_filters(command: &Command, exempt: &[&str]) {
    let mut problems = Vec::new();
    for visit in tree::walk(command, exempt, &mut problems) {
        let path = visit.path();
        for arg in visit.command().get_arguments() {
            if arg.is_positional() || !takes_values(arg) {
                continue;
            }
            if accepts_many(arg) && !accepts_repeats_and_commas(arg) {
                problems.push(format!(
                    "{path}: the argument `{}` accepts many values, so it accepts both repeats and commas, with `ArgAction::Append` and the value delimiter `,`",
                    argument_name(arg)
                ));
            }
        }
    }
    tree::report("filter rules", &problems);
}

/// Reports whether `arg` accepts more than one value, either because its
/// action appends a value on each use or because one use takes several.
fn accepts_many(arg: &Arg) -> bool {
    matches!(arg.get_action(), ArgAction::Append)
        || arg
            .get_num_args()
            .is_some_and(|range| range.max_values() > 1)
}

/// Reports whether `arg` appends a value on each use and splits each value
/// at commas.
fn accepts_repeats_and_commas(arg: &Arg) -> bool {
    matches!(arg.get_action(), ArgAction::Append) && arg.get_value_delimiter() == Some(',')
}
