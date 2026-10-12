use clap::FromArgMatches as _;

use super::{ListCommand, Parsed};
use crate::lists::List;

/// Parses `args` against the tree that reading builds for one list named
/// `to-read`.
fn parse(args: &[&str]) -> Parsed {
    let lists = [List::new(
        "to-read".to_owned(),
        "Books to read next".to_owned(),
    )];
    let matches = crate::command(&lists)
        .try_get_matches_from(args)
        .expect("the arguments parse");
    Parsed::from_arg_matches(&matches).expect("the matches convert")
}

#[test]
fn a_command_added_for_a_list_parses_as_the_list_name_and_its_command() {
    match parse(&["reading", "to-read", "list", "--limit", "2", "--json"]) {
        Parsed::Grafted {
            output,
            name,
            command: ListCommand::List(args),
        } => {
            assert_eq!(name, "to-read");
            assert_eq!(args.page.limit(), Some(2));
            assert!(output.is_json());
        }
        Parsed::Static(cli) => panic!("the command parsed as the static grammar: {cli:?}"),
    }
}

#[test]
fn a_command_of_the_static_grammar_parses_as_the_static_grammar() {
    match parse(&["reading", "entries", "list", "--list", "to-read"]) {
        Parsed::Static(_) => {}
        Parsed::Grafted { name, .. } => panic!("the command parsed as the list {name}"),
    }
}
