//! Tests for the mount: the enum, `run_to` and the leaf functions.

use clap::{CommandFactory as _, Parser};
use crumbkit::help::Effects;
use crumbkit::{DryRun, Error, Rows, Run, Sink, Tool, ViewFlags};

use super::{SkillsCommand, check_leaf, install_leaf, print_leaf, run_to};
use crate::check::{SKILLS_CHECK, check};
use crate::install::{SKILLS_INSTALL, install};
use crate::print::{SKILLS_PRINT, print};
use crate::skill::Skill;
use crate::testing::LENDING;

const SKILL: Skill = Skill::new(
    "shelf",
    "0.3.0",
    "---\nname: shelf\ndescription: Browse the catalogue.\n---\n\nRun `shelf books list`.\n",
);

/// A tool that mounts the `skills` commands, as an adopter does.
#[derive(Debug, Parser)]
#[command(name = "shelf")]
struct Cli {
    #[command(flatten)]
    view: ViewFlags,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    Skills {
        #[command(subcommand)]
        command: SkillsCommand,
    },
}

/// Parses `shelf skills` followed by `words` and `--json`, and returns the
/// run and the command.
fn parse(words: &[&str]) -> (Run, SkillsCommand) {
    let args: Vec<&str> = ["shelf", "skills"]
        .iter()
        .chain(words)
        .chain(&["--json"])
        .copied()
        .collect();
    let cli = Cli::try_parse_from(&args).unwrap();
    let run = Run::new(
        cli.view,
        DryRun::default(),
        Cli::command(),
        &Tool::new(),
        &args,
    )
    .unwrap();
    let Command::Skills { command } = cli.command;
    (run, command)
}

/// Runs `words` through `run_to` with two skills and returns the exit code
/// and the output.
fn mounted(words: &[&str]) -> (u8, String) {
    let (run, command) = parse(words);
    let mut out = Vec::new();
    let code = run_to(
        &mut Sink::new(&mut out, Vec::new()),
        run,
        &[SKILL, LENDING],
        &command,
    );
    (code, String::from_utf8(out).unwrap())
}

/// Runs `words` through `finish_to` with `value`, as a tool's own match does.
fn finished<T: Rows>(words: &[&str], document: crumbkit::Document<T>, value: T) -> (u8, String) {
    let (run, _) = parse(words);
    let mut out = Vec::new();
    let code = crumbkit::finish_to(
        &mut Sink::new(&mut out, Vec::new()),
        run,
        document,
        Ok::<_, Error>(value),
    );
    (code, String::from_utf8(out).unwrap())
}

#[test]
fn each_command_parses_with_its_arguments() {
    let (_, install_command) = parse(&["install", "--dir", "/tmp/skills", "--force"]);
    let SkillsCommand::Install(args) = install_command else {
        panic!("not install");
    };
    assert_eq!(
        format!("{args:?}"),
        "SkillsInstall { dir: Some(\"/tmp/skills\"), force: true }"
    );
    let (_, check_command) = parse(&["check", "--dir", "/tmp/skills"]);
    assert!(matches!(check_command, SkillsCommand::Check(_)));
    let (_, print_command) = parse(&["print"]);
    assert!(matches!(print_command, SkillsCommand::Print(_)));
    assert!(Cli::try_parse_from(["shelf", "skills", "print", "shelf"]).is_err());
}

#[test]
fn install_and_check_print_what_the_functions_return() {
    let home = tempfile::tempdir().unwrap();
    let dir = home.path().to_str().unwrap();
    let words = ["install", "--dir", dir];
    let (_, command) = parse(&words);
    let SkillsCommand::Install(args) = command else {
        panic!("not install");
    };
    let (code, written) = mounted(&words);
    assert_eq!(code, 0);
    assert!(written.contains("\"state\":\"written\""));
    assert!(home.path().join("shelf/SKILL.md").exists());
    assert!(home.path().join("shelf-lending/SKILL.md").exists());
    // A second install leaves every copy as it is, which `finish_to` prints
    // the same for the function's result.
    let expected = finished(&words, SKILLS_INSTALL, install(&[SKILL, LENDING], &args));
    assert_eq!(mounted(&words), expected);

    let words = ["check", "--dir", dir];
    let (_, command) = parse(&words);
    let SkillsCommand::Check(args) = command else {
        panic!("not check");
    };
    let expected = finished(&words, SKILLS_CHECK, check(&[SKILL, LENDING], &args));
    assert_eq!(mounted(&words), expected);
}

#[test]
fn print_prints_the_first_skill() {
    let expected = finished(&["print"], SKILLS_PRINT, print(&SKILL));
    assert_eq!(mounted(&["print"]), expected);
}

#[test]
fn each_leaf_has_the_path_document_and_effects_of_its_command() {
    let install = install_leaf(&["skills", "install"]);
    assert_eq!(install.path(), ["skills", "install"]);
    assert_eq!(
        install.get_effects(),
        Some(Effects::additive().idempotent())
    );
    assert_eq!(
        install.get_document().unwrap().document().name(),
        "skills-install"
    );

    let check = check_leaf(&["crumbkit", "skills", "check"]);
    assert_eq!(check.path(), ["crumbkit", "skills", "check"]);
    assert_eq!(check.get_effects(), Some(Effects::read_only()));
    assert_eq!(
        check.get_document().unwrap().document().name(),
        "skills-check"
    );

    let print = print_leaf(&["skills", "print"]);
    assert_eq!(print.get_effects(), Some(Effects::read_only()));
    assert_eq!(
        print.get_document().unwrap().document().name(),
        "skills-print"
    );
}
