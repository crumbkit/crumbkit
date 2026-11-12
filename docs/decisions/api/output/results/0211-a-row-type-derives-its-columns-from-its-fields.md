# ADR-0211. A row type derives its columns from its fields

- Date: 2026-11-12
- Status: accepted

## Context

A row type named its columns as strings in `Row::COLUMNS`,
`Row::LONG_COLUMNS` and `Row::UNCUT_COLUMNS`. A name that was not a field
compiled, and the table then left the column out while the JSON held
`null` for it. crumbkit-test's `check_help` caught such a name only for the
row types that a tool record's leaves name, and one bug came from a name
that it did not check. crumbkit-derive derived only `Failure`.

## Decision

- crumbkit-derive gains `#[derive(Row)]` for a struct with named fields.
  `#[row(column)]`, `#[row(long)]` and `#[row(uncut)]` on a field put its
  name in `COLUMNS`, `LONG_COLUMNS` and `UNCUT_COLUMNS`, and
  `#[row(problem)]` names it in `PROBLEM`. `#[row(crate = PATH)]` on the
  type names the crate that holds `Row`, as `#[failure(crate = ...)]` does.
- Each list keeps the order of the fields, so a derived type reorders its
  columns by reordering its fields.
- The derive writes each name as serde serializes it. It reads a field's
  `rename` and the type's `rename_all`, with the eight case rules of serde
  1 copied by hand, because serde_derive does not export them and
  serde_derive_internals would add a dependency.
- An unknown key, a key given twice, `column` with `long`, `uncut` without
  either, `problem` without `column`, `problem` on two fields, any key on a
  field that serde skips or flattens, a type marked
  `#[serde(transparent)]`, and an enum, a union, a tuple struct or a unit
  struct are compile errors. Unit tests check each message.
- The derive writes only the four constants. A row that sets another item
  of `Row`, an enum, and a row whose `Serialize` is written by hand keep a
  hand-written impl.
- crumbkit's `derive` feature re-exports the derive at `crumbkit::Row`
  beside the trait. The feature stays off by default (ADR-0003), and the
  README and the tutorial teach the derive and turn the feature on.

## Consequences

A derived row type cannot name a column that is not a field of its JSON.
The other items of `Row` can gain keys in a later minor release without a
break. The README's example derives `Row`, so crumbkit's doctests run with
the `derive` feature.

Considered: `rename` alone, which gives wrong names to a type that uses
`rename_all`; the `derive` feature on by default, which builds one more
procedural macro crate for every tool; keys for every item of `Row`, which
wait until each item is one value.
