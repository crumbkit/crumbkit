# The after_double_dash field

This page covers `after_double_dash` in each entry of `arguments` in a
`commands list` row ([commands list](../commands-list.md)).

The field is `true` for an argument that clap's `last` setting marks, and
`false` for every other argument. A reader that builds a command line writes
`--` before the values of an argument whose field is `true`, as in
`starmap run -- cargo run`.

A positional argument set with `trailing_var_arg` has `false`. A reader still
needs `--` when its first value starts with `-`, unless the argument sets
`allow_hyphen_values`.

## Older documents

`commands-list-v1` took the field without a version bump, because readers
ignore fields they do not know. A document from an older crumbkit lacks the
field, and a reader treats it as `false`, as crumbkit-markdown does for
`runs_program` in the effects object.
