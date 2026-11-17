# ADR-0247. A row type derives its impl from its fields

- Date: 2026-11-17
- Status: accepted
- Amends: ADR-0211, which gave `#[derive(Row)]` the keys for the column lists and the problem field

## Context

`#[derive(Row)]` wrote `COLUMNS`, `LONG_COLUMNS`, `UNCUT_COLUMNS` and the
problem and warning fields. Two adopting tools kept hand-written impls for
three reasons. Some rows set `CHILDREN`, `WHOLE` or `Problem::by`, which the
derive had no key for. One row took two columns from a field that serde
flattens, which the derive refused. Several rows were a local newtype over a
type from another crate, where the orphan rule forbids `impl Row` on the
foreign type and the derive could not see that type's fields.

## Decision

- `#[row(whole)]` on the type sets `WHOLE`. `#[row(problem_by = EXPR)]` and
  `#[row(warning_by = EXPR)]` on a column set `PROBLEM` and `WARNING` with
  `Problem::by` and `Warning::by`, and the column's serialized name is the
  column. `#[row(children)]` on a field sets `CHILDREN` with
  `Children::field`, and `children(indent = N, spaced, help = "...",
  lines = EXPR, long_lines = EXPR)` adds the matching builder calls.
  `EXPR` is a path or a closure that does not capture, which a constant
  holds as a function pointer.
- `#[row(flatten)]` on a field that also has `#[serde(flatten)]` splices the
  field type's `COLUMNS`, `LONG_COLUMNS` and `UNCUT_COLUMNS` into the row's
  lists at the field's position. The field type must implement `Row`. A
  constant assertion fails the build when that type sets `PROBLEM`,
  `WARNING` or `CHILDREN`, because the row would drop the setting. A hidden
  `crumbkit::__row_columns` const function joins the lists and fails the
  build when its length disagrees with the lists it joins.
- `#[row(remote = TYPE)]` on a local mirror struct writes `impl Row for
  TYPE` from the mirror's fields. The mirror names the foreign type's
  fields with their serialized names, as serde's `remote` does, and carries
  its `rename` and `rename_all` attributes without deriving `Serialize`, so
  the derive registers `serde` as a helper attribute. The derive writes a
  constant function that builds the mirror from itself, so the mirror's
  fields count as read. Nothing ties the mirror to the foreign type at
  compile time, and crumbkit-test's `check_help` fails a column that is not
  a field of the leaf's schema.
- Each combination that cannot mean anything is a compile error that names
  the field or the type: `problem` with `problem_by`, `warning` with
  `warning_by`, either `_by` key without `column`, `children` with a column
  key or on a second field, an unknown or repeated `children` option,
  `flatten` without `#[serde(flatten)]` or with another key, `flatten` or
  `remote` on a generic type, a type key on a field and a field key on the
  type. A type marked `#[serde(transparent)]` stays refused unless it gives
  `remote`.
- The derive still leaves `LINE`, `cell_style`, `in_default_text` and enums
  to a hand-written impl.

## Consequences

An adopting tool derives every row that sets the items above, and keeps a
hand-written impl only for `LINE`, `cell_style`, `in_default_text` and an
enum. A mirror whose names differ from the foreign type's JSON is found
only by `check_help`, and only for a leaf that the tool record gives.

Considered: a `line` key, which waits until a second adopter needs it; and a
compile-time check that a mirror's fields exist on the foreign type, which
needs the foreign type's fields and so cannot be written as a derive.

## Children

- [ADR-0211. A row type derives its columns from its fields](0211-a-row-type-derives-its-columns-from-its-fields.md)
