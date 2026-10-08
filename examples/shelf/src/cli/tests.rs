//! Tests for shelf's command-line grammar.

use clap::CommandFactory as _;

use super::Cli;

#[test]
fn the_command_tree_passes_clap_debug_assertions() {
    Cli::command().debug_assert();
}
