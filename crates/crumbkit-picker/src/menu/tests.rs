//! Tests for the menu: what it shows and how it reads each answer.

use crumbkit::pick::{Answer, Choice, Picker, Question};

use super::Menu;

/// Returns choices with the values `1` to `count`, labelled `book 1` and so
/// on.
fn books(count: usize) -> Vec<Choice> {
    (1..=count)
        .map(|id| Choice::new(id.to_string(), format!("book {id}")))
        .collect()
}

/// What the menu answered and wrote.
struct Asked {
    answer: Answer,
    shown: String,
}

/// Asks `menu` about `question`, with `input` as the person's typing.
fn ask(menu: Menu, question: &Question<'_>, input: &str) -> Asked {
    let mut shown = Vec::new();
    let answer = menu
        .pick(question, &mut input.as_bytes(), &mut shown)
        .unwrap();
    Asked {
        answer,
        shown: String::from_utf8(shown).unwrap(),
    }
}

#[test]
fn a_number_picks_the_choice_that_the_menu_shows_with_it() {
    let choices = books(3);
    let question = Question::new("shelf books show", "ID", &choices).help("The book to show");
    let asked = ask(Menu::new(), &question, "2\n");

    assert_eq!(asked.answer, Answer::Picked(vec![1]));
    assert_eq!(
        asked.shown,
        "shelf books show needs ID: The book to show\n\
         \x20 1. book 1\n\
         \x20 2. book 2\n\
         \x20 3. book 3\n\
         Type a number to pick, text to narrow the list, or nothing to cancel: "
    );
}

#[test]
fn text_narrows_the_list_and_a_number_then_picks_from_the_narrowed_list() {
    let choices = books(12);
    let question = Question::new("shelf books show", "ID", &choices);
    let asked = ask(Menu::new(), &question, "BOOK 1\n3\n");

    // `book 1`, `book 10`, `book 11` and `book 12` match, and the third is
    // `book 11`.
    assert_eq!(asked.answer, Answer::Picked(vec![10]));
    assert!(asked.shown.contains("  4. book 12\n"), "{}", asked.shown);
}

#[test]
fn text_that_matches_nothing_says_so_and_shows_the_whole_list_again() {
    let choices = books(2);
    let question = Question::new("shelf books show", "ID", &choices);
    let asked = ask(Menu::new(), &question, "dune\n1\n");

    assert_eq!(asked.answer, Answer::Picked(vec![0]));
    assert!(asked.shown.contains("No choice matches `dune`.\n"));
}

#[test]
fn an_empty_line_or_the_end_of_input_cancels() {
    let choices = books(2);
    let question = Question::new("shelf books show", "ID", &choices);

    assert_eq!(ask(Menu::new(), &question, "\n").answer, Answer::Cancelled);
    assert_eq!(ask(Menu::new(), &question, "").answer, Answer::Cancelled);
}

#[test]
fn several_numbers_pick_several_choices_when_the_question_allows_them() {
    let choices = books(3);
    let question = Question::new("shelf books show", "ID", &choices).count(1, usize::MAX);
    let asked = ask(Menu::new(), &question, "3 1\n");

    assert_eq!(asked.answer, Answer::Picked(vec![2, 0]));
    assert!(asked.shown.ends_with(
        "Type numbers separated by spaces to pick, text to narrow the list, or nothing to cancel: "
    ));
}

#[test]
fn a_wrong_count_or_a_number_that_the_menu_does_not_show_asks_again() {
    let choices = books(3);
    let question = Question::new("shelf books show", "ID", &choices);
    let asked = ask(Menu::new(), &question, "1 2\n4\n3\n");

    assert_eq!(asked.answer, Answer::Picked(vec![2]));
    assert!(asked.shown.contains("Pick exactly 1.\n"));
    assert!(asked.shown.contains("Pick numbers from 1 to 3.\n"));
}

#[test]
fn a_long_list_shows_its_first_rows_and_says_how_many_more_match() {
    let choices = books(5);
    let question = Question::new("shelf books show", "ID", &choices);
    let asked = ask(Menu::new().rows(2), &question, "\n");

    assert!(
        asked
            .shown
            .contains("  2. book 2\n3 more match. Type text to narrow the list.\n")
    );
    assert!(!asked.shown.contains("book 3"));
}

#[test]
fn a_control_character_in_a_label_shows_as_its_escape() {
    let choices = [Choice::new("1".to_owned(), "two\nlines\x1b[31m".to_owned())];
    let question = Question::new("shelf books show", "ID", &choices);
    let asked = ask(Menu::new(), &question, "\n");

    assert!(
        asked.shown.contains("  1. two\\nlines\\u{1b}[31m\n"),
        "{}",
        asked.shown
    );
}
