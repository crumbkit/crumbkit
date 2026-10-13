//! Tests for `--yes` and `Output::confirm_to`.

use clap::{Args as _, Command, CommandFactory as _, Parser, Subcommand};

use super::Yes;
use crate::help::Guide;
use crate::twins::assert_one_word_apart;
use crate::{Call, DryRun, Error, NoInput, Output, Recovery, Sink, Spelling};

/// A tool whose `remove` command takes `--dry-run` and `--yes`.
#[derive(Parser)]
#[command(name = "tool")]
struct Tool {
    #[command(subcommand)]
    command: Commands,
    #[command(flatten)]
    output: Output,
    #[command(flatten)]
    no_input: NoInput,
}

#[derive(Subcommand)]
enum Commands {
    Remove {
        names: Vec<String>,
        #[command(flatten)]
        dry_run: DryRun,
        #[command(flatten)]
        yes: Yes,
    },
}

/// What `confirm_to` returned and wrote.
struct Confirmed {
    result: Result<(), Error>,
    err: String,
}

/// Parses `tool` followed by `words` with crumbkit's `parse_to`, keeps the
/// parsed command, and asks "Remove Pluto?" with `input` as the caller's
/// answer, through a sink that is `interactive` or not.
fn confirm(words: &[&str], input: &str, interactive: bool) -> Confirmed {
    confirm_with(&Guide::new(), words, input, interactive)
}

/// Asks as [`confirm`] does, with the tool parsed against `guide`.
fn confirm_with(guide: &Guide, words: &[&str], input: &str, interactive: bool) -> Confirmed {
    let args = std::iter::once("tool").chain(words.iter().copied());
    let mut sink = Sink::new(Vec::new(), Vec::new());
    let (tool, parsed) = crate::parse_to::<Tool, _, _, _, _>(
        &mut sink,
        Tool::command(),
        guide,
        Spelling::COLONS,
        args,
    )
    .unwrap();
    let mut output = tool.output;
    output.set_command(parsed);
    let Commands::Remove { dry_run, yes, .. } = tool.command;
    output.dry_run(dry_run);
    let mut err = Vec::new();
    let mut sink = Sink::with_input(Vec::new(), &mut err, input.as_bytes(), interactive);
    let result = output.confirm_to(&mut sink, yes, "Remove Pluto?");
    Confirmed {
        result,
        err: String::from_utf8(err).unwrap(),
    }
}

/// Returns the code of `result`'s failure in the colon spelling.
fn code(result: &Result<(), Error>) -> String {
    let failure = result.as_ref().unwrap_err().failure();
    Spelling::COLONS.spell(failure.code())
}

#[test]
fn yes_proceeds_without_asking() {
    let confirmed = confirm(&["remove", "pluto", "--yes"], "", true);

    assert!(confirmed.result.is_ok());
    assert_eq!(confirmed.err, "");
}

#[test]
fn a_dry_run_proceeds_without_asking() {
    let confirmed = confirm(&["remove", "pluto", "--dry-run"], "", false);

    assert!(confirmed.result.is_ok());
    assert_eq!(confirmed.err, "");
}

#[test]
fn at_a_terminal_y_or_yes_in_any_case_proceeds() {
    for answer in ["y\n", "Y\n", "yes\n", " YES \r\n"] {
        let confirmed = confirm(&["remove", "pluto"], answer, true);

        assert!(confirmed.result.is_ok(), "{answer:?}");
        assert_eq!(confirmed.err, "Remove Pluto? [y/N] ");
    }
}

#[test]
fn at_a_terminal_any_other_answer_or_no_answer_declines_with_exit_code_1() {
    for answer in ["n\n", "\n", "yep\n", ""] {
        let confirmed = confirm(&["remove", "pluto"], answer, true);

        assert_eq!(
            code(&confirmed.result),
            "crumbkit::confirmation::declined",
            "{answer:?}"
        );
        let failure = confirmed.result.as_ref().unwrap_err().failure();
        assert!(matches!(failure.recovery(), Recovery::None { .. }));
    }
}

