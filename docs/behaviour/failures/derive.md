# Deriving Failure

`#[derive(Failure)]` writes a type's `Failure` impl and its `CODES` list
from a `#[failure(...)]` attribute on each variant. The rustdoc of
[`crumbkit-derive`](../../../crates/crumbkit-derive/src/lib.rs) is the
reference for the keys and shows the code the derive writes for an example
type. shelf derives the impl of its books failures.

## Turning it on

The derive sits behind the `derive` feature. crumbkit-core's `derive`
feature re-exports the macro from crumbkit-derive beside the trait of the
same name, and crumbkit's `derive` feature turns it on, so
`crumbkit::Failure` names both. A procedural macro must live in its own
crate. The derive writes paths under `::crumbkit` unless its `crate` key
names `::crumbkit_core`. The feature is off by default, so a tool that
writes its `Failure` impls by hand builds no syn.

Why: ADR-0001, ADR-0032, ADR-0003.

## What it writes

A hand-written impl repeats one `match` per method, and a hand-written
`CODES` list drifts from the variants, so the derive writes both.

- `code` takes a path to a constant, so `Code::new` still checks the
  segments at compile time. The derive checks each field name by the same
  rule.
- `hint` text with braces goes through `format!`.
- `forward` returns every answer of a wrapped failure, so the wrapper's
  `CODES` leaves the wrapped codes to the wrapped type's own list.
- The derive writes no `Display`, so the message stays with the tool or
  another derive.

The keys and the code that the derive writes for them are public API.
Removing or renaming a key, or changing what a key returns, is a breaking
change. A new key is a minor change, and so is a new trait method that the
derive starts to write.

Why: ADR-0001, ADR-0002.
