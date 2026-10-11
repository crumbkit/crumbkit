//! Tests for cargo-crumbkit's command-line grammar.

use clap::{CommandFactory as _, Parser as _};

use super::{Cli, GUIDE};

#[test]
fn the_command_tree_with_the_guide_applied_passes_clap_debug_assertions() {
    crumbkit::help::extend(Cli::command(), &GUIDE).debug_assert();
}

/// Returns the arguments of `cargo crumbkit weigh` with `args` after it.
fn weigh(args: &[&str]) -> Result<super::WeighArgs, clap::Error> {
    let words = ["cargo", "crumbkit", "weigh", "--bin", "shelf"];
    let cli = Cli::try_parse_from(words.iter().chain(args))?;
    let super::CargoCommand::Crumbkit { command } = cli.cargo;
    match command {
        super::CrumbkitCommand::Weigh(args) => Ok(args),
        super::CrumbkitCommand::Commands { .. } => panic!("weigh parsed as commands"),
    }
}

#[test]
fn log_implies_usage_and_since_needs_usage_or_log() {
    assert!(!weigh(&[]).unwrap().usage());
    assert!(weigh(&["--usage"]).unwrap().usage());
    assert!(weigh(&["--log", "usage.jsonl"]).unwrap().usage());
    assert_eq!(
        weigh(&["--usage", "--since", "1d"])
            .unwrap()
            .since
            .as_deref(),
        Some("1d")
    );
    assert!(weigh(&["--log", "usage.jsonl", "--since", "1d"]).is_ok());
    assert_eq!(
        weigh(&["--since", "1d"]).unwrap_err().kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );
}
