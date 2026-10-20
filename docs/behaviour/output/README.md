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
[`Row`](../../../crates/crumbkit/src/row.rs), which names its compact
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

The table pads columns by characters and escapes control characters and
format characters, so each row stays on one line and no line ends in
padding. A format character is a Unicode character of general category Cf,
such as a bidirectional override or a zero-width space. The table prints it
as its escape, such as `\u{202e}`, so a cell cannot reorder or hide the text
around it. Cell widths count characters, so a wide character misaligns a
column. A cell shows `null` or a missing field as `-`, and an array or an
object as compact JSON. A field of type
[`Age`](../../../crates/crumbkit/src/age.rs) shows how long ago its time
was, such as `3h`, in the table and in a stream's text lines, and holds the
time in RFC 3339 in every JSON form, as its rustdoc describes.

The table that `finish` prints leaves out a column whose field is `null` or
missing in every row that the table shows, header included. The shown rows
are the rows the table writes, which can be fewer than the result holds when
the output budget cuts the table. crumbkit reads each field's value, so a
string cell that holds `-` keeps its column. The problem field that
`Row::PROBLEM` names is a column like any other, so the table leaves it out
when no shown row reports a problem and the field is null in each. The rule
applies to the compact columns and to the columns of `--long`. With
`--fields`, every named column prints, because the caller named it, and
`--json` holds every selected field in every view. A table that shows no
rows keeps every column in its header. A [stream](streams.md#text-and-json-lines)
prints every column, because its header prints before any row arrives.

At a terminal, a table wider than the terminal narrows its widest columns
to fit and ends each cut cell with `…`, except in the columns that
`Row::UNCUT_COLUMNS` names ([the table's width](terminal.md#the-tables-width)).

Why: ADR-0016, ADR-0063, ADR-0065, ADR-0108, ADR-0114, ADR-0119, ADR-0121.

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

The document's schema matches the JSON of `--json`, `--json --long` and
`--json --full`, and matches a `--fields` selection only when the selection
names every column in `Row::COLUMNS` ([schemas](../schemas/README.md#versions)).

A leaf without a document offers all three, because crumbkit cannot tell
what its command prints. `--fields` cannot be used with `--long` or
`--full`, and `--full` needs `--json`, on every command. crumbkit refuses
each of these as a refused command line, in its own words, with a hint
that names the flag to add or drop or the valid fields. The four flags are
parsed once into one view, a text table or JSON rows with a selection of
fields, and the `Output` holds that view, so it never holds two flags that
cannot work together. Three checks apply the rules:

- `parse` refuses, before the handler runs, a view flag that the leaf's
  document does not offer, two flags that cannot work together, and a
  `--fields` name that the row type of the leaf's document lacks, in that
  order. It also leaves an unoffered view flag out of the leaf's help. On
  a leaf without a document, `parse` refuses only the pairs.
- `Output::new` refuses the pairs for a tool that parses with clap alone
  or builds its own `ViewFlags`, before the handler runs. It prints the
  refusal and returns the `Stop` of a refused command line, as `parse`
  does.
- When the handler returns a result, `finish` refuses by the document it
  was given: an unoffered view flag and an unknown `--fields` name.
  `stream` refuses by its document in the same way when it opens. When
  the handler returns a failure, `finish` checks nothing and prints the
  failure with its own code, as text or as a failure document with
  `--json`. A tool may finish an early failure, such as a missing config
  file, in another document than the one its leaf declares, so the
  document that `finish` was given says nothing about the flags.

A tool that parses with clap alone, a tool that builds its own
`ViewFlags`, and a leaf without a document get no check of the document
from `parse`. For them, `finish` or `stream` refuses an unoffered view
flag and an unknown `--fields` name after the handler has run, and a
failed run prints its failure even when a view flag or a `--fields` name
would not suit the command's result.

A tool that defines the view flags in its own parser cannot also flatten
`ViewFlags`, because clap refuses two arguments with one name. It builds
`ViewFlags` with `ViewFlags::new().json(..).fields(..)`, `.long(..)` and
`.full(..)`, passes them to `Output::new`, which refuses the pairs, and
`finish` and `stream` check the built flags against the result as they
check parsed ones. `parse` reads a view flag
by its argument id, so the tool's own arguments get the refusals before
the handler runs, and an unoffered flag leaves the leaf's help, only when
their ids are `json`, `fields`, `long` and `full` and they are global.

On a command that changes something and whose leaf declares no document,
an agent may repeat a write that already happened, because the refusal
comes after the handler has run. crumbkit-test's `check_guide` therefore
fails a leaf whose effects change something, whose command takes a view
flag and that declares no document ([help checks](../help/checks.md)).
A command takes a view flag when it reaches a global argument whose id is
`json`, `fields`, `long` or `full`, as `parse` reads one. A leaf whose
command takes no view flag needs no document, because no caller can type
a view flag that crumbkit would refuse after the change. A tool that
prints its own output for a leaf that the rule covers opts out of
`guide/document` until the output moves onto crumbkit's documents, and
the tool's own output then decides what a view flag does
([opt-outs](../adoption/opt-outs.md)). A leaf that passes
on a program's exit code needs no document, because crumbkit-child's
`hand_over` refuses `--json` and `--fields` before the program starts
([child processes](child-processes.md)).

Why: ADR-0016, ADR-0024, ADR-0065, ADR-0082, ADR-0093, ADR-0105, ADR-0110, ADR-0115.

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
its empty note, the note of the rows that report a problem, the tool's note
of a listing, a batch or a prose result, the `Next:` line, the output budget's [cut line](budget/README.md#listings-and-lists),
the [dry-run line](../effects/README.md) and a [stream's notes](streams.md#notes).
A pipe such as `| wc -l`, `| grep` or `| jq` therefore sees only data. A
listing's summary line stays on standard output above the table, because it
is part of the result.

crumbkit flushes standard output before it writes the notes, and it writes
all of a result's notes to standard error in one write
([`write_result`](../../../crates/crumbkit/src/output/result.rs)). A reader
that merges the two streams, such as a terminal or `2>&1`, therefore sees the
notes after the rows, in one piece. The failures of a batch or a prose result
print after that write, and the dry-run line prints last.

With `--json`, the document holds what the notes say, so no note prints. The
exceptions are the lines that a document without the envelope has no field
for, such as the cut line and the dry-run line, which print on standard
error.

A failure prints as text on standard error with nothing on standard output,
or with `--json` as one failure document on standard output. In text, each
alternative repair prints on its own `help:` line after the recovery's, and
each control character and format character in a failure's text prints as
its escape, as the table prints it.

`finish` writes through a [`Sink`](../../../crates/crumbkit/src/sink.rs), and
`finish_to` writes into a given sink, so tests run in memory. `finish_to`
returns the exit code as a `u8`, which a test can compare and a tool can
record, and `finish` returns the same code as an `ExitCode`. A pipe that the
reader closed early keeps the exit code of the result.

`finish`, `finish_to`, `stream` and `stream_to` take the `Output` by value,
because a run prints one result and nothing reads the `Output` after. A
handler that reads the `Output` with `&mut`, such as `commands::list` or a
command that confirms, runs before `finish`. A second `finish` with the same
`Output` does not compile.

Why: ADR-0050, ADR-0016, ADR-0077, ADR-0094.

## Result shapes

The [result shapes](shapes/README.md) page covers batches, prose, bytes,
headed bytes, rows printed whole, rows with their own text line and
documents without the envelope. The listings page covers
[a listing without a total](listings/README.md#a-listing-without-a-total).
