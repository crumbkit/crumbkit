//! The JSON Schema of one argument of a command, which `CommandRow`'s
//! schema lists under `arguments`.

use schemars::{Schema, json_schema};

/// Returns the schema of one argument, which `CommandRow`'s schema lists
/// under `arguments`.
pub(super) fn argument_schema() -> Schema {
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
