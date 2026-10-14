# Output

This page covers how a tool prints a command's result: the two formats, the
text table, the view flags, which stream each line goes to, and the shapes of
result that [`finish`](../../../crates/crumbkit/src/output.rs) prints. The
other pages in this directory cover [listings](listings/README.md),
[streams](streams.md), the [output budget](budget/README.md),
[child processes](child-processes.md), [terminal detection](terminal.md) and
the [JSON layout](json-layout.md).

## Formats

Results come in two formats: compact text for people and agents, and JSON for
programs. The text table has a header row, one record per line and no
repeated keys. No command prints Markdown. `cargo crumbkit docs` writes a
Markdown reference of a tool's commands to a file, and `commands list`
exports a branch of the command tree in the two formats
([commands list](../help/commands-list.md), [reference](../help/reference.md)).

Why: ADR-0016.

## The text table

A type that prints as a table row implements
[`Row`](../../../crates/crumbkit-core/src/row.rs), which names its compact
columns, the columns that `--long` adds and, optionally, the field that marks
a problem ([rows that report a problem](listings/README.md#rows-that-report-a-problem)).
A field in neither list of columns appears only with `--json --full`.

The table pads columns by characters and escapes control characters, so each
row stays on one line and no line ends in padding. Cell widths count
characters, so a wide character misaligns a column. A cell shows `null` or a
missing field as `-`, and an array or an object as compact JSON.

Why: ADR-0016.

## View flags

The view flags are `--fields`, `--long` and `--full`, and like every
[`Output`](../../../crates/crumbkit/src/output.rs) flag they are global. A
leaf with a document offers only the view flags that its document can use:

- rows offer `--fields`, offer `--long` when the row type has long columns,
  and offer `--full` when the document has the envelope;
- prose and bytes offer none of the three.

A leaf without a document offers all three, because crumbkit cannot tell
what its command prints. `parse` leaves an unoffered view flag out of the
leaf's help and refuses it as a refused command line before the handler
runs, and `finish` and `stream` refuse it by the same rule. An unknown
`--fields` name is refused before anything prints.

`--fields` cannot be used with `--long` or `--full`, and `--full` needs
`--json`. crumbkit checks these pairs itself in `parse`, `finish` and
`stream`, on every command, and refuses them in its own words with a hint
that names the flag to add or drop. A tool that parses with clap alone gets
these refusals, and the refusal of an unoffered view flag, from `finish` or
`stream` after the handler has run.

On a leaf without a document, only `finish` or `stream` refuses a view flag
that the output cannot use, and that happens after the handler has run. On a
command that changes something, an agent may then repeat a write that
already happened. crumbkit-test's `check_guide` therefore fails a leaf whose
effects change something and that declares no document, and it takes no
opt-out from that rule ([help checks](../help/checks.md)). A leaf that passes
on a program's exit code needs no document, because `child::hand_over`
refuses `--json` and `--fields` before the program starts
([child processes](child-processes.md)).

Why: ADR-0016, ADR-0024.

## Following the caller's format

A tool that prints more than one result through `finish`, such as a child
process's output, reads `Output::is_json` to follow the caller's format. The
answer exists on every path, because `parse` returns a `Stop` whose `Output`
holds the `--json` that the walk found.

A tool draws progress with a library of its choice, and only when
`Output::progress_allowed` is true: `--json` is off, the guide keeps
[terminal detection](terminal.md) on and standard error is a terminal. A
program, an agent, a pipe, a file and crumbkit-test's `check_examples`
therefore never receive progress lines. crumbkit takes no progress library
as a dependency.

Why: ADR-0016, ADR-0049.

## Standard output and standard error

In text, the lines that describe a result print on standard error, and
standard output holds the result alone. These lines are a listing's footer,
its empty note, the note of the rows that report a problem, the tool's note,
the `Next:` line, the output budget's [cut line](budget/README.md#listings-and-lists),
the [dry-run line](../effects/README.md) and a [stream's notes](streams.md#notes).
A pipe such as `| wc -l`, `| grep` or `| jq` therefore sees only data. A
listing's summary line stays on standard output above the table, because it
is part of the result.

crumbkit flushes standard output before it writes the notes, and it writes
all of a result's notes to standard error in one write
([`write_result`](../../../crates/crumbkit/src/output/result.rs)). A reader
that merges the two streams, such as a terminal or `2>&1`, therefore sees the
notes after the rows, in one piece.

With `--json`, the document holds what the notes say, so no note prints. The
exceptions are the lines that a document without the envelope has no field
for, such as the cut line and the dry-run line, which print on standard
error.

A failure prints as text on standard error with nothing on standard output,
or with `--json` as one failure document on standard output. In text, each
alternative repair prints on its own `help:` line after the recovery's. A
tool with a failure form prints each failure and refusal in that form
([failure forms](../failures/forms.md)).

`finish` writes through a [`Sink`](../../../crates/crumbkit/src/sink.rs), and
`finish_to` writes into a given sink, so tests run in memory. A pipe that the
reader closed early keeps the exit code of the result.

Why: ADR-0050, ADR-0016.

## Batches

A command that takes many ids returns a
[`Batch`](../../../crates/crumbkit-core/src/batch.rs), whose rustdoc
describes how a handler builds it. The rows print on standard output and the
failures on standard error, and with `--json` both go into one document.
Every answer has a key, and a key that fails gets its own failure. The batch
keeps each key as the text of its `Display`, which is the word a caller
types, because the [over-budget failure](budget/README.md#batches) puts the keys
into a command line. A key may repeat, and nothing in
crumbkit assumes that keys are unique. Each failure of a batch holds its key
and the key's index among the keys, counted from 0, so a reader matches a
failure to its key even when a key repeats. `key` and `index` sit before
`code` in the failure's JSON, and a `key:` line such as `key:   7 (index 1)`
follows the `error:` line in its text. They are members of the failure and
sit outside `fields`. A failure pushed after the last key has neither. A
batch with at least one failure exits with the code of its first failure,
which is 1 unless the guide declares another for that failure's code
([exit codes](../failures/exit-codes.md)).

Why: ADR-0016.

## Prose

A command whose answer is a few sentences returns a
[`Prose`](../../../crates/crumbkit-core/src/prose.rs), whose rustdoc
describes its text, its JSON document and its failures. A prose result with
failures exits as a batch with failures does, with the code of its first
failure. `--fields`, `--long`
and `--full` are refused for a prose result as a refused command line. The
hint of `--fields` and `--long` names `--json`, and the hint of `--full` says
that `--json` already prints the whole result.

Why: ADR-0016.

## Bytes

A command whose answer is bytes, such as a file's contents, returns
[`Bytes`](../../../crates/crumbkit-core/src/bytes.rs), whose rustdoc
describes its text, its JSON form and its use as a row field. `--fields`, `--long`
and `--full` are refused for it as a refused command line, and each refusal
names `--json` or says that `--json` already prints the whole result. A dry
run's line follows a bytes result's text on standard error, so standard
output holds the bytes alone.

Why: ADR-0016.

## Documents without the envelope

A command that keeps the shape of an existing document prints a document
from [`Document::without_envelope`](../../../crates/crumbkit-core/src/document.rs).
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
expect rows alone, and a const panic rejects it on a document with the
envelope, which holds its failures under `failures`. A reader tells a failure
from a row by its `schema`, so crumbkit-test's `check_guide` fails such a
document whose row type has a field named `schema`. A command that
fails as a whole still prints the failure document, so every command fails in
one shape.

crumbkit-test's `check_document_ids` requires an
[opt-out](../adoption/opt-outs.md) for each command that prints a document
without the envelope, and the opt-out report lists `failures_in_order` as a
[declaration](../adoption/declarations.md).

Why: ADR-0016.
