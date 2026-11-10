//! Tests for checking one command line against a command tree.

use super::verify;
use crate::skill::tree::{Argument, Command, Tree};

/// Returns a flag called `name` that takes the values `value_names`.
fn flag(name: &str, value_names: &[&str]) -> Argument {
    Argument {
        name: name.to_owned(),
        short: None,
        value_names: value_names.iter().map(|&value| value.to_owned()).collect(),
        many: false,
        possible_values: Vec::new(),
        global: false,
    }
}

/// Returns a positional argument whose value is called `value`.
fn positional(value: &str, many: bool) -> Argument {
    Argument {
        many,
        value_names: vec![value.to_owned()],
        ..flag(&format!("<{value}>"), &[])
    }
}

/// Returns the command at `path` with `arguments`.
fn command(path: &str, arguments: Vec<Argument>) -> Command {
    Command {
        path: path.to_owned(),
        arguments,
    }
}

/// A tree like shelf's, with a global flag at the root and below `books`, a
/// flag with possible values and one with a short name.
fn tree() -> Tree {
    let json = Argument {
        global: true,
        ..flag("--json", &[])
    };
    let fields = Argument {
        global: true,
        many: true,
        ..flag("--fields", &["FIELD"])
    };
    let quiet = Argument {
        global: true,
        short: Some("-q".to_owned()),
        ..flag("--quiet", &[])
    };
    let format = Argument {
        possible_values: vec!["text".to_owned(), "markdown".to_owned()],
        ..flag("--format", &["FORMAT"])
    };
    let limit = Argument {
        short: Some("-n".to_owned()),
        ..flag("--limit", &["N"])
    };
    Tree::new(vec![
        command("shelf", vec![json, fields]),
        command("shelf books", vec![quiet]),
        command("shelf books list", vec![limit, flag("--all", &[]), format]),
        command(
            "shelf books show",
            vec![positional("ID", true), flag("--from", &["FILE"])],
        ),
        command(
            "shelf books move",
            vec![positional("ID", false), positional("SHELF", false)],
        ),
    ])
}

/// Returns the reason that `line` fails against [`tree`], or `None` when it
/// passes.
fn reason(line: &str) -> Option<String> {
    let words: Vec<String> = line.split(' ').map(ToOwned::to_owned).collect();
    verify(&tree(), &words).err()
}

#[test]
fn a_real_command_with_its_own_and_inherited_flags_passes() {
    for line in [
        "shelf",
        "shelf books",
        "shelf books list",
        "shelf books list --limit 5 --all",
        "shelf books list --limit=5",
        "shelf books list -n 5",
        "shelf books list -n5",
        "shelf books list -qn 5",
        "shelf books list --json --fields title,author -q",
        "shelf --json books list",
        "shelf books show 1 2 3 --from book.json",
        "shelf books move 1 attic",
        "shelf books list -h",
        "shelf books --help",
        "shelf help books list",
    ] {
        assert_eq!(reason(line), None, "{line}");
    }
}

#[test]
fn a_word_that_is_no_subcommand_fails() {
    assert_eq!(
        reason("shelf book list").as_deref(),
        Some("`book` is not a command of `shelf`")
    );
    assert_eq!(
        reason("shelf books lists").as_deref(),
        Some("`lists` is not a command of `shelf books`")
    );
}

#[test]
fn a_flag_that_the_command_does_not_declare_fails() {
    assert_eq!(
        reason("shelf books list --author Austen").as_deref(),
        Some("`shelf books list` has no flag `--author`")
    );
    assert_eq!(
        reason("shelf books list -x").as_deref(),
        Some("`shelf books list` has no flag `-x`")
    );
    // A flag of a lower command is not a flag of the command above it.
    assert_eq!(
        reason("shelf books --all list").as_deref(),
        Some("`shelf books` has no flag `--all`")
    );
    // A global flag applies below the command that defines it, not above.
    assert_eq!(
        reason("shelf -q books").as_deref(),
        Some("`shelf` has no flag `-q`")
    );
}

#[test]
fn a_flag_that_takes_a_value_and_has_none_fails() {
    assert_eq!(
        reason("shelf books list --limit").as_deref(),
        Some("`--limit` needs a value")
    );
    assert_eq!(
        reason("shelf books list --limit --all").as_deref(),
        Some("`--limit` needs a value")
    );
    assert_eq!(
        reason("shelf books list -n").as_deref(),
        Some("`-n` needs a value")
    );
    assert_eq!(
        reason("shelf books list --all=yes").as_deref(),
        Some("`--all` takes no value")
    );
}

#[test]
fn a_value_outside_the_possible_values_fails_and_a_placeholder_passes() {
    assert_eq!(reason("shelf books list --format markdown"), None);
    assert_eq!(reason("shelf books list --format FORMAT"), None);
    assert_eq!(reason("shelf books list --format <format>"), None);
    assert_eq!(
        reason("shelf books list --format html").as_deref(),
        Some("`--format` takes text or markdown, and `html` is neither")
    );
}

#[test]
fn an_extra_argument_fails() {
    assert_eq!(
        reason("shelf books list 5").as_deref(),
        Some("`shelf books list` takes no argument `5`")
    );
    assert_eq!(
        reason("shelf books move 1 attic cellar").as_deref(),
        Some("`shelf books move` takes no argument `cellar`")
    );
}

#[test]
fn placeholders_count_as_values_and_help_notation_is_skipped() {
    for line in [
        "shelf books show ID",
        "shelf books show <id> --from FILE",
        "shelf books show ID...",
        "shelf books show <ID>... --from <file>",
        "shelf books list [OPTIONS]",
        "shelf books list [--limit N]",
        "shelf books list [--all]",
        "shelf books move ID [SHELF]",
        "shelf books move ID ...",
        "shelf COMMAND",
        "shelf books <command> -h",
    ] {
        assert_eq!(reason(line), None, "{line}");
    }
    assert_eq!(
        reason("shelf books list [--author NAME]").as_deref(),
        Some("`shelf books list` has no flag `--author`")
    );
}

#[test]
fn a_double_dash_ends_the_check() {
    assert_eq!(reason("shelf books show 1 -- --anything goes"), None);
}
