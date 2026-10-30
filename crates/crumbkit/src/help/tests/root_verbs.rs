//! Tests for root verbs, which root help lists under their own heading when
//! the guide declares them.

use clap::Command;

use super::{help, section, tool};
use crate::help::{Goal, Group, Guide, extend};

/// The reason that the tests' guides give for their root verbs.
const REASON: &str = "People start and stop the telescope every night.";

/// `tool()` with two commands without subcommands directly under the root.
fn telescope() -> Command {
    tool()
        .subcommand(Command::new("start").about("Start the telescope"))
        .subcommand(Command::new("stop").about("Stop the telescope"))
}

/// Returns the root's `-h` of `telescope()` with `guide`.
fn root_help(guide: &Guide) -> String {
    let mut out = Vec::new();
    let _ = crate::parse_to::<(), _, _, _, _>(
        &mut crate::Sink::new(&mut out, Vec::new()),
        telescope(),
        guide,
        ["tool", "-h"],
    );
    String::from_utf8(out).unwrap()
}

#[test]
fn a_guide_declares_no_root_verbs_until_its_builder_declares_them() {
    assert_eq!(Guide::new().get_root_verbs(), None);
    let guide = Guide::new().root_verbs(&["start", "stop"], REASON);
    assert_eq!(
        guide.get_root_verbs(),
        Some((["start", "stop"].as_slice(), REASON))
    );
}

#[test]
fn root_help_lists_the_root_verbs_under_their_heading_before_the_other_commands() {
    const GUIDE: Guide = Guide::new().root_verbs(&["stop", "start"], REASON);

    let expected = "\
Verbs
  stop            Stop the telescope
  start           Start the telescope

Commands
  planets         Work with planets
  moons           Work with moons
  constellations  Work with constellations

Options:
";
    assert_eq!(section(&root_help(&GUIDE)), expected);
}

#[test]
fn root_help_without_root_verbs_keeps_clap_s_commands_list() {
    let page = root_help(&Guide::new());

    assert!(page.contains("Commands:\n  planets "), "{page}");
    assert!(
        page.contains("\n  start           Start the telescope\n"),
        "{page}"
    );
    assert!(!page.contains("Verbs"), "{page}");
}

#[test]
fn root_help_lists_the_root_verbs_after_the_root_group_s_goals() {
    const GUIDE: Guide = Guide::new()
        .groups(&[Group::new(
            &[],
            &[Goal::new("Explore", &["planets", "moons", "stop"])],
        )])
        .root_verbs(&["start", "stop"], REASON);

    let expected = "\
Explore
  planets         Work with planets
  moons           Work with moons
  stop            Stop the telescope

Verbs
  start           Start the telescope

Commands
  constellations  Work with constellations

Options:
";
    assert_eq!(section(&root_help(&GUIDE)), expected);
}

#[test]
fn a_compact_root_group_s_short_help_lists_the_root_verbs_on_their_own_line() {
    const GUIDE: Guide = Guide::new()
        .groups(&[Group::new(&[], &[Goal::new("Explore", &["planets", "moons"])]).compact()])
        .root_verbs(&["start", "stop"], REASON);

    let page = root_help(&GUIDE);

    assert!(
        page.contains(
            "Explore:   planets, moons\nVerbs:     start, stop\nCommands:  constellations\n"
        ),
        "{page}"
    );
}

#[test]
fn root_verbs_change_only_the_root_s_help() {
    const GUIDE: Guide = Guide::new().root_verbs(&["start", "stop"], REASON);

    assert_eq!(
        help(&GUIDE, &["tool", "planets", "-h"]),
        help(&Guide::new(), &["tool", "planets", "-h"])
    );
}

#[test]
fn extend_lists_the_root_verbs_under_their_heading_as_parse_does() {
    const GUIDE: Guide = Guide::new().root_verbs(&["start"], REASON);

    let page = extend(telescope(), &GUIDE).render_help().to_string();

    assert!(
        page.contains("\n\nVerbs\n  start           Start the telescope\n\nCommands\n  planets "),
        "{page}"
    );
}
