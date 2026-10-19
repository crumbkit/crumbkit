//! Tests for a compact group, whose `-h` lists each goal's subcommand names
//! on one line and whose `--help` keeps the name and description lines.

use clap::Command;

use super::{help, section, tool};
use crate::help::{Goal, Group, Guide, extend};

/// A compact guide of `planets`, with two goals.
const PLANETS: Guide = Guide::new().groups(&[Group::new(
    &["planets"],
    &[
        Goal::new("Read", &["list", "show"]),
        Goal::new("Change", &["rename"]),
    ],
)
.compact()]);

#[test]
fn a_group_lists_names_with_descriptions_until_its_builder_makes_it_compact() {
    const GROUP: Group = Group::new(&["planets"], &[Goal::new("Read", &["list"])]);
    let group = GROUP;
    assert!(!group.is_compact());
    assert!(group.compact().is_compact());
}

#[test]
fn a_compact_group_s_short_help_lists_each_goal_s_names_on_one_line() {
    let expected = "\
Work with planets

Usage: tool planets [OPTIONS] <COMMAND>

Read:    list, show
Change:  rename

Options:
  -h, --help  Print help (see more with '--help')

Global flags: --json. `tool planets --help` describes them.
Next: tool planets <COMMAND> -h
";
    assert_eq!(help(&PLANETS, &["tool", "planets", "-h"]), expected);
}

#[test]
fn a_compact_group_s_long_help_keeps_each_name_with_its_description() {
    let expected = "\
Read
  list    List the planets
  show    Show planets by name

Change
  rename  Rename a planet

Options:
";
    let page = help(&PLANETS, &["tool", "planets", "--help"]);
    assert_eq!(section(&page), expected);
}

#[test]
fn extend_and_parse_print_the_same_compact_and_full_lists() {
    let mut tree = extend(tool(), &PLANETS);
    let planets = tree.find_subcommand_mut("planets").unwrap();
    let short = planets.render_help().to_string();
    let long = planets.render_long_help().to_string();

    let parsed_short = help(&PLANETS, &["tool", "planets", "-h"]);
    let parsed_long = help(&PLANETS, &["tool", "planets", "--help"]);

    assert_eq!(section(&short), section(&parsed_short));
    assert_eq!(section(&long), section(&parsed_long));
    assert!(
        section(&short).starts_with("Read:    list, show\n"),
        "{short}"
    );
    assert!(section(&long).starts_with("Read\n  list  "), "{long}");
}

#[test]
fn a_compact_group_names_the_commands_that_no_goal_names_on_a_final_commands_line() {
    const GUIDE: Guide =
        Guide::new()
            .groups(&[Group::new(&[], &[Goal::new("Explore", &["planets", "moons"])]).compact()]);

    let expected = "\
Explore:   planets, moons
Commands:  constellations

Options:
";
    assert_eq!(section(&help(&GUIDE, &["tool", "-h"])), expected);
}

#[test]
fn a_compact_group_s_short_help_leaves_out_aliases_and_hidden_commands() {
    const GUIDE: Guide =
        Guide::new().groups(&[Group::new(&[], &[Goal::new("Read", &["list", "weigh"])]).compact()]);
    let tool = Command::new("tool")
        .subcommand(
            Command::new("list")
                .about("List the planets")
                .visible_alias("ls"),
        )
        .subcommand(Command::new("weigh").about("Weigh a planet").hide(true));

    let mut tree = extend(tool, &GUIDE);
    let short = tree.render_help().to_string();
    let long = tree.render_long_help().to_string();

    assert_eq!(section(&short), "Read:  list\n\nOptions:\n");
    assert_eq!(
        section(&long),
        "Read\n  list  List the planets [alias: ls]\n\nOptions:\n"
    );
}

#[test]
fn a_compact_group_s_command_drops_its_own_before_help_as_a_list_with_a_brace_does() {
    const GUIDE: Guide =
        Guide::new().groups(&[Group::new(&[], &[Goal::new("Read", &["list"])]).compact()]);
    let tool = Command::new("tool")
        .about("Explore a small solar system")
        .before_help("The tool's own short text")
        .before_long_help("The tool's own long text")
        .subcommand(Command::new("list").about("List the planets"));

    let mut tree = extend(tool, &GUIDE);
    let short = tree.render_help().to_string();
    let long = tree.render_long_help().to_string();

    assert!(
        short.starts_with(
            "Explore a small solar system\n\nUsage: tool [COMMAND]\n\nRead:  list\n\nOptions:\n"
        ),
        "{short}"
    );
    assert!(
        long.starts_with("Explore a small solar system\n\nUsage: tool [COMMAND]\n\nRead\n  list  List the planets\n\nOptions:\n"),
        "{long}"
    );
}

#[test]
fn a_compact_group_prints_a_heading_or_a_description_with_a_brace_as_written() {
    const GUIDE: Guide =
        Guide::new().groups(&[Group::new(&[], &[Goal::new("Read {all}", &["list"])]).compact()]);
    let tool = Command::new("tool").subcommand(Command::new("list").about("Print {version}"));

    let mut tree = extend(tool, &GUIDE);
    let short = tree.render_help().to_string();
    let long = tree.render_long_help().to_string();

    assert_eq!(section(&short), "Read {all}:  list\n\nOptions:\n");
    assert_eq!(
        section(&long),
        "Read {all}\n  list  Print {version}\n\nOptions:\n"
    );
}

#[cfg(feature = "color")]
#[test]
fn a_compact_line_prints_its_heading_and_each_name_in_the_root_s_styles() {
    let mut tree = extend(tool(), &PLANETS);
    let planets = tree.find_subcommand_mut("planets").unwrap();
    let styles = planets.get_styles().clone();
    let (header, literal) = (styles.get_header(), styles.get_literal());

    let page = planets.render_help().ansi().to_string();

    let line =
        format!("{header}Read:{header:#}    {literal}list{literal:#}, {literal}show{literal:#}\n");
    assert!(page.contains(&line), "{page:?}");
}
