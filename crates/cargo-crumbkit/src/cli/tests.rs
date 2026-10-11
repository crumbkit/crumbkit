//! Tests for cargo-crumbkit's command-line grammar.

use clap::CommandFactory as _;

use super::{Cli, GUIDE};

#[test]
fn the_command_tree_with_the_guide_applied_passes_clap_debug_assertions() {
    crumbkit::help::extend(Cli::command(), &GUIDE).debug_assert();
}
