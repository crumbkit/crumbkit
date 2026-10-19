//! Tests for the dashed form of a command's name.

use std::ffi::OsStr;

use clap::Command;

use super::{dashed_form, dashed_name, is_dashed, subcommand};

/// Returns a command named `name` with one subcommand, so it is a resource.
fn resource(name: &'static str) -> Command {
    Command::new(name).subcommand(Command::new("list"))
}

#[test]
fn a_dashed_name_is_lowercase_letters_and_digits_in_words_joined_by_single_dashes() {
    for name in ["books", "reading-lists", "books2", "b2-go"] {
        assert!(is_dashed(name), "{name}");
    }
    for name in [
        "",
        "Books",
        "reading_lists",
        "2books",
        "-books",
        "books-",
        "reading--lists",
        "bücher",
    ] {
        assert!(!is_dashed(name), "{name}");
    }
}

#[test]
fn names_in_snake_camel_and_pascal_case_map_to_lowercase_words_joined_by_dashes() {
    for (name, dashed) in [
        ("reading_lists", "reading-lists"),
        ("readingLists", "reading-lists"),
        ("ReadingLists", "reading-lists"),
        ("Books", "books"),
        ("books2Go", "books2-go"),
        ("reading-lists", "reading-lists"),
    ] {
        assert_eq!(dashed_name(name).as_deref(), Some(dashed), "{name}");
    }
}

#[test]
fn a_name_that_starts_with_a_digit_or_a_dash_or_holds_an_empty_word_maps_to_nothing() {
    for name in ["2books", "-books", "books_", "reading__lists", "bücher", ""] {
        assert_eq!(dashed_name(name), None, "{name}");
    }
}

#[test]
fn a_subcommand_answers_to_its_dashed_form_when_no_other_subcommand_has_or_maps_to_it() {
    let tool = Command::new("shelf")
        .subcommand(resource("reading_lists"))
        .subcommand(resource("Books"))
        .subcommand(resource("authors"));

    assert_eq!(
        dashed_form(&tool, "reading_lists").as_deref(),
        Some("reading-lists")
    );
    assert_eq!(dashed_form(&tool, "Books").as_deref(), Some("books"));
    assert_eq!(dashed_form(&tool, "authors"), None);
    assert_eq!(
        subcommand(&tool, OsStr::new("reading-lists")).map(Command::get_name),
        Some("reading_lists")
    );
}

#[test]
fn a_dashed_form_that_another_subcommand_has_or_maps_to_or_that_is_help_reaches_nothing() {
    let tool = Command::new("shelf")
        .subcommand(resource("reading_lists"))
        .subcommand(resource("readingLists"))
        .subcommand(resource("Books"))
        .subcommand(resource("authors").alias("books"))
        .subcommand(resource("Help"));

    for (name, form) in [
        ("reading_lists", "reading-lists"),
        ("Books", "books"),
        ("Help", "help"),
    ] {
        assert_eq!(dashed_form(&tool, name), None, "{name}");
        assert_ne!(
            subcommand(&tool, OsStr::new(form)).map(Command::get_name),
            Some(name),
            "{form}"
        );
    }
}
