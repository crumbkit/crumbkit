//! Tests for writing the reference.

use serde_json::{Value, json};

use super::render;

/// Returns `base` with each field of `fields` set over it.
fn with(mut base: Value, fields: Value) -> Value {
    if let (Some(base), Value::Object(fields)) = (base.as_object_mut(), fields) {
        base.extend(fields);
    }
    base
}

/// Returns an argument named `name` with the empty defaults and `fields`.
fn argument(name: &str, fields: Value) -> Value {
    let base = json!({
        "name": name, "short": null, "value_names": [], "required": false,
        "many": false, "defaults": [], "possible_values": [], "help": null,
        "long_help": null, "global": false, "one_of": [],
    });
    with(base, fields)
}

/// Returns a command row with `path`, the `about` text, the empty defaults
/// and `fields`.
fn command(path: &str, about: &str, fields: Value) -> Value {
    let base = json!({
        "path": path, "about": about, "long_about": null, "arguments": [],
        "examples": [], "columns": [], "form": null, "exits": [], "effects": null,
    });
    with(base, fields)
}

/// Returns the bytes of the document of `rows`, which holds every row.
fn document(rows: &[Value]) -> Vec<u8> {
    let total = rows.len();
    let document = json!({"schema": "crumbkit/commands-list/v1", "data": rows, "total": total});
    serde_json::to_vec(&document).unwrap()
}

/// A tool with a root, a group and a leaf that uses every part of a section.
fn catalogue() -> Vec<u8> {
    let json = argument("--json", json!({"help": "Print JSON", "global": true}));
    let limit = argument(
        "--limit",
        json!({"short": "-n", "value_names": ["N"], "defaults": ["4"], "help": "Print at most this many rows"}),
    );
    let id = argument(
        "<ID>",
        json!({"value_names": ["ID"], "required": true, "many": true, "help": "The ids", "one_of": ["--from"]}),
    );
    let sort = argument(
        "--sort",
        json!({
            "value_names": ["KEY"], "possible_values": ["id", "title"], "help": "The key",
            "long_help": "The sort key\n\nSorting is stable.",
        }),
    );
    document(&[
        command("tool", "Browse a catalogue", json!({"arguments": [json]})),
        command("tool books", "Work with books", json!({})),
        command(
            "tool books list",
            "List books",
            json!({
                "long_about": "List books.\n\nThe list is in id order.",
                "arguments": [limit, id, sort],
                "examples": [
                    {"command": "tool books list", "about": "Print the first page"},
                    {"command": "tool books list --all", "about": ""},
                ],
                "columns": [
                    {"name": "id", "long": false, "description": "The id."},
                    {"name": "title", "long": false, "description": null},
                    {"name": "form", "long": true, "description": "The form."},
                ],
                "form": "stream",
                "exits": [{"exit": 3, "meaning": "The catalogue is locked"}],
                "effects": {"read_only": true, "destructive": false, "idempotent": true, "open_world": false},
            }),
        ),
    ])
}

/// The reference of [`catalogue`].
const CATALOGUE: &str = "# Commands of `tool`

This reference is generated from the help of `tool`. Edit the help in the tool's code, and not this file.

## `tool`

Browse a catalogue

### Commands

