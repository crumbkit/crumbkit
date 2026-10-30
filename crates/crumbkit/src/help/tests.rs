//! Tests for goal headings: the help pages that `parse_to` prints with a
//! guide, and the tree that `extend` returns.

use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};

use super::{Goal, Group, Guide, extend};
use crate::{Sink, parse_to};

mod compact;
mod root_verbs;

/// A tool with a global flag and three groups. The names have different
/// lengths, `planets` has a hidden command, and `moons` has a command
/// without a description.
fn tool() -> Command {
    Command::new("tool")
        .about("Explore a small solar system")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print JSON"),
        )
        .subcommand(
            Command::new("planets")
                .about("Work with planets")
                .subcommand_required(true)
                .subcommand(Command::new("list").about("List the planets"))
                .subcommand(Command::new("show").about("Show planets by name"))
                .subcommand(Command::new("rename").about("Rename a planet"))
                .subcommand(Command::new("weigh").about("Weigh a planet").hide(true)),
        )
        .subcommand(
            Command::new("moons")
                .about("Work with moons")
                .subcommand_required(true)
                .subcommand(Command::new("list").about("List the moons"))
                .subcommand(Command::new("forget")),
        )
        .subcommand(
            Command::new("constellations")
                .about("Work with constellations")
                .subcommand_required(true)
                .subcommand(Command::new("list").about("List the constellations")),
        )
}

/// Runs `parse_to` over `tool()` with `guide`, for `args` that ask for help,
/// and returns the help page it printed.
fn help(guide: &Guide, args: &[&str]) -> String {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result =
        parse_to::<(), _, _, _, _>(&mut Sink::new(&mut out, &mut err), tool(), guide, args);
    assert_eq!(result.err().map(ExitCode::from), Some(ExitCode::SUCCESS));
    assert_eq!(err, b"");
    String::from_utf8(out).unwrap()
}

/// Returns the part of a help page after its usage line, up to and including
/// its `Options:` heading.
fn section(page: &str) -> &str {
    let usage = page.find("Usage:").unwrap();
    let start = usage + page[usage..].find("\n\n").unwrap() + 2;
    let end = page.find("Options:\n").unwrap() + "Options:\n".len();
    &page[start..end]
}

#[test]
fn a_guided_group_lists_its_children_under_each_goal_heading_in_guide_order() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &[],
        &[
            Goal::new("Explore", &["planets", "moons"]),
            Goal::new("Map", &["constellations"]),
        ],
    )]);

    let expected = "\
Explore a small solar system

Usage: tool [OPTIONS] <COMMAND>

Explore
  planets         Work with planets
  moons           Work with moons

Map
  constellations  Work with constellations

Options:
  -h, --help  Print help (see more with '--help')

Global flags: --json. `tool --help` describes them.
Topics: exit-codes. `tool help <TOPIC>` prints one.
Next: tool <COMMAND> -h
";
    assert_eq!(help(&GUIDE, &["tool", "-h"]), expected);
}

#[test]
fn a_guided_nested_group_lists_its_children_under_its_own_goals() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &["planets"],
        &[
            Goal::new("Read", &["list", "show"]),
            Goal::new("Change", &["rename"]),
        ],
    )]);

    let expected = "\
Read
  list    List the planets
  show    Show planets by name

Change
  rename  Rename a planet

Options:
";
    assert_eq!(section(&help(&GUIDE, &["tool", "planets", "-h"])), expected);
}

#[test]
fn a_guided_groups_long_help_lists_the_goal_headings_before_the_options() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &["planets"],
        &[
            Goal::new("Read", &["list", "show"]),
            Goal::new("Change", &["rename"]),
        ],
    )]);

    let expected = "\
Work with planets

Usage: tool planets [OPTIONS] <COMMAND>

Read
  list    List the planets
  show    Show planets by name

Change
  rename  Rename a planet

Options:
      --json
          Print JSON

  -h, --help
          Print help (see a summary with '-h')

Next: tool planets <COMMAND> -h
";
    assert_eq!(help(&GUIDE, &["tool", "planets", "--help"]), expected);
}

