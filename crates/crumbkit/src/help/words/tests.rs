use super::quote;

#[test]
fn a_word_that_a_posix_shell_would_split_or_expand_is_single_quoted() {
    assert_eq!(quote("--fields=title,year"), "--fields=title,year");
    assert_eq!(quote("./shelf.toml"), "./shelf.toml");
    assert_eq!(quote("Jane Austen"), "'Jane Austen'");
    assert_eq!(quote("$HOME"), "'$HOME'");
    assert_eq!(quote("*.txt"), "'*.txt'");
    assert_eq!(quote("~"), "'~'");
    assert_eq!(quote(""), "''");
    assert_eq!(quote("Ceres's"), r"'Ceres'\''s'");
}
