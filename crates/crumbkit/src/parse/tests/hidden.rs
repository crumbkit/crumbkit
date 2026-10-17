//! Tests for a refusal whose suggestion clap took from a hidden alias,
//! which the refusal replaces with the real name.

use clap::{Arg, Command};

use super::{admits, parse_tree};

/// A tool whose `books` command has the hidden alias `book`, and whose
/// `books list` has a `--limit` option with the hidden alias `--max`.
fn tool_with_hidden_aliases() -> Command {
    Command::new("shelf").subcommand(Command::new("books").alias("book").subcommand(
        Command::new("list").arg(Arg::new("limit").long("limit").alias("max").value_name("N")),
    ))
}

#[test]
fn a_refusal_suggests_the_real_name_of_a_flag_whose_hidden_alias_clap_suggested() {
    let refused = ["shelf", "books", "list", "--maxx", "5"];
    admits(
        tool_with_hidden_aliases(),
        &refused,
        &["shelf", "books", "list", "--limit", "5"],
    );
    let parsed = parse_tree(tool_with_hidden_aliases(), &refused);

    let expected = "\
error: unexpected argument '--maxx' found; a similar argument exists: '--limit'
  help:  run `shelf books list -h`
  code:  shelf::cli::usage
";
    assert_eq!(parsed.err, expected);
}

#[test]
fn a_refusal_suggests_the_real_name_of_a_command_whose_hidden_alias_clap_suggested() {
    admits(
        tool_with_hidden_aliases(),
        &["shelf", "bok"],
        &["shelf", "books"],
    );
    let parsed = parse_tree(tool_with_hidden_aliases(), &["shelf", "bok"]);

    let expected = "\
error: unrecognized subcommand 'bok'; a similar subcommand exists: 'books'
  help:  run `shelf -h`
  code:  shelf::cli::usage
";
    assert_eq!(parsed.err, expected);
}
