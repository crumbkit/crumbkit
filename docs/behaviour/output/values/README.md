# Value types

A value type is a field type that reads one way in text and another way in
JSON. Its text form is its `Display`. Its JSON form is whatever it chooses,
such as a number, an ISO 8601 string or an object. crumbkit picks the form
for each place that prints the value:

| Where | Form |
| --- | --- |
| Text table cells, `--fields` text, a stream's text line | text |
| The default child lines under a row | text |
| The labels of a picker's choices from `pick::from_rows` | text |
| A failure's message | text, which the tool writes with `Display` |
| `--json` in every view, a stream's JSON lines | JSON |
| A failure's `fields` through `Field::shown` | JSON |
| The row's JSON Schema | the schema of the JSON form |
| A tool that serializes the value itself | JSON |
| A picker choice's value from `pick::from_rows` | JSON |

[`Shown`](../../../../crates/crumbkit/src/shown.rs) is the trait of a value
type and `crumbkit::shown!` writes its `Serialize` and `JsonSchema`.
crumbkit switches the form with a mode that it sets on the current thread
only while it serializes rows into text cells, so a tool never sets it.
The tool's own code that runs inside that serialization, such as a
`serialize_with` function, a hand-written `Serialize` or a `Display`, sees
the text form of every value type. crumbkit decides `Row::is_problem`
before it sets the mode, so the default judges the JSON form.

Why: ADR-0147.

## Built-in value types

- [`Age`](../../../../crates/crumbkit/src/age.rs) holds a time. Text shows
  how long ago it was, such as `3h` or `2w`, and JSON holds the time in RFC
  3339 in UTC with milliseconds.
- [`Duration`](../../../../crates/crumbkit/src/duration.rs) holds a length
  of time. Text shows `300ms` under a second, seconds with one decimal
  place under a minute, such as `1.9s`, and the two largest units from a
  minute up, such as `4m 10s` or `3d 2h`. JSON holds an ISO 8601 duration
  with hours as its largest unit, such as `PT4M10S`, `PT1H0M5S`, `PT0.3S`
  or `PT72H`, and its schema gives the pattern of that form.
- [`Bytes`](../../../../crates/crumbkit/src/bytes.rs) holds a file's bytes.
  Its JSON form is `{"text": ...}` or `{"base64": ...}`, and it keeps that
  form in text cells too, because a row flattens it with
  `#[serde(flatten)]` and serde flattens only an object. A table shows such
  a field as compact JSON.

Why: ADR-0108, ADR-0141, ADR-0147, ADR-0148.

## Writing your own value type

A tool writes the `Display`, implements `Shown` and calls `shown!`. The
defaults of `Shown` make the JSON form the text as a string, so a status or
an id that reads the same in both forms needs an empty `impl Shown`. A
value with a typed JSON form overrides `json` and `schema` together.
shelf's `Pages` reads `432 pages` in a table and is the number `432` in
JSON:

```rust
use std::fmt;

use crumbkit::Shown;
use crumbkit::schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::Serializer;

/// A number of pages, which reads `432 pages` in text and `432` in JSON.
#[derive(Clone, Copy)]
struct Pages(u32);

impl fmt::Display for Pages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} pages", self.0)
    }
}

impl Shown for Pages { fn json<S: Serializer>(&self, serializer: S) ->
Result<S::Ok, S::Error> { serializer.serialize_u32(self.0) }

    fn schema(generator: &mut SchemaGenerator) -> Schema {
        u32::json_schema(generator)
    }
}

crumbkit::shown!(Pages);
```

The type is then a plain field of a row, and `COLUMNS`, `LONG_COLUMNS`,
`--fields`, `--full` and the schema treat it as any other field. `shelf
books list --long` prints `432 pages` in its `PAGES` column, and `--json
--full` holds `"pages":432`.

`Shown::schema_name` returns `None` by default, which puts the schema
inline in each property, so the property keeps the field's doc comment as
its description and an `Option` of the value is the schema or null. A type
whose schema is large, such as an object, returns a name to define the
schema once under `$defs`.

A row whose `Row::CHILDREN` field is a list of value types prints each
value's text as a child line without overriding `Row::child_lines`. A
failure passes a value to `Field::shown`, and writes the same value with
`{}` in its message, so the message and the `fields` object agree.

`shown!` names serde and schemars through crumbkit, so the call needs no
dependency on either. A type that overrides `json`, as `Pages` does, names
serde's `Serializer` and depends on serde. `shown!` takes one concrete
type, such as `Count<u32>`. A generic impl for every parameter is written
by hand: `Serialize` calls `crumbkit::shown::serialize`, and `JsonSchema`
returns `Shown::schema`. A type that a row flattens writes its own
`Serialize`, which calls `Shown::json`, as `Bytes` does. A type from
another crate needs a newtype of the tool's own, as for any trait from a
third crate.

Why: ADR-0147.
