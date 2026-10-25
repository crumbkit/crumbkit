# Result shapes

This page covers the shapes of result that
[`finish`](../../../../crates/crumbkit/src/output.rs) prints besides rows
and listings, how a tool shapes the document of its result, and rows
that print their own text line. The
[output](../README.md) page covers the formats, the text table, the view
flags and which stream each line goes to.

## Batches

A command that takes many ids returns a
[`Batch`](../../../../crates/crumbkit/src/batch.rs), whose rustdoc
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
([exit codes](../../failures/exit-codes.md)). The over-budget failure that names
the answers left out prints after the batch's own failures.

`Batch::note` sets a line that the text prints on standard error after the
rows and before the failures, in the place of a
[listing's note](../listings/README.md#the-lines-that-describe-a-listing). It
prints also when the output budget leaves out some answers, and the JSON
document leaves it out. shelf's `books show` notes which of the ids it
cannot find name books that were removed from the shelf.

Why: ADR-0016, ADR-0077, ADR-0081.

## Prose

A command whose answer is a few sentences returns a
[`Prose`](../../../../crates/crumbkit/src/prose.rs), whose rustdoc
describes its text, its JSON document and its failures. A prose result with
failures exits as a batch with failures does, with the code of its first
failure. `--fields`, and `--long` without `--json`, are refused for a prose
result as a refused command line with a hint that names `--json`. `--long`
or `--full` with `--json` adds nothing and [prints a
note](#rows-printed-whole).

`Prose::note` sets a line that the text prints on standard error after the
text and before the failures, as a batch's note prints, and the JSON
document leaves it out. A prose result that the output budget refuses
prints no note. shelf's `books summarize` notes the removed books that a
summary of the whole shelf leaves out.

Why: ADR-0016, ADR-0077, ADR-0149.

## Bytes

A command whose answer is bytes, such as a file's contents, returns
[`Bytes`](../../../../crates/crumbkit/src/bytes.rs), whose rustdoc
describes its text, its JSON form and its use as a row field. `--fields`,
and `--long` without `--json`, are refused for it as a refused command line
that names `--json`. `--long` or `--full` with `--json` adds nothing and
[prints a note](#rows-printed-whole). A dry
run's line follows a bytes result's text on standard error, so standard
output holds the bytes alone.

Why: ADR-0016, ADR-0149.

## Headed bytes

A command that returns the bytes of several answers, such as `show
<NAME>...`, returns a batch whose row type implements
[`Headed`](../../../../crates/crumbkit/src/row.rs) and whose document
calls [`Document::headed`](../../../../crates/crumbkit/src/document.rs).
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
  than one key, with the key's control characters and format characters
  escaped. A format character is a Unicode character of general category
  Cf, such as a bidirectional override or a zero-width space. With one key
  the bytes print alone.
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
the text has no columns. `--long` is refused without `--json`, and `--long`
or `--full` with `--json` adds nothing and [prints a
note](#rows-printed-whole). The help
of the leaf ends with an `Output:` line that describes this text.

Why: ADR-0062, ADR-0114, ADR-0149.

## Rows printed whole

A row type whose JSON readers expect every field of each row sets
`Row::WHOLE` ([`Row`](../../../../crates/crumbkit/src/row.rs)). With
`--json`, each row then prints as the type serializes it, without `--full`,
so a field that the type leaves out when it holds nothing stays out. This
holds for a single row, a list, a listing, a batch and each line of a
stream.

- `--fields` selects fields in text and in JSON, and prints `null` for a
  field that a row leaves out.
- `--long` adds the long columns to the text table.
- `--long` or `--full` with `--json` prints the document that `--json`
  alone prints, exits as `--json` alone does, and writes one note to
  standard error: `note: --long adds nothing to --json here; the JSON
  already holds the whole result.`, or the same note for `--full`. With
  both flags, the note names `--full` once, because `--full` already takes
  in `--long`. The note follows the result and its cut line, and comes
  before a batch's or prose result's failures and before the dry-run
  line. A stream writes it after its last line, so after any failure it
  printed, also when the output budget stopped it or it ended in a
  failure. `finish` prints no note for a command that fails as a whole.
  Help still
  leaves `--full` out, because it adds nothing. The same holds for prose,
  bytes, headed bytes and a document without the envelope, whose JSON
  also holds the whole result.

A row type that sets `Row::WHOLE` may leave `Row::COLUMNS` empty, as a row
that serializes as an array must, only when it also declares `Row::LINE`.
Its text then prints each row's own line
([rows with their own text line](#rows-with-their-own-text-line)), and
`--fields`, and `--long` without `--json`, are refused, because there is
no table to shape.
`finish` and `stream` fail to compile for a whole row type without columns
and without a `LINE`, and crumbkit-test's `check_guide` fails a batch of
such a row type, which the compiler cannot check.

`check_guide` also fails a row type that sets `Row::WHOLE`, names no
columns and has fields in its JSON Schema, because those fields could be
the columns of a table. Its `check_document_ids` requires an
[opt-out](../../adoption/opt-outs.md) from `documents/rows-whole` for each
command whose row type sets `Row::WHOLE`, because the document costs more
tokens than the compact columns.

Why: ADR-0065, ADR-0064, ADR-0072, ADR-0138, ADR-0149.

## Rows with their own text line

A row type may print each row in its own form. `Row::LINE` names the form
for help, such as `Some("<NAME>: <ABOUT>")`, and `Row::text_line` builds
each row's line. Without `--json`, `--long` and `--fields`, the text then
prints each row's line with control characters and format characters
escaped and no header, and a row whose `text_line` returns `None` prints as
one line of compact JSON.
`--long` and `--fields` print the table, and JSON is unchanged. Streams,
the [output budget](../budget/README.md) and the listing footer work as for a
table, and help prints a `Text:` line ([help](../../help/README.md)). A row
type that also sets `Row::WHOLE` and names no columns offers neither flag,
so its `Text:` line names no table. crumbkit-test's `check_guide` fails a
`LINE` that is not one non-empty line, and a
[headed](#headed-bytes) leaf whose row type declares one.

Why: ADR-0064, ADR-0114.

## Rows with children

A row type may print the items of one field as indented lines under each
row. `Row::CHILDREN` names the field, such as `Some("components")`, and
`Row::child_lines` builds one line for each item, such as `+ database`.
Its default prints each item's text when the field holds strings or
[value types](../values/README.md), and prints nothing otherwise.
Without `--json` and `--fields`, the text prints each child line under the
row's table line, or under its own text line, with control characters and
format characters escaped. `Row::CHILD_INDENT` sets the number of spaces
before each child line, 2 by default and at most 255, the largest `u8`. A
type that sets 0 prints child lines at column 1, as table lines start, so a
reader tells them apart only by their content, which the doc comment of the
children field should describe. A fitted table
([the table's width](../terminal.md#the-tables-width)) never cuts a child
line. A row without child lines prints nothing more. `--long` prints the
lines of `Row::long_child_lines` in their place, which are the child lines
unless the type overrides it, and a stream prints them after each item's
line. The
[output budget](../budget/README.md) counts them with their indent.

A row type that sets `Row::SPACED` prints a blank line after a row that
printed child lines, when another row follows. The blank line prints under
the table, with `--long` and under rows with their own text line. A row
without child lines and the last row that prints are followed by none, so
the output never ends in a blank line. `--fields` and a stream print no
blank line, `--json` does not change, and the output budget counts each
blank line.

Compact JSON, with or without `--long`, holds the field after the columns,
and the compact schema requires it. `--fields` holds the field only when it
names it, and the text then prints it as a cell of compact JSON and prints
no child lines. Help prints a `Children:` line ([help](../../help/README.md))
with `Row::CHILDREN_HELP`, the row type's own description of its child
lines, or with crumbkit's sentence, which calls the lines indented only
when the indent is above 0. crumbkit-test's `check_guide` fails a field
that is not a documented field of the row type, a field that is also one of
the columns, a [headed](#headed-bytes) leaf whose row type sets `CHILDREN`,
and a `CHILDREN_HELP` that is not one line ending with a full stop or that
comes without `CHILDREN`.

Why: ADR-0120, ADR-0135, ADR-0137, ADR-0138, ADR-0151.

## Documents without the envelope

A tool chooses a document's envelope with
[`Document::envelope`](../../../../crates/crumbkit/src/document/envelope.rs),
which takes one `Envelope` variant. A new document has `Envelope::Crumbkit`.
A command that keeps the shape of an existing document chooses
`Envelope::Without`. With `--json` it prints the result alone with every
field:

- a row as an object;
- a list, listing or batch as an array;
- a prose result's value;
- a bytes result's `text` or `base64` object.

`--fields` selects fields as it does for any other document, and `--long`
or `--full` with `--json` adds nothing and [prints a
note](#rows-printed-whole). A listing prints one page
without `total`, and its text output keeps the footer and the next step.

The failures of a batch or prose result print as text on standard error even
with `--json`, and the command exits as any batch or prose result with
failures does. A command that fails as a whole still prints the failure document, so every command fails in one
shape.

crumbkit-test's `check_document_ids` requires an
[opt-out](../../adoption/opt-outs.md) for each command that prints a document
without the envelope.

Why: ADR-0016, ADR-0071, ADR-0081, ADR-0149.

## A command that prints no result

A command that prints no result on standard output, such as a server that
runs until the caller interrupts it, declares
[`Leaf::prints_no_result`](../../../../crates/crumbkit/src/help/leaf/no_result.rs)
in place of a document. Its rustdoc shows how the command ends. The leaf
offers none of `--fields`, `--long` and `--full`. `parse` refuses each of
them before the handler runs, with a message that says the command prints
no result and a hint that tells the caller to drop the flag, and the
leaf's help leaves them out. `--json` stays, so a failure prints as the
failure document. The leaf's `-h` and `--help` print the line `Output: no
result on standard output, and --json prints a failure as JSON.`

The command never succeeds. It prints its failure, such as the interrupt,
with `Output::print_failure`, takes its exit code from
`Output::failure_exit` and returns the code through `events::ended`. The
leaf has no document, so it publishes no schema and `commands list` gives
it `form: null`. crumbkit-test's `check_guide` accepts the leaf without an
opt-out from `guide/document`, and fails a leaf that declares both a
document and `Leaf::prints_no_result` ([help checks](../../help/checks.md)).

Why: ADR-0143.