#[test]
fn without_a_terminal_the_run_fails_with_the_same_command_and_yes_as_its_recovery() {
    let refused = ["remove", "Le Guin", "--", "-7"];
    let admitted = ["remove", "Le Guin", "--yes", "--", "-7"];
    assert_one_word_apart(&refused, &admitted);
    assert!(confirm(&admitted, "", false).result.is_ok());
    let confirmed = confirm(&refused, "y\n", false);

    assert_eq!(confirmed.err, "");
    assert_eq!(code(&confirmed.result), "crumbkit::confirmation::needs_yes");
    let failure = confirmed.result.as_ref().unwrap_err().failure();
    let words = ["remove", "Le Guin", "--yes", "--", "-7"].map(str::to_owned);
    assert_eq!(
        failure.recovery(),
        Recovery::Run(Call::typed(words.to_vec()))
    );
}

#[test]
fn without_the_typed_words_the_needs_yes_failure_has_no_next_step() {
    let tool = Tool::try_parse_from(["tool", "remove", "pluto"]).unwrap();
    let mut output = tool.output;
    output.set_command(Tool::command());
    let mut sink = Sink::new(Vec::new(), Vec::new());

    let result = output.confirm_to(&mut sink, Yes::default(), "Remove Pluto?");

    assert_eq!(code(&result), "crumbkit::confirmation::needs_yes");
    assert!(matches!(
        result.unwrap_err().failure().recovery(),
        Recovery::None { .. }
    ));
}

#[test]
fn yes_is_the_flag_yes_and_serializes_as_one_field() {
    let command = Yes::augment_args(Command::new("remove"));
    let arg = command.get_arguments().next().unwrap();
    assert_eq!(arg.get_long(), Some("yes"));

    let tool = Tool::try_parse_from(["tool", "remove", "--yes"]).unwrap();
    let Commands::Remove { yes, .. } = tool.command;
    assert_eq!(serde_json::to_string(&yes).unwrap(), "{\"yes\":true}");
}

#[test]
fn dry_run_and_yes_add_their_flags_and_no_argument_group() {
    let command = Tool::command();
    let remove = command.find_subcommand("remove").unwrap();

    let groups: Vec<&str> = remove
        .get_groups()
        .map(|group| group.get_id().as_str())
        .collect();

    assert!(!groups.contains(&"DryRun"), "{groups:?}");
    assert!(!groups.contains(&"Yes"), "{groups:?}");
    assert!(
        remove
            .get_arguments()
            .any(|arg| arg.get_long() == Some("dry-run"))
    );
    assert!(
        remove
            .get_arguments()
            .any(|arg| arg.get_long() == Some("yes"))
    );
}

#[test]
fn no_input_fails_with_needs_yes_without_asking_at_a_terminal() {
    assert_one_word_apart(&["remove", "pluto", "--no-input"], &["remove", "pluto"]);
    assert!(confirm(&["remove", "pluto"], "y\n", true).result.is_ok());
    let confirmed = confirm(&["remove", "pluto", "--no-input"], "y\n", true);

    assert_eq!(code(&confirmed.result), "crumbkit::confirmation::needs_yes");
    assert_eq!(confirmed.err, "");
}

#[test]
fn with_terminal_detection_off_confirm_fails_with_needs_yes_without_asking_at_a_terminal() {
    const QUIET: Guide =
        Guide::new().without_terminal_detection("Our CI runs in a pseudo-terminal.");
    let refused = ["remove", "pluto"];
    let admitted = ["remove", "pluto", "--yes"];
    assert_one_word_apart(&refused, &admitted);
    assert!(confirm_with(&QUIET, &admitted, "", true).result.is_ok());

    let confirmed = confirm_with(&QUIET, &refused, "y\n", true);

    assert_eq!(confirmed.err, "");
    assert_eq!(code(&confirmed.result), "crumbkit::confirmation::needs_yes");
}

#[test]
fn a_tool_that_parses_with_clap_alone_keeps_the_guides_terminal_detection() {
    const QUIET: Guide =
        Guide::new().without_terminal_detection("Our CI runs in a pseudo-terminal.");
    let mut output = Output::default();
    output.set_command(crate::Parsed::guided(Tool::command(), &QUIET));
    let mut err = Vec::new();
    let mut sink = Sink::with_input(Vec::new(), &mut err, &b"y\n"[..], true);

    let result = output.confirm_to(&mut sink, Yes::default(), "Remove Pluto?");

    assert_eq!(String::from_utf8(err).unwrap(), "");
    assert_eq!(code(&result), "crumbkit::confirmation::needs_yes");
}
