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
