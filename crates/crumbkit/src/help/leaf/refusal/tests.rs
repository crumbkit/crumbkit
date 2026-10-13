//! Tests for the context that a leaf's refusal alternatives read.

use std::ffi::OsString;

use clap::error::ErrorKind;
use clap::{Arg, Command};

use super::RefusalContext;

/// A command with a positional that takes a number and a flag.
fn get() -> Command {
    Command::new("get")
        .arg(
            Arg::new("id")
                .value_name("ID")
                .value_parser(clap::value_parser!(u32)),
        )
        .arg(Arg::new("sort").long("sort").value_name("KEY"))
}

/// Returns the words after the program name in `line`, and the error with
/// which `get()` refuses `line`.
fn refuse(line: &[&str]) -> (Vec<OsString>, clap::Error, Command) {
    let mut command = get();
    let Err(error) = command.try_get_matches_from_mut(line) else {
        panic!("{line:?} parsed");
    };
    let words = line.iter().skip(1).map(OsString::from).collect();
    (words, error, command)
}

#[test]
fn the_context_of_a_refused_value_names_the_arguments_id_the_value_the_kind_and_the_words() {
    let (words, error, command) = refuse(&["get", "a.json", "--sort", "name"]);

    let context = RefusalContext::new(&command, &error, &words);

    assert_eq!(context.argument(), Some("id"));
    assert_eq!(context.value(), Some("a.json"));
    assert_eq!(context.kind(), ErrorKind::ValueValidation);
    assert_eq!(context.words(), ["a.json", "--sort", "name"]);
}

#[test]
fn the_context_of_an_unknown_flag_names_no_argument_and_no_value() {
    let (words, error, command) = refuse(&["get", "7", "--srot", "name"]);

    let context = RefusalContext::new(&command, &error, &words);

    assert_eq!(context.argument(), None);
    assert_eq!(context.value(), None);
    assert_eq!(context.kind(), ErrorKind::UnknownArgument);
}
