//! Tests for how `finish_to` prints a listing's steps: each as a `Next:`
//! line with its sentence, followed by its commands indented by two spaces,
//! on standard error in text, and nowhere in JSON.

use std::process::ExitCode;

use super::{PLANET_LISTING, Planet, finish_with, mercury, saturn};
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
fn json_output_has_no_steps_and_its_document_is_unchanged() {
    let plain = finish_with(&["--json"], PLANET_LISTING, two_of_eight());
    let stepped = finish_with(
        &["--json"],
        PLANET_LISTING,
        two_of_eight().step(commit_step()),
    );

    assert_eq!(stepped.out, plain.out);
    assert_eq!(stepped.err, "");
}
