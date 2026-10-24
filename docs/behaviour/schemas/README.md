# Schemas

This page covers the shape of the JSON documents that a tool prints: the
envelope, the fields that each result shape adds, the failure document,
document ids and their versions. The [schema commands](commands.md) page
covers `schemas list`, `schemas show` and the checks that pin each schema.
[`Document`](../../../crates/crumbkit/src/document.rs) describes the id
type.

## The envelope

Every JSON document is an object with a `schema` id and the result under
`data` ([`finish`](../../../crates/crumbkit/src/output.rs)). A document with
`Envelope::Without` prints the result alone, and its schema is the
schema of `data` with the id as its title
([result shapes](../output/shapes/README.md#documents-without-the-envelope)). A dry run's
document holds `dry_run: true` after `schema`, and a real run's holds no
`dry_run` field ([effects](../effects/README.md#dry-runs)). The usage record
is one line of a log, so it has its fields at the top level beside `schema`,
without `data` ([usage log](../usage-log/README.md)).

- A listing's document always holds `truncated`, with every output flag, so
  a program can tell whether it has every row. It holds `total` when the tool
  counted the matching rows, and leaves it out for every listing from an
  uncounted source ([listings](../output/listings/README.md)).
- A listing or list document may hold `cut`, an object with the over-budget
  `code` and the `budget` in bytes, when the
  [output budget](../output/budget/README.md) cut it short.
- A batch's document always holds `failures`, which is empty when every id
  worked. A prose result's document holds its value under `data` and holds
  `failures` in the same way. Each entry has the fields of the failure
  document except `schema`, and crumbkit writes both from the same code, so
  the two shapes cannot drift. A batch's entry also holds the `key` it failed
  on and the `index` of that key among the keys, when the failure has a key.
  The failure document's schema holds both as optional fields.
- A bytes result's document holds `{"text": "..."}` under `data` when the
  bytes are valid UTF-8, and `{"base64": "..."}` in standard base64 with
  padding (RFC 4648) otherwise. Its schema, under the definition `Bytes`,
  is an object with both properties, a `oneOf` that requires one of them,
  and no other properties. A row that flattens a `Bytes` field therefore
  lists `text` and `base64` among its own properties.
- A row field of type `Age` holds a time in RFC 3339 in UTC with
  milliseconds, and its schema is a string in the `date-time` format,
  inline in the row's properties.
- A row field of type `Duration` holds an ISO 8601 duration with hours as
  its largest unit, such as `PT4M10S` or `PT0.3S`, and its schema is a
  string with a `pattern` of that form and no `format`, inline in the
  row's properties. A tool's own value type has the schema of its JSON
  form.

Why: ADR-0007, ADR-0016, ADR-0036, ADR-0039, ADR-0046, ADR-0050, ADR-0062, ADR-0071, ADR-0108, ADR-0147, ADR-0148.

## The failure document

A failure prints with `--json` as the `<tool>/failure/v1` document.

- A failure's `cause` is one string that joins the source chain, because
  those errors usually come from libraries whose wording and depth are not
  stable. A program matches on `code` and shows `cause` as one line. The
  schema accepts any spelling of `code`, because each tool chooses its own
  spelling.
- The document and each batch failure hold `alternatives` when the failure
  offers at least one alternative repair, each in the shape of `recovery`.
- A `run` step may hold `all`, the name of a list field of the same failure,
  such as `left_out`. The step then runs its call once with every value of
  that field, each as its own word, in place of the argument's placeholder,
  such as `<ID>...` ([output budget](../output/budget/README.md#batches)).
- The document and each batch failure hold `fields` after `cause` when the
  failure names at least one field, as one object of names to values in the
  order the failure gave them. A value is a string, an integer, a boolean,
  null, or an array or object of such values. No value holds a float at any
  depth, because JSON has no form for NaN. The schema promises the name rule
  of [fields](../failures/README.md#fields), so `finish` leaves out a field
  that breaks it. An object inside a value holds each key once (RFC 8259), so
  `finish` leaves out an entry whose key repeats an earlier one.
- With the `miette` feature, the document and each batch failure hold the
  diagnostic's labels and its related diagnostics. Each label holds an
  `excerpt` with the lines of its span and one line before and after where
  the source has them. Each related diagnostic holds its own labels and
  related diagnostics, down to 8 levels below the failure. A diagnostic at
  the ninth level is left out, which stops a diagnostic that lists itself as
  related from repeating forever. A label on a long line carries the whole
  line. The checked-in schema lists `labels` and `related` whether or not
  the feature is on, so there is one schema file.

Why: ADR-0007, ADR-0001, ADR-0046, ADR-0081.

## Document ids

A document id is a typed constant, a `Document`, so the compiler ties the id
to its result type and rejects a malformed id. `Document::new` takes a
one-line description, which becomes the schema's `description` and which
`schemas list` prints beside each id. A required argument means every
document has one, and the compiler finds a document without one.

A document id's component names the document's owner. It is labels joined
by single dots, each of lowercase letters, digits, `_` and `-`, so a tool
can name its owner as a domain such as `example.dev` does.

The name of the document that a command prints is the command's path below
the tool joined with `-`, such as `shelf/books-list/v1` for
`shelf books list`, and `<tool>/schemas-list/v1` for `schemas list`. An
agent can then predict a document's id from the command it ran, and the
command from an id. A cargo subcommand's path starts below its own name, so
`cargo crumbkit weigh` prints `crumbkit/weigh/v1`. The failure document and
the usage record are printed by no single command, so their names are
`failure` and `usage`. The name is a default. A tool that gives a document
another name, such as a name that scripts read from before a rename,
[declares](../adoption/declarations.md) it and needs no opt-out.

Why: ADR-0007, ADR-0035, ADR-0056.

## Versions

Removing, renaming or retyping a field, making it optional, or adding an enum
value needs a new `/vN` id. Readers ignore unknown fields, so a new optional
field keeps the `v1` id. Until crumbkit's first release, a document's schema
changes in place, and the `/vN` rule applies from that release on.

Every JSON document type has a 2020-12 `JsonSchema`, and
`schemas::document_schema` builds the schema of a whole document from its id
and its result type. A test compares each of crumbkit's schemas with its
checked-in file in a `schemas` directory.

A document's schema describes what the tool prints, so a field that is left
out when empty is optional and a field that is always printed is required,
whether or not it has a serde default. A file format's schema describes what
the tool reads, so a field with a serde default is optional there. In a
document with the envelope whose `--json` prints the row's columns, a row
requires only the fields in `Row::COLUMNS`, so the documents of `--json`,
`--json --long` and `--json --full` all match the schema. A column that the
row type skips when it is empty prints `null` in the compact document, so its
property's type admits null. The column stays optional, because
`--json --full` leaves it out. The compact document also prints `null` for
a column or the children field that the row type, or one variant of an enum
row, has no field for. Such a row schema or variant gains the property
`{"type": "null"}` for that name when it has a `properties` map or
`additionalProperties: false`, so a variant that refuses unknown fields
still matches. A name that a schema around the variant declares, such as a
field of a struct that flattens the enum, gains no such property.

Why: ADR-0007, ADR-0110, ADR-0113, ADR-0132.
