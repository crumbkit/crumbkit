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
            name,
            command: ListCommand::List(args),
        } => {
            assert_eq!(name, "to-read");
            assert_eq!(args.page.limit(), Some(2));
        }
        other => panic!("the command parsed as another command: {other:?}"),
    }
}

#[test]
fn the_output_of_a_command_added_for_a_list_reads_the_view_flags_after_it() {
    let lists = [List::new(
        "to-read".to_owned(),
        "Books to read next".to_owned(),
    )];
    let (out, err) = (&mut Vec::new(), &mut Vec::new());
    let parsed = crumbkit::parse_to::<Parsed, _, _, _, _>(
        &mut crumbkit::Sink::new(out, err),
        crate::command(&lists),
        &crate::guide(&lists),
        ["reading", "to-read", "list", "--json"],
    );

    assert!(parsed.is_ok_and(|(_, output)| output.is_json()));
}

#[test]
fn a_command_of_the_static_grammar_parses_as_the_static_grammar() {
    match parse(&["reading", "entries", "list", "--list", "to-read"]) {
        Parsed::Static(_) => {}
        Parsed::Grafted { name, .. } => panic!("the command parsed as the list {name}"),
    }
}
