//! Tests for building calls and reading their targets. crumbkit's tests
//! check how a call renders against a command tree.

use super::{Call, CallArg, CallTarget};

#[test]
fn a_call_names_its_path_then_its_placeholders_and_values_in_the_order_they_were_added() {
    let call = Call::new(&["books", "list"])
        .arg("author")
        .value("limit", "2")
        .value("title", "Le Guin".to_owned());

    assert_eq!(
        call.target(),
        &CallTarget::Command {
            path: &["books", "list"],
            args: vec![
                CallArg::Placeholder("author"),
                CallArg::Value {
                    id: "limit",
                    value: "2".to_owned(),
                },
                CallArg::Value {
                    id: "title",
                    value: "Le Guin".to_owned(),
                },
            ],
        }
    );
}

#[test]
fn a_help_call_names_the_path_the_caller_typed() {
    let call = Call::help(vec!["books".to_owned(), "shw".to_owned()]);

    assert_eq!(
        call.target(),
        &CallTarget::Help(vec!["books".to_owned(), "shw".to_owned()])
    );
}

#[test]
fn a_help_call_keeps_no_arguments_or_values() {
    let call = Call::help(vec!["books".to_owned()])
        .arg("ids")
        .value("ids", "7");

    assert_eq!(call.target(), &CallTarget::Help(vec!["books".to_owned()]));
}

#[test]
fn a_typed_call_holds_the_words_and_keeps_no_arguments_or_values() {
    let words = vec!["books".to_owned(), "remove".to_owned(), "2".to_owned()];
    let call = Call::typed(words.clone()).arg("ids").value("ids", "7");

    assert_eq!(call.target(), &CallTarget::Typed(words));
}

#[test]
fn an_all_argument_names_the_field_whose_values_it_takes_together() {
    let call = Call::new(&["books", "show"]).all("ids", "left_out");

    assert_eq!(
        call.target(),
        &CallTarget::Command {
            path: &["books", "show"],
            args: vec![CallArg::All {
                id: "ids",
                field: "left_out",
            }],
        }
    );
}

#[test]
fn a_typed_call_that_takes_every_value_of_a_field_keeps_its_words_and_the_place_of_the_values() {
    let words = vec!["books".to_owned(), "show".to_owned(), "--json".to_owned()];
    let call = Call::typed_all(words.clone(), 2, "<ID>...".to_owned(), "left_out");

    assert_eq!(call.target(), &CallTarget::Typed(words.clone()));
    assert_eq!(call.typed_slot(), Some((2, "<ID>...", "left_out")));
    assert_eq!(Call::typed(words).typed_slot(), None);
}
