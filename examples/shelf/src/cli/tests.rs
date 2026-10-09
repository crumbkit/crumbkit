//! Tests for shelf's command-line grammar.

use clap::CommandFactory as _;

use super::{Cli, GUIDE};

#[test]
fn the_command_tree_with_shelf_s_guide_applied_passes_clap_debug_assertions() {
    crumbkit::help::extend(Cli::command(), &GUIDE).debug_assert();
}

#[test]
fn the_version_is_the_package_version_with_optional_git_metadata() {
    let version = Cli::command().render_version();
    let version = version.trim_end();

    let metadata = version
        .strip_prefix(concat!("shelf ", env!("CARGO_PKG_VERSION")))
        .unwrap();
    if metadata.is_empty() {
        return;
    }
    let commit = metadata.strip_prefix("+git-").unwrap();
    let commit = commit.strip_suffix(".dirty").unwrap_or(commit);
    assert_eq!(commit.len(), 12, "{version}");
    assert!(
        commit.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "{version}"
    );
}