- [`tool books`](#tool-books): Work with books

### Arguments

- `--json`: Print JSON. Works after every subcommand.

## `tool books`

Work with books

### Commands

- [`tool books list`](#tool-books-list): List books

## `tool books list`

List books.

The list is in id order.

### Arguments

- `-n`, `--limit <N>`: Print at most this many rows. Default: `4`.
- `<ID>`: The ids. Required, or give `--from` in its place. Takes more than one value.
- `--sort <KEY>`: The sort key

  Sorting is stable. One of `id`, `title`.

### Examples

Print the first page.

```sh
tool books list
```

```sh
tool books list --all
```

### Columns

- `id`: The id.
- `title`

### Columns that `--long` adds

- `form`: The form.

Effects: read-only, closed-world.

Output: one row per line as each arrives, and --json prints one JSON document per line.

### Exit codes

- `3`: The catalogue is locked.
";

#[test]
fn each_command_has_a_section_with_its_description_commands_arguments_examples_columns_effects_and_exit_codes()
 {
    let reference = render(&catalogue()).unwrap();

    assert_eq!(reference.text(), CATALOGUE);
    assert_eq!(reference.commands(), 3);
}

#[test]
fn the_effects_line_words_the_effects_as_help_does() {
    let input = document(&[command(
        "tool",
        "Remove",
        json!({"effects": {"read_only": false, "destructive": true, "idempotent": true, "open_world": true}}),
    )]);

    let reference = render(&input).unwrap();

    assert!(
        reference
            .text()
            .contains("\n\nEffects: destructive, idempotent, open-world.\n"),
        "{}",
        reference.text()
    );
}

#[test]
fn a_headed_command_lists_its_json_fields_and_prints_the_headed_output_line_as_help_does() {
    let input = document(&[command(
        "tool show",
        "Show",
        json!({
            "form": "headed",
            "columns": [
                {"name": "name", "long": false, "description": "The name."},
                {"name": "text", "long": false, "description": "The bytes as text."},
            ],
            "effects": {"read_only": true, "destructive": false, "idempotent": true, "open_world": false},
        }),
    )]);

    let reference = render(&input).unwrap();

    let expected = format!(
        "\n### JSON fields that `--fields` selects\n\n- `name`: The name.\n- `text`: The bytes as text.\n\nEffects: read-only, closed-world.\n\n{}\n",
        crumbkit::help::HEADED_LINE
    );
    assert!(
        reference.text().ends_with(&expected),
        "{}",
        reference.text()
    );
    assert!(
        !reference.text().contains("### Columns"),
        "{}",
        reference.text()
    );
}

#[test]
fn a_bytes_command_prints_the_bytes_output_line_as_help_does() {
    let input = document(&[command(
        "tool export",
        "Export",
        json!({
            "form": "bytes",
            "effects": {"read_only": true, "destructive": false, "idempotent": true, "open_world": false},
        }),
    )]);

    let reference = render(&input).unwrap();

    let expected = format!(
        "\nEffects: read-only, closed-world.\n\n{}\n",
        crumbkit::help::BYTES_LINE
    );
    assert!(
        reference.text().ends_with(&expected),
        "{}",
        reference.text()
    );
}

#[test]
fn the_writes_line_follows_the_effects_line_and_words_the_guards_as_help_does() {
    let input = document(&[command(
        "tool",
        "Edit",
        json!({
            "effects": {"read_only": false, "destructive": true, "idempotent": false, "open_world": false},
            "writes": {"replaces": true, "expect": true, "items": "all_or_none"},
        }),
    )]);

    let reference = render(&input).unwrap();

    assert!(
        reference.text().contains(
            "\n\nEffects: destructive, not idempotent, closed-world.\n\nWrites: replaces existing data; --expect <VERSION> fails with a conflict when the data changed; applies all items or none.\n"
        ),
        "{}",
        reference.text()
    );
}

#[test]
fn a_command_without_writes_or_with_null_writes_has_no_writes_line() {
    let effects =
        json!({"read_only": false, "destructive": true, "idempotent": true, "open_world": false});
    let input = document(&[
        command("tool one", "Remove", json!({"effects": effects})),
        command(
            "tool two",
            "Remove",
            json!({"effects": effects, "writes": null}),
        ),
    ]);

    let reference = render(&input).unwrap();

    assert!(
        !reference.text().contains("Writes:"),
        "{}",
        reference.text()
    );
}

#[test]
fn an_argument_with_several_alternatives_and_defaults_names_them_all() {
    let from = argument(
        "--from",
        json!({"value_names": ["FILE"], "defaults": ["a", "b"], "one_of": ["<ID>", "--all"]}),
    );
    let input = document(&[command("tool", "Read", json!({"arguments": [from]}))]);

    let reference = render(&input).unwrap();

    assert!(
        reference.text().contains(
            "- `--from <FILE>`: Required, or give one of `<ID>` and `--all` in its place. Defaults: `a`, `b`.\n"
        ),
        "{}",
        reference.text()
    );
}

#[test]
fn help_text_with_markup_characters_prints_as_written() {
    let input = document(&[command("tool", "Print <N> *rows*", json!({}))]);

    let reference = render(&input).unwrap();

    assert!(
        reference.text().contains("\n\nPrint \\<N\\> \\*rows\\*\n"),
        "{}",
        reference.text()
    );
}

#[test]
fn two_commands_with_one_anchor_are_refused_naming_both() {
    let input = document(&[
        command("tool", "Root", json!({})),
        command("tool books-list", "One", json!({})),
        command("tool books list", "Two", json!({})),
    ]);

    let error = render(&input).unwrap_err();

    assert_eq!(
        error.to_string(),
        "`tool books-list` and `tool books list` both have the anchor #tool-books-list, so a link to the second would reach the first"
    );
}

#[test]
fn a_command_whose_anchor_is_the_title_s_is_refused() {
    let input = document(&[
        command("commands", "Root", json!({})),
        command("commands of", "Of", json!({})),
        command("commands of commands", "Again", json!({})),
    ]);

    let error = render(&input).unwrap_err();

    assert_eq!(
        error.to_string(),
        "`Commands of commands` and `commands of commands` both have the anchor #commands-of-commands, so a link to the second would reach the first"
    );
}

#[test]
fn a_command_whose_rows_print_their_own_line_prints_the_text_line_as_help_does() {
    let input = document(&[command(
        "tool links check",
        "Check",
        json!({
            "form": "document",
            "line": "<POINTER> -> <TARGET>",
            "columns": [
                {"name": "pointer", "long": false, "description": "The pointer."},
                {"name": "status", "long": true, "description": "The status."},
            ],
        }),
    )]);

    let reference = render(&input).unwrap();

    let expected = format!(
        "\n{}\n",
        crumbkit::help::text_line("`<POINTER> -> <TARGET>`", true, true)
    );
    assert!(
        reference.text().ends_with(&expected),
        "{}",
        reference.text()
    );
}

#[test]
fn a_document_from_before_the_line_field_existed_prints_no_text_line() {
    let input = document(&[command(
        "tool links check",
        "Check",
        json!({"form": "document"}),
    )]);

    let reference = render(&input).unwrap();

    assert!(!reference.text().contains("Text:"), "{}", reference.text());
}

#[test]
fn a_command_whose_lined_rows_have_no_columns_prints_a_text_line_that_names_no_table() {
    let input = document(&[command(
        "tool settings list",
        "List",
        json!({"form": "document", "line": "<KEY> = <VALUE>", "columns": []}),
    )]);

    let reference = render(&input).unwrap();

    assert!(
        reference
            .text()
            .ends_with("\nText: one line per row, `<KEY> = <VALUE>`.\n"),
        "{}",
        reference.text()
    );
}
