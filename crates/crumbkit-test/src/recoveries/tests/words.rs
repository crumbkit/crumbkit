//! Tests for the reading of the placeholders inside one word of a step.

use super::super::chain::fill::fill_word;
use super::{NOT_FOUND, OTHERS, Setup, failure, problems, recoveries, run};
use crate::recoveries::Failing;

/// Returns the value of the names `plugin`, `capability` and `path`.
fn known(name: &str) -> Option<&'static str> {
    match name {
        "plugin" => Some("mail"),
        "capability" => Some("send"),
        "path" => Some("/tasks/a"),
        _ => None,
    }
}

#[test]
fn a_word_with_two_placeholders_and_text_between_them_fills_each_placeholder_and_keeps_the_text() {
    assert_eq!(
        fill_word("<plugin>.<capability>", known),
        Ok("mail.send".to_owned())
    );
    assert_eq!(
        fill_word("--to=<path>", known),
        Ok("--to=/tasks/a".to_owned())
    );
    assert_eq!(fill_word("<path>...", known), Ok("/tasks/a".to_owned()));
}

#[test]
fn a_less_than_sign_that_starts_no_placeholder_stays_literal_text() {
    assert_eq!(fill_word("a<b", known), Ok("a<b".to_owned()));
    assert_eq!(fill_word("<>", known), Ok("<>".to_owned()));
    assert_eq!(fill_word("<a.b>", known), Ok("<a.b>".to_owned()));
    assert_eq!(fill_word("<<path>>", known), Ok("</tasks/a>".to_owned()));
}

#[test]
fn a_word_whose_second_placeholder_has_no_value_fails_the_check_naming_that_placeholder() {
    let show = failure(
        "tool::books::not_found",
        &run("tool books list <plugin>.<capability>"),
    );
    let setup = Setup::new(
        "recoveries-word-missing",
        &[("books show 7 --json", 1, show.as_str(), "")],
    );
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"])];
    let recoveries = recoveries(&setup, &starts, OTHERS).values(&[("plugin", "mail")]);

    assert_eq!(
        problems(&setup, &recoveries).unwrap(),
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool books show 7: the step `tool books list <plugin>.<capability>` has the placeholder <capability>, and no value of Failing::values, the fields of the failure or Recoveries::values fills it"
    );
}
