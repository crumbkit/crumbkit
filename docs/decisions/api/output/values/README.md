# ADR-0147. A value type prints its Display in text and its own form in JSON

- Date: 2026-10-24
- Status: accepted
- Supersedes: ADR-0108, which it amends
- Missed: ADR-0108 gave the text and JSON forms only to `Age`, inside crumbkit, so a tool's own value type could not print one form in a table and another in JSON.

## Children

- [ADR-0148. A length of time is a row field type](0148-a-length-of-time-is-a-row-field-type.md)

## Context

crumbkit renders each text cell from the row's serialized JSON value. A
value that should read one way in a table and another in JSON, such as a
count of milliseconds that reads `300 ms` in text and `300` in JSON, had
three ways to print, and each cost the tool something. It could serialize
the text form and lose the typed JSON. It could serialize the JSON form and
show raw units in the table. It could write the text separately, in a
second row type or a `child_lines` override, and keep the two in step by
hand. `Age` avoided all three only because it is inside crumbkit and read a
crate-private thread-local that `finish` set while it built text cells
(ADR-0108).

serde gives a `Serializer` no way to call any method of a value other than
`Serialize::serialize`. A crumbkit serializer for text cells could
recognise a marker type by its name, but it could not reach the value's
`Display` through it. The one signal that serde passes from a serializer
to a value is `Serializer::is_human_readable`, which is `true` for
`serde_json` and which many crates read to choose between two encodings
of their own.

## Decision

- `crumbkit::Shown` is a trait for a value type. Its text form is its
  `Display`, which the trait requires. `Shown::json` writes the JSON form
  and `Shown::schema` describes it, and both default to the text form as a
  string. `Shown::schema_name` names a schema definition, and its default
  of `None` puts the schema inline.
- `crumbkit::shown::serialize` writes the `Display` while the text mode is
  set on the current thread, and `Shown::json` otherwise. The
  `crumbkit::shown!` macro writes a type's `Serialize`, which calls that
  function, and its `JsonSchema`, which returns `Shown::schema`. The macro
  is `macro_rules!` and names serde and schemars through crumbkit, so it
  needs no derive feature and no syn.
- The text mode is the thread-local guard of ADR-0108, moved from `Age` to
  the `shown` module and kept crate-private. crumbkit sets it only around
  a serialization whose value becomes text: the rows of a text table, a
  stream's text line, the table that `Room` measures, and the default
  child lines.
- `Row::child_lines` defaults to the items of the `Row::CHILDREN` field,
  serialized in the text mode, when that field is an array of strings, and
  to no lines otherwise. A field of shown values therefore prints one child
  line for each value with no override.
- `Field::shown` builds a failure field from the JSON form of a shown
  value. The failure's message holds the text form, which the tool writes
  with the value's `Display`.
- `Age` implements `Shown` through the macro, and its `Serialize` no longer
  reads the mode itself. `Bytes` implements `Shown`, and its `Serialize`
  writes the JSON form in both modes, because a row flattens its bytes with
  `#[serde(flatten)]` and serde flattens only a map. Nothing outside
  `age.rs` and `bytes.rs` treats either type differently from other values.
- `Shown` requires `Display`, because a value that reads one way in a
  sentence and another in a table cell is two values, and a tool then
  writes two types.

## Consequences

A tool adds a value type with a typed JSON form by writing its `Display`,
two methods and one macro call, and uses the type as a plain field of a
row. `COLUMNS`, `--fields`, `--long`, `--full` and the schema treat it as
any other field, and no row needs `#[schemars(with = "String")]`. The JSON
of `Age` and `Bytes` is unchanged.

The mode is per thread and its lifetime is one serialization inside
`finish`, `Room` or a stream, on the thread that calls them. A tool that
serializes a row itself always gets the JSON form. A tool cannot set the
mode, so a value type cannot observe a mode that crumbkit did not set.

A row whose `CHILDREN` field holds strings and that kept the old default
now prints those strings as child lines. A row that overrides
`child_lines` prints as before. The default serializes the whole row again
for each row that prints child lines, so a row of such a type serializes
twice in the text table, and up to four times when `Room` measures it
first. The code marks this cost and what would remove it.

Code of the tool that runs inside a serialization in the text mode sees
the text form of every shown value: a `serialize_with` function, a hand
`Serialize`, a `Display` and an `is_problem` override that serializes the
row. crumbkit decides `Row::is_problem` before it sets the mode.

Considered: a `Row::cells` method that every row writes by hand, which
costs each row type a second description of its fields; a derive that
writes `Row::cells`, which makes every tool build syn; text derived from
the JSON form through the schema's `format`, which needs a formatter for
each format in crumbkit and gives a tool's own type no text of its own; a
serializer that recognises a marker type by name, which cannot reach the
value's `Display`; a text cell serializer whose `is_human_readable` is
`false` for the JSON form, or the reverse, which changes how chrono, uuid,
IP addresses and similar types serialize in every row, not only the
values that opt in; and `Shown` without the `Display` bound, which lets
one value print two different texts.
