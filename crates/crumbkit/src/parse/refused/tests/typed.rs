//! Tests for the flags and the values that a `Refused` reports when
//! crumbkit turned the word `help` into `--help`: they come from the words
//! that the caller typed (ADR-0136).

use std::ffi::OsStr;

use super::{walked, words};

#[test]
fn a_refusal_of_the_word_help_before_a_word_that_names_no_command_reports_no_flags() {
    let refused = walked(
        &["tool", "planets", "help", "nonsense"],
        &["tool", "planets", "show", "nonsense"],
    );

    assert_eq!(refused.parsed(), words(&["planets", "nonsense", "--help"]));
    assert_eq!(refused.flags(), Vec::<String>::new());
}

#[test]
fn a_refusal_of_the_word_help_reports_the_option_typed_after_it_and_its_value() {
    let refused = walked(
        &["tool", "planets", "help", "nonsense", "--format", "json"],
        &["tool", "planets", "show", "nonsense", "--format", "json"],
    );

    assert_eq!(refused.path(), ["planets"]);
    assert_eq!(refused.flags(), ["format"]);
    assert_eq!(refused.value("format"), Some(OsStr::new("json")));
}

#[test]
fn a_refusal_whose_caller_typed_the_help_flag_reports_clap_s_help_id() {
    let refused = walked(
        &["tool", "planets", "show", "Mars", "--srot", "--help"],
        &["tool", "planets", "show", "Mars", "--", "--help"],
    );

    assert_eq!(refused.flags(), ["help"]);
}
