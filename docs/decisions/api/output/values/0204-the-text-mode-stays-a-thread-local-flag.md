# ADR-0204. The text mode stays a thread-local flag

- Date: 2026-11-11
- Status: accepted

## Context

A shown value serializes as its `Display` while the text mode is set on the
current thread, and as its JSON form otherwise (ADR-0147). The mode is a
thread-local flag, which serde knows nothing about. A serializer that
carried the mode itself would remove the flag and its two limits, which the
Consequences name.

ADR-0147 rejected a serializer that recognises a marker type by name, on the
ground that the serializer cannot reach the value's `Display`. That ground
is wrong. The value can write its own `Display` once the serializer tells it
which form to write.

## Decision

- The text mode stays a thread-local flag in the `shown` module.
- One crate-private function, `shown::text_cells`, sets the flag for the
  length of one closure and restores the value it found, when the closure
  returns and when it panics. No other code sets the flag.
- `shown::serialize` is the only code that reads the flag.

## Reason

A wrapper serializer was built and tested. It wrapped the serializer of
each text serialization, reported the text form through
`Serializer::is_human_readable`, and marked a shown value with a newtype
struct of a reserved name, so that only values that opt in changed form.

serde serializes two row shapes by buffering their fields through a private
serializer of its own: a struct variant of an enum that a row flattens with
`#[serde(flatten)]`, and a struct variant of an enum that a newtype variant
of an internally tagged enum holds. The private serializer does not carry the wrapper's signal, so
a shown value in either position printed its JSON form in a text cell. The
flag is read by the value itself, on the thread that serializes it, so it
reaches a value through any serializer.

The tests of `crates/crumbkit/tests/shown.rs` cover five row shapes: a
plain struct, a row that flattens a struct, a row that flattens an enum
with a struct variant, an internally tagged enum, and an internally tagged
newtype variant that holds an enum with a struct variant. The flag prints
the `Display` in all five. The wrapper serializer printed the JSON form in
the third and the fifth.

## Consequences

The two limits of the flag remain. A `Serialize` impl that serializes on
another thread gets the JSON form in a text cell. A tool's `Serialize` impl
that serializes a shown value for another purpose inside a text
serialization gets the `Display`.

A new place that builds text cells calls `text_cells` around its one
serialization. A change that replaces the flag must pass the five row-shape
tests.

Considered: the wrapper serializer, which regresses two of the five row
shapes; and text written by a `Row` method without serde, which ADR-0147
already rejected because each row type would describe its fields twice.
