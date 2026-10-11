//! Tests for building calls and reading their targets. crumbkit's tests
//! check how a call renders against a command tree.

use super::{Call, CallTarget};

#[test]
fn a_call_names_its_path_then_its_arguments_and_words_in_the_order_they_were_added() {
    let call = Call::new(&["books", "list"])
        .arg("author")
        .word("books".to_owned())
        .arg("limit")
        .word("two words".to_owned());

    assert_eq!(
        call.target(),
        &CallTarget::Command {
            path: &["books", "list"],
            args: vec!["author", "limit"],
            words: vec!["books".to_owned(), "two words".to_owned()],
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
fn a_help_call_keeps_no_arguments_or_words() {
    let call = Call::help(vec!["books".to_owned()])
        .arg("ids")
        .word("7".to_owned());

    assert_eq!(call.target(), &CallTarget::Help(vec!["books".to_owned()]));
}
