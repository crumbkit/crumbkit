//! Tests for how `finish_to` prints a listing's steps: each as a `Next:`
//! line with its sentence, followed by its commands indented by two spaces,
//! on standard error in text, and under `next` in the JSON document.

use std::process::ExitCode;

use clap::CommandFactory as _;
use serde_json::Value;

use super::{
    Cli, PLANET_LISTING, Planet, finish_output, finish_with, mercury, parse_against, saturn,
};
use crate::conforms::conforms;
use crate::schemas::document_schema;
use crate::{Call, Listing, Program, Step};

/// The program that the tests' steps run beside the tool's own commands.
const GIT: Program = Program::new("git");

/// Returns a listing of Mercury and Saturn out of 8 planets.
fn two_of_eight() -> Listing<Planet> {
    Listing::new(vec![mercury(), saturn()], 8)
}

/// Returns a step that commits the planets and then shows them.
fn commit_step() -> Step {
    Step::new("Commit the planets, then show them.")
        .run(GIT, ["commit", "-m", "Add planets"])
        .call(Call::new(&["show"]).arg("names"))
}

#[test]
fn a_step_prints_its_sentence_on_a_next_line_and_its_commands_indented_by_two_spaces() {
    let finished = finish_with(&[], PLANET_LISTING, two_of_eight().step(commit_step()));

    let expected = "\
Showing 2 of 8. --all shows every one.
Next: Commit the planets, then show them.
  git commit -m 'Add planets'
  tool show <NAME>...
";
    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.err, expected);
}

#[test]
fn steps_print_in_the_order_they_were_added_after_the_next_line_and_the_note() {
    let listing = two_of_eight()
        .note("Planets from the 2006 definition.")
        .next(Call::new(&["show"]).arg("names"), "shows planets by name.")
        .step(commit_step())
        .step(Step::new("Push the commit.").run(GIT, ["push"]));

    let finished = finish_with(&[], PLANET_LISTING, listing);

    let expected = "\
Showing 2 of 8. --all shows every one.
Planets from the 2006 definition.
Next: tool show <NAME>... shows planets by name.
Next: Commit the planets, then show them.
  git commit -m 'Add planets'
  tool show <NAME>...
Next: Push the commit.
  git push
";
    assert_eq!(finished.err, expected);
}

#[test]
fn a_step_without_commands_prints_its_sentence_alone() {
    let listing = two_of_eight().step(Step::new("Restart your shell."));

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(
        finished.err,
        "Showing 2 of 8. --all shows every one.\nNext: Restart your shell.\n"
    );
}

#[test]
fn a_step_with_a_call_that_does_not_resolve_is_left_out_whole_and_the_exit_code_is_0() {
    let broken = Step::new("Remove a planet, then push.")
        .call(Call::new(&["remove"]))
        .run(GIT, ["push"]);
    let listing = two_of_eight()
        .step(broken)
        .step(Step::new("Push the commit.").run(GIT, ["push"]));

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(
        finished.err,
        "Showing 2 of 8. --all shows every one.\nNext: Push the commit.\n  git push\n"
    );
}

#[test]
fn json_holds_the_next_step_and_then_each_step_under_next_with_its_text_and_commands() {
    let listing = two_of_eight()
        .next(Call::new(&["show"]).arg("names"), "shows planets by name.")
        .step(commit_step())
        .step(Step::new("Restart your shell."));

    let finished = finish_with(&["--json"], PLANET_LISTING, listing);

    let expected = concat!(
        r#"{"schema":"solar/planet-listing/v1","#,
        r#""data":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}],"#,
        r#""total":8,"truncated":true,"next":["#,
        r#"{"text":"shows planets by name.","commands":[{"call":"tool show <NAME>..."}]},"#,
        r#"{"text":"Commit the planets, then show them.","commands":[{"run":"git commit -m 'Add planets'"},{"call":"tool show <NAME>..."}]},"#,
        r#"{"text":"Restart your shell.","commands":[]}]}"#,
        "\n"
    );
    assert_eq!(finished.out, expected);
    assert_eq!(finished.err, "");
}

#[test]
fn the_json_of_a_listing_with_a_next_step_and_steps_matches_the_document_schema() {
    let listing = two_of_eight()
        .next(Call::new(&["show"]).arg("names"), "shows planets by name.")
        .step(commit_step())
        .step(Step::new("Restart your shell."));

    let finished = finish_with(&["--json"], PLANET_LISTING, listing);

    let document: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(document["next"].as_array().map(Vec::len), Some(3));
    let schema = document_schema(PLANET_LISTING).to_value();
    assert_eq!(conforms(&schema, &document), Ok(()));
}

#[test]
fn a_step_that_names_a_hidden_command_is_left_out_of_the_text_and_of_next() {
    let hidden = clap::Command::new("purge").hide(true);
    let flags = |flags: &[&str]| parse_against(Cli::command().subcommand(hidden.clone()), flags);
    let listing = || {
        two_of_eight()
            .step(Step::new("Purge the planets.").call(Call::new(&["purge"])))
            .step(Step::new("Push the commit.").run(GIT, ["push"]))
    };

    let text = finish_output(flags(&[]), PLANET_LISTING, listing());
    let json = finish_output(flags(&["--json"]), PLANET_LISTING, listing());

    assert_eq!(
        text.err,
        "Showing 2 of 8. --all shows every one.\nNext: Push the commit.\n  git push\n"
    );
    assert!(
        json.out.ends_with(
            r#""next":[{"text":"Push the commit.","commands":[{"run":"git push"}]}]}
"#
        ),
        "{}",
        json.out
    );
}

#[test]
fn a_step_whose_argument_holds_a_control_character_is_left_out_whole_and_json_has_no_next() {
    let unsafe_step = Step::new("Commit the planets.").run(GIT, ["commit", "-m", "a\u{1b}[2Jb"]);
    let listing = || two_of_eight().step(unsafe_step.clone());

    let text = finish_with(&[], PLANET_LISTING, listing());
    let json = finish_with(&["--json"], PLANET_LISTING, listing());
    let plain = finish_with(&["--json"], PLANET_LISTING, two_of_eight());

    assert_eq!(text.code, ExitCode::SUCCESS);
    assert_eq!(text.err, "Showing 2 of 8. --all shows every one.\n");
    assert_eq!(json.out, plain.out);
}

#[test]
fn steps_print_before_the_dry_run_line() {
    let mut output = parse_against(Cli::command(), &[]);
    output.dry_run = true;
    let listing = two_of_eight().step(Step::new("Push the commit.").run(GIT, ["push"]));

    let finished = finish_output(output, PLANET_LISTING, listing);

    let expected = "\
Showing 2 of 8. --all shows every one.
Next: Push the commit.
  git push
This was a dry run, so nothing changed.
";
    assert_eq!(finished.err, expected);
}
