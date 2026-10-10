//! The command tree that the unit tests expand and list against.

use clap::{Args as _, Command};

use crate::AliasesList;

/// Returns a tool's command tree as `crumbkit::parse` builds it, with
/// clap's generated `help` subcommand turned off.
///
/// `books` has the visible alias `b` and its `list` the hidden alias `ls`,
/// so the built-in aliases sit at two levels. The hidden `admin` branch has
/// aliases at both levels too, which `aliases list` leaves out. The hidden
/// `aliases list` command takes [`AliasesList`].
pub(crate) fn tool() -> Command {
    Command::new("shelf")
        .disable_help_subcommand(true)
        .subcommand(
            Command::new("books")
                .visible_alias("b")
                .subcommand(Command::new("list").alias("ls"))
                .subcommand(Command::new("show")),
        )
        .subcommand(
            Command::new("admin")
                .hide(true)
                .alias("adm")
                .subcommand(Command::new("reset").alias("rs")),
        )
        .subcommand(
            Command::new("aliases")
                .hide(true)
                .subcommand(AliasesList::augment_args(Command::new("list"))),
        )
}
