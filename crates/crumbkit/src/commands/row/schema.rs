//! The JSON Schema of `CommandRow`, and of one argument, one exit code and
//! one goal heading of a command, which it lists under `arguments`, `exits`
//! and `goals`.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};

use super::CommandRow;
use crate::help::effects;

/// Returns the schema of one argument, which `CommandRow`'s schema lists
/// under `arguments`.
fn argument_schema() -> Schema {
    json_schema!({
        "type": "object",
        "properties": {
            "name": {
                "description": "The name as help writes it: `--all` for an argument with a long name, `-a` for one with only a short name, and `<ID>` for a positional argument.",
                "type": "string",
            },
            "short": {
                "description": "The short form, such as `-n`, when the argument has one.",
                "type": ["string", "null"],
            },
            "value_names": {
                "description": "The names of the values that the argument takes, such as `N`, and none for a flag.",
                "type": "array",
                "items": { "type": "string" },
            },
            "required": {
                "description": "Whether the command refuses a command line without the argument.",
                "type": "boolean",
            },
            "many": {
                "description": "Whether the argument takes more than one value, through repeats, commas or several words.",
                "type": "boolean",
            },
            "defaults": {
                "description": "The values that the argument takes when the caller leaves it out, as help shows them.",
                "type": "array",
                "items": { "type": "string" },
            },
            "possible_values": {
                "description": "The values that the argument accepts, as help shows them, and none when it accepts any value.",
                "type": "array",
                "items": { "type": "string" },
            },
            "help": {
                "description": "The one-line help: the argument's `help`, or its `long_help` when it has no `help`.",
                "type": ["string", "null"],
            },
            "long_help": {
                "description": "The full help that `--help` shows, when the argument has one.",
                "type": ["string", "null"],
            },
            "global": {
                "description": "Whether the argument also works after every subcommand below the command that defines it.",
                "type": "boolean",
            },
            "one_of": {
                "description": "The visible arguments that can be given in place of this one. These are the other arguments of each required group that holds it and, when the argument is required, the arguments it conflicts with. The list is empty when no argument can be given in its place.",
                "type": "array",
                "items": { "type": "string" },
            },
        },
        "required": [
            "name",
            "short",
            "value_names",
            "required",
            "many",
            "defaults",
            "possible_values",
            "help",
            "long_help",
            "global",
            "one_of",
        ],
    })
}

/// Returns the schema of one exit code, which `CommandRow`'s schema lists
/// under `exits`.
fn exit_schema() -> Schema {
    json_schema!({
        "type": "object",
        "properties": {
            "exit": {
                "description": "The exit code, or the first code of a range of codes that the command passes on from another program.",
                "type": "integer",
                "minimum": 0,
                "maximum": 255,
            },
            "last": {
                "description": "The last code of the range, which equals exit for a single code.",
                "type": "integer",
                "minimum": 0,
                "maximum": 255,
            },
            "meaning": {
                "description": "What the code means, as one sentence.",
                "type": "string",
            },
        },
        "required": ["exit", "last", "meaning"],
    })
}

/// Returns the schema of one goal heading, which `CommandRow`'s schema lists
/// under `goals`.
fn goal_schema() -> Schema {
    json_schema!({
        "type": "object",
        "properties": {
            "heading": {
                "description": "The heading, such as `Read`.",
                "type": "string",
            },
            "commands": {
                "description": "The names of the visible subcommands that the help page lists under the heading, in the heading's order.",
                "type": "array",
                "items": { "type": "string" },
            },
        },
        "required": ["heading", "commands"],
    })
}

/// Describes the object that `Serialize` writes. The descriptions are the
/// only ones the fields have, and a leaf's `--help` describes the columns
/// `path` and `about` from them.
///
/// This impl is written by hand so crumbkit needs no schemars `derive`
/// feature. The nested objects are written inline, so the schema has no
/// `$defs`.
impl JsonSchema for CommandRow {
    fn schema_name() -> Cow<'static, str> {
        "CommandRow".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::commands::CommandRow".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "One visible command of a branch of the command tree.",
            "type": "object",
            "properties": {
                "path": {
                    "description": "The command line that runs the command, such as `shelf books list`.",
                    "type": "string",
                },
                "about": {
                    "description": "The command's one-line description, or its full description when it has no one-line description.",
                    "type": ["string", "null"],
                },
                "long_about": {
                    "description": "The full description that the command's `--help` shows, when the command has one.",
                    "type": ["string", "null"],
                },
                "arguments": {
                    "description": "The arguments that the command's help shows, in definition order.",
                    "type": "array",
                    "items": argument_schema(),
                },
                "examples": {
                    "description": "The examples of the tool record's leaf for the command, in declared order.",
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "command": {
                                "description": "The full command line, such as `shelf books list --all`, with each word that a POSIX shell would split or expand between single quotes.",
                                "type": "string",
                            },
                            "about": {
                                "description": "The one-line description, which is empty when the example has none.",
                                "type": "string",
                            },
                        },
                        "required": ["command", "about"],
                    },
                },
                "columns": {
                    "description": "The columns of the result that the command prints, from the document of the tool record's leaf for the command. A headed command's text has no columns, so it lists the JSON fields that `--fields` selects, ending with `text` and `base64`.",
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "description": "The column's name, as the row type serializes the field.",
                                "type": "string",
                            },
                            "long": {
                                "description": "Whether `--long` adds the column.",
                                "type": "boolean",
                            },
                            "description": {
                                "description": "The first paragraph of the field's doc comment, when it has one.",
                                "type": ["string", "null"],
                            },
                        },
                        "required": ["name", "long", "description"],
                    },
                },
                "form": {
                    "description": "stream when the command prints a stream, one row on each line as each arrives, bytes when it prints one document that holds bytes, which its text prints unchanged, headed when it prints one document whose text prints each answer's bytes under a header, document when it prints any other document, and null when the tool record's leaf for the command declares no document.",
                    "enum": ["bytes", "document", "headed", "stream", null],
                },
                "line": {
                    "description": "The form of the text line that each row prints in place of a table row, such as `<POINTER> -> <TARGET>`, or null when the rows print a table or the command declares no document.",
                    "type": ["string", "null"],
                },
                "children": {
                    "description": "The field whose items each row prints in text as lines under it, such as `components`, or null when the rows print no such lines or the command declares no document.",
                    "type": ["string", "null"],
                },
                "children_help": {
                    "description": "The description of the child lines that help prints after `Children: `, which is the row type's own text or crumbkit's sentence that names the children field, or null when `children` is null.",
                    "type": ["string", "null"],
                },
                "exits": {
                    "description": "The exit codes that the tool declares for the command besides 0, 1 and 2, which apply to every command, in declared order. `<tool> help exit-codes` describes every code.",
                    "type": "array",
                    "items": exit_schema(),
                },
                "effects": effects::schema(),
                "writes": effects::writes_schema(),
                "goals": {
                    "description": "The goal headings that the command's help page lists its subcommands under, in the page's order. Only the row of a command whose help page prints goal headings for its group, which needs at least one goal that names a visible subcommand, has this field.",
                    "type": "array",
                    "items": goal_schema(),
                },
                "prints_no_result": {
                    "description": "true when the tool record's leaf for the command declares that it prints no result on standard output, so it offers none of --fields, --long and --full and its form is null. Only the row of such a command has this field, which tells it from a command whose leaf declares no document.",
                    "const": true,
                },
            },
            "required": ["path", "about", "long_about", "arguments", "examples", "columns", "form", "line", "children", "children_help", "exits", "effects", "writes"],
        })
    }
}
