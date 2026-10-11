use super::known;
use crate::usage::report::entry::Entry;
use crate::usage::testing::tool;

/// Returns a person's run of `command` that typed `flags`.
fn run(command: &str, flags: &[&str]) -> Entry {
    let mut entry = Entry::run("s", 1, command);
    entry.flags = flags.iter().map(|flag| (*flag).to_owned()).collect();
    entry
}

#[test]
fn a_record_of_the_tools_own_commands_and_flags_is_known() {
    let tool = tool();

    assert!(known(&tool, &run("", &["--full", "--help"])));
    assert!(known(&tool, &run("books", &["--help"])));
    assert!(known(&tool, &run("books list", &["--all", "--full", "-n"])));
    assert!(known(&tool, &run("books show", &[])));
}

#[test]
fn a_command_path_that_the_tool_lacks_or_spells_as_an_alias_is_not_known() {
    let tool = tool();

    assert!(!known(&tool, &run("books delete", &[])));
    assert!(!known(&tool, &run("books ls", &[])));
    assert!(!known(&tool, &run("books  list", &[])));
    assert!(!known(&tool, &run("sk_live_4eC39HqLyjWDarjtT1", &[])));
}

#[test]
fn a_flag_or_a_nearest_flag_that_the_command_lacks_is_not_known() {
    let tool = tool();
    let mut nearest = run("books show", &[]);
    nearest.nearest = Some("--password=hunter2".to_owned());

    assert!(!known(&tool, &run("books show", &["--all"])));
    assert!(!known(&tool, &run("books list", &["--number"])));
    assert!(!known(&tool, &run("books list", &["all"])));
    assert!(!known(&tool, &nearest));
}

#[test]
fn a_code_in_a_predefined_or_a_custom_spelling_is_known() {
    let tool = tool();
    let spelled = [
        "shelf::books::not_found",
        "shelf.books.not_found",
        "crumbkit/cli/usage",
        "shelf-books-not_found",
        "SHELF/BOOKS/NOT_FOUND",
        &"a".repeat(128),
    ];

    for code in spelled {
        let mut entry = run("books show", &[]);
        entry.code = Some(code.to_owned());
        assert!(known(&tool, &entry), "{code}");
    }
}

#[test]
fn a_code_with_a_space_another_character_or_more_than_128_characters_is_not_known() {
    let tool = tool();
    let codes = [
        "",
        "shelf books not_found",
        "shelf::books::not_found\n",
        "shelf::bücher::not_found",
        "token=ghp_16C7e42F292c6912E7710c838347Ae178B4a",
        &"a".repeat(129),
    ];

    for code in codes {
        let mut entry = run("books show", &[]);
        entry.code = Some(code.to_owned());
        assert!(!known(&tool, &entry), "{code}");
    }
}

#[test]
fn a_caller_is_known_only_when_the_log_writes_it() {
    let tool = tool();
    let caller = |name: &str| {
        let mut entry = run("books show", &[]);
        name.clone_into(&mut entry.caller);
        known(&tool, &entry)
    };

    assert!(caller("person"));
    assert!(caller("unknown"));
    assert!(caller("claude-code"));
    assert!(caller("codex"));
    assert!(!caller("Claude-Code"));
    assert!(!caller("ada@example.com"));
    assert!(!caller(""));
}
