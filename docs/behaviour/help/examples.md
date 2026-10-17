# Leaf examples and columns

This page covers the leaves of a tool's guide: the examples that a leaf's
help lists and the descriptions of the columns that its `--help` prints.
The rustdoc of [`Leaf`](../../../crates/crumbkit/src/help/leaf.rs) and
[`parse_to`](../../../crates/crumbkit/src/parse.rs) states each item's
contract.

## Leaves

A tool declares its leaves in its `Guide`. Each `Leaf` names a command by the
real names of its subcommands from the root, as `Group` does, and holds its
examples and the `Document` it prints. A command that the tool adds at run
time has a [`RuntimeLeaf`](README.md#runtime-leaves), which holds examples
and the document the command prints, as `Leaf` does.

`Leaf::document` keeps the document's id and description, the row type's
columns, and function pointers that build the row's JSON Schema and the
whole document's. `Leaf` therefore has no type parameter, and one guide
holds leaves whose rows have different types. The
[schema commands](../schemas/commands.md) and crumbkit-test's document
checks read the document through `Leaf::get_document` and
`Leaf::document_schema`.

`Leaf::effects` declares what the command changes, and `Leaf::stream`
declares a leaf whose document is a stream. Help prints them on the
`Effects:` and `Output:` lines that the [help page](README.md#lines-crumbkit-adds-to-a-help-page)
order places after the examples.

Why: ADR-0024, ADR-0035, ADR-0039, ADR-0041, ADR-0080.

## What a leaf's help shows

- A leaf's `-h` lists the examples as full command lines and ends with a
  `Next:` line that points to `--help` for the columns.
- A leaf's `--help` describes each example and each column.
- A prose document has no columns, so its leaf has neither the `Next:` line
  nor the column sections, and `schemas show` describes its fields.
- A [headed](../output/shapes/README.md#headed-bytes) document's text has no
  columns. Its `--help` lists the row's columns, then `text` and `base64`,
  under `JSON fields that --fields selects:`, and its `-h` ends with a
  `Next:` line that says `--help` explains each JSON field, also when its
  row type names no column. crumbkit-test's
  `check_guide` fails a headed leaf whose row has no top-level `text` or
  `base64` field, because its `Bytes` field is not marked
  `#[serde(flatten)]`.
- `parse` applies a leaf only to the deepest command on the walked path.
  `help::extend` adds every leaf's examples and no columns.
- The runtime never fails on a leaf.

Why: ADR-0024, ADR-0062.

## Column descriptions

A column's description is the doc comment of the row type's field, which
schemars puts into the JSON Schema, so the doc comment is the only place a
column is described. `--help` and crumbkit-test's `check_guide` both read
the columns through `Leaf::column_descriptions`, so the check sees what the
page prints. `parse` builds the schema only when the typed words ask for
`--help`, so a run that prints no long help never builds it.

Why: ADR-0024.

## Quoting of command lines

crumbkit prints each command line with
[`help::quote`](../../../crates/crumbkit/src/help/words.rs), which quotes a
word for a POSIX shell. On Windows, help prints the same POSIX quoting, so a
line pasted into PowerShell or `cmd` may need other quoting.

Why: ADR-0024, ADR-0003.

## Checks of leaves

crumbkit-test's [`check_guide`](../../../crates/crumbkit-test/src/guide.rs)
checks the leaves. It parses each example with clap and refuses an example
that asks for `--help` or `--version`. It requires an example for every
visible command without subcommands, a description for every column, a
compact column on a leaf whose rows are not a headed batch, a `Row::LINE`
that is one non-empty line and belongs to a leaf that is not headed, and a
document on a leaf whose effects change something. A row type that sets
`Row::WHOLE` may name no column only when it declares a `Row::LINE` and its
schema has no fields
([rows printed whole](../output/shapes/README.md#rows-printed-whole)). Its rustdoc
lists every rule.

Why: ADR-0024, ADR-0016, ADR-0063, ADR-0064, ADR-0065, ADR-0072.