#[test]
fn a_guided_group_off_the_typed_path_keeps_its_default_help() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &["planets"],
        &[Goal::new("Read", &["list", "show", "rename"])],
    )]);

    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(Vec::new(), Vec::new()),
        tool(),
        &GUIDE,
        ["tool", "moons", "list"],
    );

    let Ok(((), mut command)) = result else {
        panic!("the command line did not parse");
    };
    let mut planets = command
        .command()
        .find_subcommand("planets")
        .unwrap()
        .clone();
    let page = planets.render_help().to_string();
    assert!(page.contains("Commands:\n"), "{page}");
    assert!(!page.contains("Read\n"), "{page}");
}

#[test]
fn a_guided_command_prints_its_own_before_help_above_its_about_and_the_goal_list_below_its_usage() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(&[], &[Goal::new("Read", &["list"])])]);
    let tool = Command::new("tool")
        .about("Explore a small solar system")
        .before_help("The tool's own short text")
        .before_long_help("The tool's own long text")
        .subcommand(Command::new("list").about("List the planets"));

    let page = extend(tool, &GUIDE).render_help().to_string();

    let expected = "\
The tool's own short text

Explore a small solar system

Usage: tool [COMMAND]

Read
  list  List the planets

Options:
  -h, --help  Print help (see more with '--help')
";
    assert_eq!(page, expected);
}

#[test]
fn a_guided_command_prints_its_own_before_long_help_in_its_long_help() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(&[], &[Goal::new("Read", &["list"])])]);
    let tool = Command::new("tool")
        .about("Explore a small solar system")
        .before_help("The tool's own short text")
        .before_long_help("The tool's own long text")
        .subcommand(Command::new("list").about("List the planets"));

    let page = extend(tool, &GUIDE).render_long_help().to_string();

    assert!(
        page.starts_with("The tool's own long text\n\nExplore a small solar system\n\nUsage: tool [COMMAND]\n\nRead\n  list  List the planets\n\nOptions:\n"),
        "{page}"
    );
}

#[test]
fn a_guided_command_whose_list_holds_a_brace_keeps_the_list_in_before_help() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(&[], &[Goal::new("Read", &["list"])])]);
    let tool = Command::new("tool")
        .before_help("The tool's own short text")
        .subcommand(Command::new("list").about(r#"Print {"a": {"b": 1}} and {version}"#));

    let page = extend(tool, &GUIDE).render_help().to_string();

    let expected = r#"Usage: tool [COMMAND]

Read
  list  Print {"a": {"b": 1}} and {version}

Options:
  -h, --help  Print help
"#;
    assert_eq!(page, expected);
}

#[test]
fn a_guided_page_without_long_help_keeps_clap_s_short_help_line() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(&[], &[Goal::new("Read", &["list"])])]);
    let tool = Command::new("tool").subcommand(Command::new("list").about("List the planets"));

    let page = extend(tool, &GUIDE).render_help().to_string();

    assert!(
        page.ends_with("Options:\n  -h, --help  Print help\n"),
        "{page}"
    );
}

#[test]
fn a_visible_command_that_no_goal_names_is_listed_under_a_final_commands_heading() {
    const GUIDE: Guide =
        Guide::new().groups(&[Group::new(&["planets"], &[Goal::new("Read", &["list"])])]);

    let expected = "\
Read
  list    List the planets

Commands
  show    Show planets by name
  rename  Rename a planet

Options:
";
    assert_eq!(section(&help(&GUIDE, &["tool", "planets", "-h"])), expected);
}

#[test]
fn a_goal_leaves_out_a_hidden_command() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &["planets"],
        &[Goal::new("Read", &["list", "show", "weigh", "rename"])],
    )]);

    let expected = "\
Read
  list    List the planets
  show    Show planets by name
  rename  Rename a planet

Options:
";
    assert_eq!(section(&help(&GUIDE, &["tool", "planets", "-h"])), expected);
}

#[test]
fn a_goal_leaves_out_a_name_that_is_not_a_child_of_its_group() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &["planets"],
        &[Goal::new("Read", &["list", "lsit", "show", "rename"])],
    )]);

    let expected = "\
Read
  list    List the planets
  show    Show planets by name
  rename  Rename a planet

Options:
";
    assert_eq!(section(&help(&GUIDE, &["tool", "planets", "-h"])), expected);
}

