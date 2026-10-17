# Result shapes

This page covers the shapes of result that
[`finish`](../../../../crates/crumbkit/src/output.rs) prints besides rows
and listings, how a tool shapes the document of its result, and rows
that print their own text line. The
[output](../README.md) page covers the formats, the text table, the view
flags and which stream each line goes to.

## Batches

A command that takes many ids returns a
[`Batch`](../../../../crates/crumbkit-core/src/batch.rs), whose rustdoc
describes how a handler builds it. The rows print on standard output and the
failures on standard error, and with `--json` both go into one document.
Every answer has a key, and a key that fails gets its own failure. The batch
keeps each key as the text of its `Display`, which is the word a caller
types, because the [over-budget failure](../budget/README.md#batches) puts the keys
into a command line. A key may repeat, and nothing in
crumbkit assumes that keys are unique. Each failure of a batch holds its key
and the key's index among the keys, counted from 0, so a reader matches a
failure to its key even when a key repeats. `key` and `index` sit before
`code` in the failure's JSON, and a `key:` line such as `key:   7 (index 1)`
follows the `error:` line in its text. They are members of the failure and
sit outside `fields`. A failure pushed after the last key has neither. A
batch with at least one failure exits with the code of its first failure,
which is 1 unless the guide declares another for that failure's code
([exit codes](../../failures/exit-codes.md)).

Why: ADR-0016.

## Prose

A command whose answer is a few sentences returns a
[`Prose`](../../../../crates/crumbkit-core/src/prose.rs), whose rustdoc
describes its text, its JSON document and its failures. A prose result with
failures exits as a batch with failures does, with the code of its first
failure. `--fields`, `--long`
and `--full` are refused for a prose result as a refused command line. The
hint of `--fields` and `--long` names `--json`, and the hint of `--full` says
that `--json` already prints the whole result.

Why: ADR-0016.

## Bytes

A command whose answer is bytes, such as a file's contents, returns
[`Bytes`](../../../../crates/crumbkit-core/src/bytes.rs), whose rustdoc
describes its text, its JSON form and its use as a row field. `--fields`, `--long`
and `--full` are refused for it as a refused command line, and each refusal
names `--json` or says that `--json` already prints the whole result. A dry
run's line follows a bytes result's text on standard error, so standard
output holds the bytes alone.

Why: ADR-0016.

## Headed bytes

A command that returns the bytes of several answers, such as `show
<NAME>...`, returns a batch whose row type implements
[`Headed`](../../../../crates/crumbkit-core/src/row.rs) and whose document
calls [`Document::headed`](../../../../crates/crumbkit-core/src/document.rs).
The row usually holds its bytes in a `Bytes` field marked
`#[serde(flatten)]`, so its JSON and its schema hold `text` or `base64` at
the top level of the row.

The text prints each row's bytes unchanged, as `head` prints several files:

```text
==> to-read <==
# Books to read next
North and South

==> finished <==
# Books finished this year
Emma
```

- A header line `==> KEY <==` precedes each row when the caller gave more
  than one key, with the key's control characters escaped. With one key the
  bytes print alone.
- A line feed precedes every header but the first, and crumbkit adds no
  line feed after a row's bytes.
- A row pushed after the last key has the header `==> (index N) <==`.
- A key whose lookup failed gets no header, and its failure prints on
  standard error with its `key:` line, as in any batch.
- A row whose `Headed::bytes` returns `None` prints under its header as one
  line of compact JSON with every field.

Each row prints whole or not at all within the output budget
([headed bytes](../budget/README.md#headed-bytes)).

With `--json` the batch prints as its ordinary document with every field of
each row, unless `--fields` names some. `--fields` needs `--json`, because
the text has no columns, and `--long` and `--full` are not offered. The help
of the leaf ends with an `Output:` line that describes this text.

Why: ADR-0062.

## Rows printed whole

A row type whose JSON readers expect every field of each row sets
`Row::WHOLE` ([`Row`](../../../../crates/crumbkit-core/src/row.rs)). With
`--json`, each row then prints as the type serializes it, without `--full`,
so a field that the type leaves out when it holds nothing stays out. This
holds for a single row, a list, a listing, a batch and each line of a
stream.

- `--full` is refused, because the JSON already holds every field.
- `--fields` selects fields in text and in JSON, and prints `null` for a
  field that a row leaves out.
- `--long` adds the long columns to the text table.

A row type that sets `Row::WHOLE` may leave both `Row::COLUMNS` and
`Row::LONG_COLUMNS` empty, as a row that serializes as an array must. Its
text then prints each row as one line of compact JSON with no header, and
the footer and notes print on standard error as for a table. A stream
prints each item that way. `--fields` and `--long` are refused, because
there is no table to shape. The help of the leaf ends with an `Output:`
line that describes this text, and `commands list` writes the form
`json-rows` ([commands list](../../help/commands-list.md)). A row type that
also declares `Row::LINE` prints its own lines instead
([rows with their own text line](#rows-with-their-own-text-line)).

crumbkit-test's `check_guide` fails a row type that sets `Row::WHOLE`,
names no columns and has fields in its JSON Schema, because those fields
could be the columns of a table. Its `check_document_ids` requires an
[opt-out](../../adoption/opt-outs.md) from `documents/rows-whole` for each
command whose row type sets `Row::WHOLE`, because the document costs more
tokens than the compact columns.

Why: ADR-0065, ADR-0064.

## Rows with their own text line

A row type may print each row in its own form. `Row::LINE` names the form
for help, such as `Some("<NAME>: <ABOUT>")`, and `Row::text_line` builds
each row's line. Without `--json`, `--long` and `--fields`, the text then
prints each row's line with control characters escaped and no header, and
a row whose `text_line` returns `None` prints as one line of compact JSON.
`--long` and `--fields` print the table, and JSON is unchanged. Streams,
the [output budget](../budget/README.md) and the listing footer work as for a
table, and help prints a `Text:` line ([help](../../help/README.md)). A row
type that also sets `Row::WHOLE` and names no columns offers neither flag,
so its `Text:` line names no table. crumbkit-test's `check_guide` fails a
`LINE` that is not one non-empty line, and a
[headed](#headed-bytes) leaf whose row type declares one.

Why: ADR-0064.

## Documents without the envelope

A command that keeps the shape of an existing document prints a document
from [`Document::without_envelope`](../../../../crates/crumbkit-core/src/document.rs).
With `--json` it prints the result alone with every field:

- a row as an object;
- a list, listing or batch as an array;
- a prose result's value;
- a bytes result's `text` or `base64` object.

`--fields` selects fields as it does for any other document, `--long` changes
nothing in its JSON, and `--full` is refused. A listing prints one page
without `total`, and its text output keeps the footer and the next step.

The failures of a batch or prose result print as text on standard error even
with `--json`, and the command exits as any batch or prose result with
failures does. A batch whose document also calls
`Document::failures_in_order` prints with `--json` one array that holds each
row and each failure document in the order of the keys, with the
over-budget failure last, so a program reads every answer from standard
output. `failures_in_order` is opt-in, because the readers of an existing document
expect rows alone, and a const panic rejects it on a document with
crumbkit's own envelope, which holds its failures under `failures`. A
document with the tool's own keys holds the array under its data key, as
the next section describes. A reader tells a failure
from a row by its `schema`, so crumbkit-test's `check_guide` fails such a
document whose row type has a field named `schema`. A command that
fails as a whole still prints the failure document, so every command fails in
one shape.

crumbkit-test's `check_document_ids` requires an
[opt-out](../../adoption/opt-outs.md) for each command that prints a document
without the envelope, and the opt-out report lists `failures_in_order` as a
[declaration](../../adoption/declarations.md).

Why: ADR-0016.

## Documents with the tool's own keys

A tool whose readers expect other names for the two keys of the envelope
declares them with
[`Document::with_keys`](../../../../crates/crumbkit-core/src/document/keys.rs),
such as `with_keys("$type", "answers")`. The document prints its id under
the type key and its result under the data key. `Document::with_type` puts
a fixed string, such as `tool/get-report/v1`, under the type key in place
of the id. Every other part of the envelope stays: `total`, `truncated`,
`summary`, `cut`, `failures` and `dry_run` print as they do in crumbkit's
envelope, and the view flags, the output budget and streams work as they do
for any other document. A stream prints both keys on each line.

A const panic rejects a key that is one of the names crumbkit writes beside
the result (`dry_run`, `total`, `truncated`, `summary`, `cut` and
`failures`), two keys that are the same, and `with_keys` together with
`without_envelope`.

A batch that also calls `Document::failures_in_order` prints each row and
each failure under the data key in the order of the keys, repeats
included, with the over-budget failure last, and prints no `failures`. Each
failure prints in the guide's failure form when it has one.

A listing whose document calls `Document::summary_at_top` prints the
fields of its summary at the top of the document beside the two keys, in
place of a `summary` object, so a tool can print fields such as a
location or a degraded flag there. The summary must serialize as an object.
A summary field whose name is one of the document's keys or one of the
names crumbkit writes fails the write: the run exits with 1, prints
nothing on standard output and names the field on standard error. The
summary line of the text output stays.

`schemas list` shows such a document with the kind `own-envelope`, and its
schema names the declared keys, with the summary's schema under `allOf`
when the summary prints at the top. The keys are a
[declaration](../../adoption/declarations.md), so no opt-out is needed, and
the opt-out report lists them as an `envelope-keys` customisation.

Why: ADR-0059.

### Listings without a total

A listing whose document declares its keys and calls
[`Document::without_total`](../../../../crates/crumbkit-core/src/document/keys.rs)
leaves `total` out of its JSON, for readers that know a listing without a
count. The JSON keeps `truncated`, so a reader still learns whether rows
were left out. The text footer, the output budget's cut line and the
[usage log](../../usage-log/README.md) still count the rows. The document's
schema has no `total` property. A const panic rejects `without_total` on a
document without `with_keys`, because crumbkit's own envelope holds `total`
whenever the tool counted the rows.

crumbkit-test's `check_document_ids` requires an
[opt-out](../../adoption/opt-outs.md) from `documents/total` for each command
that prints such a listing, so the opt-out report states what the missing
count costs a reader.

Why: ADR-0066.
