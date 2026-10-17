# Output

This page covers how a tool prints a command's result: the two formats, the
text table, the view flags and which stream each line goes to. The
other pages in this directory cover the [result shapes](shapes/README.md)
that [`finish`](../../../crates/crumbkit/src/output.rs) prints,
[listings](listings/README.md),
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

The compact columns name at least one field, because the table and the
JSON without `--full` print only the columns. `finish` fails to compile for
a single row, a `Vec` or a `Listing` whose row type has an empty
`Row::COLUMNS`, and `stream` fails to compile for such a row type, so the
rows of such a result cannot vanish while `total` still counts them. A
batch compiles, because a [headed batch](shapes/README.md#headed-bytes) prints each row's
bytes in text and every field in JSON and may name no column.
crumbkit-test's `check_guide` fails a leaf whose batch is not headed and
whose row type has no compact column. A row type that sets `Row::WHOLE` and
declares `Row::LINE` may also name no column, as [rows printed whole](shapes/README.md#rows-printed-whole) describes.

The table pads columns by characters and escapes control characters, so each
row stays on one line and no line ends in padding. Cell widths count
characters, so a wide character misaligns a column. A cell shows `null` or a
missing field as `-`, and an array or an object as compact JSON.

Why: ADR-0016, ADR-0063, ADR-0065.

## View flags

The view flags are `--fields`, `--long` and `--full`, and like every
[`Output`](../../../crates/crumbkit/src/output.rs) flag they are global. A
leaf with a document offers only the view flags that its document can use:

- rows offer `--fields`, offer `--long` when the row type has long columns,
  and offer `--full` when the document has the envelope;
- a headed batch offers `--fields`, which `parse` refuses without `--json`,
  and offers neither `--long` nor `--full`;
- rows whose type sets `Row::WHOLE` offer no `--full`, and offer neither
  `--fields` nor `--long` when the row type has no columns;
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

A tool that defines the view flags in its own parser cannot also flatten
`ViewFlags`, because clap refuses two arguments with one name. It builds
`ViewFlags` with `ViewFlags::new().json(..).fields(..)`, `.long(..)` and
`.full(..)`, passes them to `Output::new`, and `finish` and `stream`
check the built flags as they check parsed ones. `parse` reads a view flag
by its argument id, so the tool's own arguments get the refusals before
the handler runs, and an unoffered flag leaves the leaf's help, only when
their ids are `json`, `fields`, `long` and `full` and they are global.

On a leaf without a document, only `finish` or `stream` refuses a view flag
that the output cannot use, and that happens after the handler has run. On a
command that changes something, an agent may then repeat a write that
already happened. crumbkit-test's `check_guide` therefore fails a leaf whose
effects change something and that declares no document, and it takes no
opt-out from that rule ([help checks](../help/checks.md)). A leaf that passes
on a program's exit code needs no document, because crumbkit-child's `hand_over`
refuses `--json` and `--fields` before the program starts
([child processes](child-processes.md)).

Why: ADR-0016, ADR-0024, ADR-0065.

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
`finish_to` writes into a given sink, so tests run in memory. `finish_to`
returns the exit code as a `u8`, which a test can compare and a tool can
record, and `finish` returns the same code as an `ExitCode`. A pipe that the
reader closed early keeps the exit code of the result.

Why: ADR-0050, ADR-0016.

## Result shapes

The [result shapes](shapes/README.md) page covers batches, prose, bytes,
headed bytes, rows printed whole, rows with their own text line, documents
without the envelope, and documents with the tool's own keys, including
listings without a total.
