//! Tests for the types that a picker receives and returns, and for
//! `from_rows`.

use schemars::JsonSchema;
use serde::Serialize;

use super::{Answer, Choice, Choices, Picker, Question, from_rows};
use crate::{Error, Row};

/// A book with an id, a title and a long column.
#[derive(Serialize, JsonSchema)]
struct Book {
    id: u32,
    title: &'static str,
    author: Option<&'static str>,
    pages: u32,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id", "title", "author"];
    const LONG_COLUMNS: &'static [&'static str] = &["pages"];
}

fn books() -> Vec<Book> {
    vec![
        Book {
            id: 1,
            title: "Dune",
            author: Some("Frank Herbert"),
            pages: 412,
        },
        Book {
            id: 2,
            title: "Two\nlines",
            author: None,
            pages: 9,
        },
    ]
}

#[test]
fn from_rows_takes_each_value_from_the_id_field_and_labels_it_with_the_other_compact_columns() {
    let choices = from_rows(&books(), "id");
    let pairs: Vec<(&str, &str)> = choices
        .iter()
        .map(|choice| (choice.value(), choice.label()))
        .collect();
    assert_eq!(
        pairs,
        [("1", "Dune  Frank Herbert"), ("2", "Two\\nlines  -")]
    );
}

#[test]
fn from_rows_skips_a_row_whose_id_field_is_missing() {
    assert_eq!(from_rows(&books(), "isbn").len(), 0);
}

/// A row whose id is a string.
#[derive(Serialize, JsonSchema)]
struct List {
    name: &'static str,
}

impl Row for List {
    const COLUMNS: &'static [&'static str] = &["name"];
}

#[test]
fn from_rows_keeps_a_string_id_as_it_is_and_labels_a_row_without_other_columns_with_its_value() {
    let choices = from_rows(&[List { name: "to read" }], "name");
    assert_eq!(choices[0].value(), "to read");
    assert_eq!(choices[0].label(), "to read");
}

#[test]
fn a_new_question_asks_for_exactly_one_value_and_has_no_help() {
    let choices = [Choice::new("1".to_owned(), "Dune".to_owned())];
    let question = Question::new("shelf books show", "ID", &choices);
    assert_eq!(question.command(), "shelf books show");
    assert_eq!(question.name(), "ID");
    assert_eq!(question.get_help(), None);
    assert_eq!((question.least(), question.most()), (1, 1));
    assert_eq!(question.choices().len(), 1);
}

#[test]
fn a_question_keeps_the_help_and_the_count_that_its_builders_set() {
    let question = Question::new("shelf books show", "ID", &[])
        .help("The books to show")
        .count(1, usize::MAX);
    assert_eq!(question.get_help(), Some("The books to show"));
    assert_eq!((question.least(), question.most()), (1, usize::MAX));
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "a choices source returns a `Result`"
)]
fn one_choice() -> Result<Vec<Choice>, Error> {
    Ok(vec![Choice::new("a".to_owned(), "A".to_owned())])
}

#[test]
fn choices_name_their_argument_and_call_their_source() {
    const CHOICES: Choices = Choices::new("list", one_choice);
    assert_eq!(CHOICES.arg(), "list");
    assert_eq!(CHOICES.load().unwrap()[0].value(), "a");
}

/// A picker that always picks the last choice.
struct Last;

impl Picker for Last {
    fn pick(
        &self,
        question: &Question<'_>,
        _input: &mut dyn std::io::BufRead,
        _output: &mut dyn std::io::Write,
    ) -> std::io::Result<Answer> {
        Ok(Answer::Picked(vec![question.choices().len() - 1]))
    }
}

#[test]
fn a_picker_debugs_as_its_trait_name() {
    let picker: &dyn Picker = &Last;
    assert_eq!(format!("{picker:?}"), "Picker");
}

/// A ticket number, which reads `#7` in text and is the number `7` in JSON.
struct Ticket(u32);

impl std::fmt::Display for Ticket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl crate::Shown for Ticket {
    fn json<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.0)
    }

    fn schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        u32::json_schema(generator)
    }
}

crate::shown!(Ticket);

/// A run of a job, whose id and length are shown values.
#[derive(Serialize, JsonSchema)]
struct Run {
    id: Ticket,
    took: crate::Duration,
}

impl Row for Run {
    const COLUMNS: &'static [&'static str] = &["id", "took"];
}

#[test]
fn from_rows_labels_a_choice_with_the_text_form_and_keeps_the_json_form_as_its_value() {
    let runs = [Run {
        id: Ticket(7),
        took: crate::Duration(std::time::Duration::from_secs(250)),
    }];

    let choices = from_rows(&runs, "id");

    let pairs: Vec<(&str, &str)> = choices
        .iter()
        .map(|choice| (choice.value(), choice.label()))
        .collect();
    assert_eq!(pairs, [("7", "4m 10s")]);
}