#[test]
fn a_goal_whose_commands_are_all_hidden_prints_no_heading() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &["planets"],
        &[
            Goal::new("Read", &["list", "show", "rename"]),
            Goal::new("Measure", &["weigh"]),
        ],
    )]);

    let expected = "\
Read
  list    List the planets
  show    Show planets by name
  rename  Rename a planet

Options:
";
    assert_eq!(section(&help(&GUIDE, &["tool", "planets", "-h"])), expected);
}

#[test]
fn a_command_without_a_description_ends_its_line_at_its_name() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &["moons"],
        &[
            Goal::new("Read", &["list"]),
            Goal::new("Change", &["forget"]),
        ],
    )]);

    let expected = "\
Read
  list    List the moons

Change
  forget

Options:
";
    assert_eq!(section(&help(&GUIDE, &["tool", "moons", "-h"])), expected);
}

#[test]
fn a_group_the_guide_does_not_name_keeps_clap_s_commands_list() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &[],
        &[Goal::new(
            "Explore",
            &["planets", "moons", "constellations"],
        )],
    )]);

    let expected = "\
Commands:
  list    List the planets
  show    Show planets by name
  rename  Rename a planet

Options:
";
    assert_eq!(section(&help(&GUIDE, &["tool", "planets", "-h"])), expected);
}

#[test]
fn a_guide_path_that_names_no_command_or_a_command_without_subcommands_changes_nothing() {
    const GUIDE: Guide = Guide::new().groups(&[
        Group::new(&["comets"], &[Goal::new("Read", &["list"])]),
        Group::new(&["planets", "list"], &[Goal::new("Read", &["list"])]),
    ]);

    for args in [
        ["tool", "-h"].as_slice(),
        &["tool", "planets", "list", "-h"],
    ] {
        assert_eq!(help(&GUIDE, args), help(&Guide::new(), args), "{args:?}");
    }
}

#[test]
fn a_guided_list_prints_each_visible_alias_after_the_description_as_clap_s_list_does() {
    const GUIDE: Guide = Guide::new().groups(&[Group::new(
        &["moons"],
        &[Goal::new("Read", &["list", "forget"])],
    )]);
    let tool = tool().mut_subcommand("moons", |moons| {
        moons
            .mut_subcommand("list", |list| {
                list.visible_aliases(["ls", "l"]).alias("hidden")
            })
            .mut_subcommand("forget", |forget| forget.visible_alias("drop"))
    });
    let page = |guide: &Guide| {
        let mut out = Vec::new();
        let _ = parse_to::<(), _, _, _, _>(
            &mut Sink::new(&mut out, Vec::new()),
            tool.clone(),
            guide,
            ["tool", "moons", "-h"],
        );
        String::from_utf8(out).unwrap()
    };

    let expected = "\
Read
  list    List the moons [aliases: ls, l]
  forget  [alias: drop]

Options:
";
    assert_eq!(section(&page(&GUIDE)), expected);
    let clap = page(&Guide::new());
    assert!(
        clap.contains("  list    List the moons [aliases: ls, l]\n"),
        "{clap}"
    );
    assert!(clap.contains("  forget  [alias: drop]\n"), "{clap}");
}

/// A picker that picks nothing.
struct Nobody;

impl crate::pick::Picker for Nobody {
    fn pick(
        &self,
        _question: &crate::pick::Question<'_>,
        _input: &mut dyn std::io::BufRead,
        _output: &mut dyn std::io::Write,
    ) -> std::io::Result<crate::pick::Answer> {
        Ok(crate::pick::Answer::Cancelled)
    }
}

static NOBODY: Nobody = Nobody;

#[test]
fn a_guide_has_no_picker_until_its_builder_sets_one() {
    assert!(Guide::new().get_picker().is_none());
    assert!(Guide::new().picker(&NOBODY).get_picker().is_some());
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "a choices source returns a `Result`"
)]
fn planets() -> Result<Vec<crate::pick::Choice>, crate::Error> {
    Ok(Vec::new())
}

#[test]
fn a_leaf_keeps_the_choices_that_its_builder_sets() {
    const CHOICES: &[crate::pick::Choices] = &[crate::pick::Choices::new("name", planets)];
    let leaf = super::Leaf::new(&["planets", "show"]);
    assert!(leaf.get_choices().is_empty());
    assert_eq!(leaf.choices(CHOICES).get_choices()[0].arg(), "name");
}
