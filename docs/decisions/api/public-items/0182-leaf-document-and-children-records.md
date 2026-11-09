# ADR-0182. A leaf's document and a row's child lines are records

- Date: 2026-11-09
- Status: accepted
- Supersedes: ADR-0120, ADR-0135, ADR-0137 and ADR-0151, which it amends

## Context

`Leaf` had 36 public functions, and about 15 of them read one facet: the
document that the leaf's command prints and the columns of its row type.
crumbkit's help, `commands list` and schema code and crumbkit-test read them,
and the builder methods that a tool needs were hard to find among them. `RuntimeLeaf` stored a `Leaf` without a path to hold its
document, because no smaller type existed.

`Row` had four constants for child lines: `CHILDREN`, `CHILD_INDENT`, `SPACED`
and `CHILDREN_HELP`. Three of them meant nothing unless `CHILDREN` was set.
`check_help` caught only `CHILDREN_HELP` without `CHILDREN`, at test time, and
the other two states compiled and silently did nothing.

## Decision

- `help::LeafDocument` holds the document facet.
  `Leaf::get_document` returns `Option<&LeafDocument>`. The record's getters are
  `document`, `is_stream`, `rows_whole`, `problem`, `line`, `uncut_columns`,
  `shape`, `document_schema`, `row_has_fields`, `column_descriptions`,
  `children` and `children_column`. `Children::text` returns the text that
  help prints. `Leaf::offers` stays on
  `Leaf`, because a leaf without a document also answers it.
- `RuntimeLeaf` holds `Option<LeafDocument>`.
- `Row::CHILDREN` is `Option<Children>`. `Children` is a const-built record that
  holds the field, the indent, the blank line and the help text, so a row type
  cannot set one of those settings without naming the field. A row type can
  still override `child_lines` or `long_child_lines` with `CHILDREN` at `None`,
  which compiles and does nothing, as it did before. `CHILD_INDENT`,
  `SPACED` and `CHILDREN_HELP` no longer exist. `child_lines` and
  `long_child_lines` stay as trait methods, because they read the row.
- `check_help` loses the rule about help text without a children field, because
  no such row type can be written.

## Consequences

A tool that declares child lines writes one constant:

```rust
const CHILDREN: Option<Children> = Some(
    Children::field("notes").indent(0).spaced().help("one line for each note."),
);
```

A reader of `Leaf::get_document` handles a leaf without a document once, at
that call, and the record's getters answer for a document that exists.

Considered: keeping the getters on `Leaf` and adding the record beside them,
which leaves two ways to read each fact; a `get_children` on `Leaf`, which
keeps the facet on the type a tool uses to declare a command; asserting at
compile time that the three settings need `CHILDREN`, which a const assert on a
trait constant can only do with a separate check in every leaf builder.
