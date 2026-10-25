use super::{Program, Step, StepCommand};
use crate::Call;

/// A program that the tests name in their steps.
const GIT: Program = Program::new("git");

#[test]
fn a_step_holds_its_sentence_and_its_commands_in_the_order_they_were_added() {
    let step = Step::new("Commit the change.")
        .run(GIT, ["commit", "-m", "Add books"])
        .call(Call::new(&["books", "list"]));

    assert_eq!(step.sentence, "Commit the change.");
    assert_eq!(
        step.commands,
        [
            StepCommand::Program {
                program: GIT,
                args: vec!["commit".to_owned(), "-m".to_owned(), "Add books".to_owned()],
            },
            StepCommand::Call(Call::new(&["books", "list"])),
        ]
    );
}

#[test]
fn a_program_command_renders_its_name_and_quotes_each_argument_for_a_shell() {
    let args = ["use".to_owned(), "ruby@3.4".to_owned(), "it's".to_owned()];

    let line = super::program_line(Program::new("mise"), &args);

    assert_eq!(line, r"mise use ruby@3.4 'it'\''s'");
}

#[test]
fn a_program_name_may_hold_letters_digits_and_the_marks_dash_underscore_dot_and_plus() {
    let program = Program::new("g++-14.2_x");

    assert_eq!(program.name, "g++-14.2_x");
}

#[test]
#[should_panic(expected = "a program name is one word")]
fn an_empty_program_name_panics() {
    let _ = Program::new("");
}

#[test]
#[should_panic(expected = "a program name is one word")]
fn a_program_name_with_a_space_panics_so_its_arguments_stay_separate() {
    let _ = Program::new("git commit");
}

#[test]
#[should_panic(expected = "a program name is one word")]
fn a_program_name_with_a_slash_panics_so_a_step_names_no_path() {
    let _ = Program::new("/usr/bin/git");
}

#[test]
#[should_panic(expected = "a program name is one word")]
fn a_program_name_that_starts_with_a_dash_panics_so_it_never_reads_as_a_flag() {
    let _ = Program::new("-git");
}
