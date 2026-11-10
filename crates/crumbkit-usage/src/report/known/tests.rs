use super::known;
use crate::report::entry::Entry;
use crate::testing::tree;

/// Returns a person's run of `command` that typed `flags`.
fn run(command: &str, flags: &[&str]) -> Entry {
    let mut entry = Entry::run("s", 1, command);
    entry.flags = flags.iter().map(|flag| (*flag).to_owned()).collect();
    entry
}

#[test]
fn a_record_of_the_tools_own_commands_and_flags_is_known() {
    let tool = tree();

    assert!(known(&tool, &run("", &["--full", "--help"])));
    assert!(known(&tool, &run("books", &["--help"])));
    assert!(known(&tool, &run("books list", &["--all", "--full", "-n"])));
    assert!(known(&tool, &run("books show", &[])));
}

#[test]
fn a_command_path_that_the_tool_lacks_or_spells_as_an_alias_is_not_known() {
    let tool = tree();

    assert!(!known(&tool, &run("books delete", &[])));
    assert!(!known(&tool, &run("books ls", &[])));
    assert!(!known(&tool, &run("books  list", &[])));
    assert!(!known(&tool, &run("sk_live_4eC39HqLyjWDarjtT1", &[])));
}

#[test]
fn a_flag_or_a_nearest_flag_that_the_command_lacks_is_not_known() {
    let tool = tree();
    let mut nearest = run("books show", &[]);
    nearest.nearest = Some("--password=hunter2".to_owned());

    assert!(!known(&tool, &run("books show", &["--all"])));
    assert!(!known(&tool, &run("books list", &["--number"])));
    assert!(!known(&tool, &run("books list", &["all"])));
    assert!(!known(&tool, &nearest));
}

#[test]
fn a_code_in_crumbkit_s_spelling_is_known() {
    let tool = tree();
    let spelled = [
        "shelf::books::not_found",
        "crumbkit::cli::usage",
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
    let tool = tree();
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
fn a_tolerated_code_is_checked_as_the_failure_code_is() {
    let tool = tree();
    let tolerated = |codes: &[&str]| {
        let mut entry = run("books show", &[]);
        entry.tolerated = codes.iter().map(|code| (*code).to_owned()).collect();
        known(&tool, &entry)
    };

    assert!(tolerated(&[
        "shelf::catalogue::locked",
        "shelf::network::timed_out"
    ]));
    assert!(!tolerated(&[
        "shelf::catalogue::locked",
        "token=ghp_16C7e42F"
    ]));
    assert!(!tolerated(&[""]));
}

#[test]
fn a_caller_is_known_only_when_the_log_writes_it() {
    let tool = tree();
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

#[test]
fn a_record_whose_trace_ids_are_lowercase_hex_of_the_right_length_is_known() {
    let tool = tree();
    let mut child = run("books show", &[]);
    child.parent = Some("b7ad6b7169203331".to_owned());

    assert!(known(&tool, &run("books show", &[])));
    assert!(known(&tool, &child));
}

#[test]
fn a_record_whose_trace_run_or_parent_id_is_not_lowercase_hex_of_the_right_length_is_not_known() {
    let tool = tree();
    let mut trace = run("books show", &[]);
    trace.trace = "4BF92F3577B34DA6A3CE929D0E0E4736".to_owned();
    let mut short = run("books show", &[]);
    short.trace = "4bf92f3577b34da6".to_owned();
    let mut span = run("books show", &[]);
    span.run = "sk_live_4eC39HqL".to_owned();
    let mut parent = run("books show", &[]);
    parent.parent = Some("00f067aa0ba902b".to_owned());

    for entry in [trace, short, span, parent] {
        assert!(!known(&tool, &entry), "{entry:?}");
    }
}
