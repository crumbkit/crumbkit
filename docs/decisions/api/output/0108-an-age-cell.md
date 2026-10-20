# ADR-0108. An age cell

- Date: 2026-10-20
- Status: accepted

## Context

A listing often has a column that says how long ago something happened. A
person reading the table wants `3h`, and a program reading JSON wants the
time itself, because an age is stale as soon as it prints. crumbkit renders
each text cell from the row's serialized JSON value, so a field printed the
same string in both formats.

API-1 asks a new row field type for a second use with its own shape. The
first use is a record's own timestamp. The reading example adds the second:
the age of a list's file since it was last modified, which the file system
supplies and may not know.

## Decision

- `crumbkit::Age` is a row field type. `Age::since(at, now)` holds the time
  and the duration from it to `now`, which saturates to zero. The tool
  passes `now`, so crumbkit reads no clock and a golden file stays
  deterministic.
- The text form, which is also its `Display`, is the largest whole unit:
  seconds, minutes, hours or days. Days are the largest unit because the
  length of a month or a year depends on a calendar.
- The JSON form is RFC 3339 in UTC with milliseconds, the usage log's form,
  written with humantime. The schema is an inline string in
  the `date-time` format.
- `Age`'s `Serialize` writes the compact age while a thread-local text mode
  is set, and the RFC 3339 time otherwise. crumbkit sets the mode with a
  guard that restores the previous value when it drops, and only around
  the serialization whose value becomes text cells: the rows of a text
  table, a stream's text line, and the table that `Room` measures.

## Consequences

A tool adds an `Age` column without a second row type for text. Every JSON
form, a failure document and a usage record hold the RFC 3339 time, and a
tool that serializes a row itself gets it too. A time before 1970 or after
the year 9999 has no four-digit RFC 3339 form and fails to serialize in
JSON. The mode is per thread, so it holds because crumbkit serializes rows
on the thread that calls `finish` or drives a stream. A picker's label,
which `pick::from_rows` builds from a row's cells, still shows the time.

Considered: a `Row` declaration of the columns that hold a timestamp, which
puts a second list of field names beside `COLUMNS` and still needs a
conversion for each format; a serializer wrapper that recognises a marker
newtype by name and unwraps it for text cells, which depends on a type name
that serde does not promise to pass on.
