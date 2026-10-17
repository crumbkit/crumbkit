//! Tests for reading the `<tool>/commands-list/v1` document.

use super::read;

/// A document of one command with every field.
const ONE: &str = r#"{"schema":"shelf/commands-list/v1","data":[
{"path":"shelf","about":"Browse","long_about":null,"arguments":[
{"name":"--json","short":null,"value_names":[],"required":false,"many":false,"defaults":[],"possible_values":[],"help":"Print JSON","long_help":null,"global":true,"one_of":[]}],
"examples":[],"columns":[],"form":null,"exits":[],"effects":null}],"total":1}"#;

#[test]
fn a_commands_list_document_gives_its_commands_in_order() {
    let commands = read(ONE.as_bytes()).unwrap();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].path, "shelf");
    assert_eq!(commands[0].arguments[0].name, "--json");
}

#[test]
fn a_document_with_another_schema_id_is_refused_with_that_id() {
    let other = ONE.replace("shelf/commands-list/v1", "shelf/books-list/v1");

    let error = read(other.as_bytes()).unwrap_err();

    assert_eq!(
        error.to_string(),
        "the input is a shelf/books-list/v1 document, and the reference needs the commands-list/v1 document that `commands list --all --json --full` prints"
    );
}

#[test]
fn input_that_is_not_json_of_the_document_s_shape_is_refused_with_serde_s_error_as_its_source() {
    for input in [
        "PATH  ABOUT\n",
        r#"{"schema":"crumbkit/commands-list/v1","data":[{}],"total":1}"#,
    ] {
        let error = read(input.as_bytes()).unwrap_err();

        assert_eq!(
            error.to_string(),
            "the input is not the document that `commands list --all --json --full` prints"
        );
        assert!(std::error::Error::source(&error).is_some());
    }
}

#[test]
fn one_page_of_a_longer_listing_is_refused_so_no_command_is_left_out() {
    let page = ONE.replace(r#""total":1"#, r#""total":3"#);

    let error = read(page.as_bytes()).unwrap_err();

    assert_eq!(
        error.to_string(),
        "the document holds 1 of 3 commands, so the reference would leave some out; list them with `commands list --all`"
    );
}

#[test]
fn a_page_without_a_total_is_refused_so_no_command_is_left_out() {
    let page = ONE.replace(r#""total":1"#, r#""truncated":true"#);

    let error = read(page.as_bytes()).unwrap_err();

    assert_eq!(
        error.to_string(),
        "the document leaves out commands after its first 1, so the reference would leave some out; list them with `commands list --all`"
    );
}

#[test]
fn a_document_marked_truncated_is_refused_even_when_it_holds_its_total() {
    let cut = ONE.replace(r#""total":1"#, r#""total":1,"truncated":true"#);

    assert!(read(cut.as_bytes()).is_err());
}

#[test]
fn a_document_without_commands_is_refused() {
    let empty = r#"{"schema":"crumbkit/commands-list/v1","data":[],"total":0}"#;

    assert_eq!(
        read(empty.as_bytes()).unwrap_err().to_string(),
        "the document holds no command"
    );
}
