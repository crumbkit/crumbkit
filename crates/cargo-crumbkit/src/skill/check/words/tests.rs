//! Tests for splitting a command line into words.

use super::split;

/// Returns the words of `line`, or panics with the reason it cannot be split.
fn words(line: &str) -> Vec<String> {
    split(line).unwrap()
}

#[test]
fn spaces_and_tabs_separate_words() {
    assert_eq!(
        words("shelf  books\tlist --limit 5"),
        ["shelf", "books", "list", "--limit", "5"]
    );
}

#[test]
fn quotes_and_backslashes_join_words_as_a_shell_does() {
    assert_eq!(
        words(r#"shelf books add 'Jane Austen' "Emma \"E\"" a\ b"#),
        ["shelf", "books", "add", "Jane Austen", r#"Emma "E""#, "a b"]
    );
    assert_eq!(words("shelf books add ''"), ["shelf", "books", "add", ""]);
}

#[test]
fn a_pipe_a_list_operator_or_a_comment_ends_the_command() {
    assert_eq!(words("shelf books list | head"), ["shelf", "books", "list"]);
    assert_eq!(words("shelf books list|head"), ["shelf", "books", "list"]);
    assert_eq!(
        words("shelf books list && shelf"),
        ["shelf", "books", "list"]
    );
    assert_eq!(words("shelf books list; ls"), ["shelf", "books", "list"]);
    assert_eq!(
        words("shelf books list # first page"),
        ["shelf", "books", "list"]
    );
    assert_eq!(
        words("shelf books show a#b"),
        ["shelf", "books", "show", "a#b"]
    );
}

#[test]
fn a_redirection_ends_the_command_with_its_file_descriptor() {
    assert_eq!(
        words("shelf books list > out.txt"),
        ["shelf", "books", "list"]
    );
    assert_eq!(
        words("shelf books list 2>/dev/null"),
        ["shelf", "books", "list"]
    );
    assert_eq!(
        words("shelf books list < in.txt"),
        ["shelf", "books", "list"]
    );
}

#[test]
fn an_angle_bracket_placeholder_stays_one_word() {
    assert_eq!(
        words("shelf books show <id> --from <file name>"),
        ["shelf", "books", "show", "<id>", "--from", "<file name>"]
    );
}

#[test]
fn an_unclosed_quote_gives_a_reason() {
    assert_eq!(
        split("shelf books add 'Jane").unwrap_err(),
        "the command line has an unclosed quote"
    );
}
