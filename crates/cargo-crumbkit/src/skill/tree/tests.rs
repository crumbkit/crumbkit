//! Tests for reading a tool's command tree.

use super::Tree;

/// A `shelf/commands-list/v1` document of three commands, with fields that
/// the tree does not read.
const DOCUMENT: &str = r#"{"schema":"shelf/commands-list/v1","data":[
{"path":"shelf","about":"Browse","arguments":[
{"name":"--json","short":null,"value_names":[],"required":false,"many":false,"defaults":[],"possible_values":[],"global":true}]},
{"path":"shelf books","about":"Books","arguments":[]},
{"path":"shelf books show","about":"Show","arguments":[
{"name":"<ID>","short":null,"value_names":["ID"],"required":true,"many":true,"defaults":[],"possible_values":[],"global":false},
{"name":"--from","short":"-f","value_names":["FILE"],"required":false,"many":false,"defaults":[],"possible_values":[],"global":false}]}
],"total":3}"#;

#[test]
fn a_commands_document_gives_the_root_s_name_each_command_and_its_arguments() {
    let tree = Tree::read(DOCUMENT.as_bytes()).unwrap();

    assert_eq!(tree.name(), "shelf");
    let show = tree.command("shelf books show").unwrap();
    let names: Vec<&str> = show.arguments.iter().map(|arg| arg.name.as_str()).collect();
    assert_eq!(names, ["<ID>", "--from"]);
    assert!(show.arguments[1].is_called("-f"));
    assert!(show.arguments[1].is_called("--from"));
    assert!(!show.arguments[0].is_called("<ID>"));
    assert_eq!(show.positionals().count(), 1);
    assert!(tree.command("shelf books list").is_none());
}

#[test]
fn a_command_has_subcommands_when_a_row_sits_one_word_below_it() {
    let tree = Tree::read(DOCUMENT.as_bytes()).unwrap();

    assert!(tree.has_subcommands("shelf"));
    assert!(tree.has_subcommands("shelf books"));
    assert!(!tree.has_subcommands("shelf books show"));
    assert!(!tree.has_subcommands("shelf book"));
}

#[test]
fn output_that_is_not_a_commands_document_or_lists_no_command_gives_none() {
    let other = DOCUMENT.replace("shelf/commands-list/v1", "shelf/books-list/v1");

    assert_eq!(Tree::read(other.as_bytes()), None);
    assert_eq!(Tree::read(b"PATH  ABOUT\n"), None);
    assert_eq!(
        Tree::read(br#"{"schema":"shelf/commands-list/v1","data":[]}"#),
        None
    );
}

#[test]
fn a_command_keeps_its_about_text_and_goal_headings() {
    let document = DOCUMENT.replace(
        r#""about":"Browse","#,
        r#""about":"Browse","goals":[{"heading":"Browse the catalogue","commands":["books"]}],"#,
    );
    let tree = Tree::read(document.as_bytes()).unwrap();

    assert_eq!(tree.root().about.as_deref(), Some("Browse"));
    assert_eq!(tree.root().goals[0].heading, "Browse the catalogue");
    assert_eq!(tree.root().goals[0].commands, ["books"]);
    assert_eq!(tree.command("shelf books").unwrap().goals, []);
    let paths: Vec<&str> = tree
        .commands()
        .map(|command| command.path.as_str())
        .collect();
    assert_eq!(paths, ["shelf", "shelf books", "shelf books show"]);
}

#[test]
fn a_flag_resolves_on_its_command_or_as_a_global_of_a_command_above_it() {
    let tree = Tree::read(DOCUMENT.as_bytes()).unwrap();

    let from = tree.flag("shelf books show", "-f").unwrap();
    assert_eq!(from.name, "--from");
    assert_eq!(
        tree.flag("shelf books show", "--json").unwrap().name,
        "--json"
    );
    assert!(tree.flag("shelf books show", "--all").is_none());
    assert!(tree.flag("shelf books", "--from").is_none());
}
