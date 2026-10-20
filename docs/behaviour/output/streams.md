# Streams

This page covers a command that prints rows as they arrive, such as a watch
or a command that reads a large file: how it declares its form, how it prints
in text and JSON, its failures and exit code, and its notes.

## Declaring a stream

A stream command declares its document as `Document<Stream<R>>`, with
[`Stream`](../../../crates/crumbkit/src/stream.rs), and that constant decides the command's form everywhere. The compiler
refuses a stream document passed to `finish` and a document of one result
passed to `stream`. The handler opens
the stream with [`stream`](../../../crates/crumbkit/src/output/stream.rs), or
`stream_to` in tests, and gets a `Streaming`, whose rustdoc describes
`item`, `failure`, `end` and `budget`. `stream` takes the `Output` by value,
and the `Streaming` owns its view and command tree until `end`, as `finish`
takes the `Output` (ADR-0094). Opening the stream checks the view
flags against the row type and the document before anything prints, and a
refusal exits as a refused command line does, as in `finish`.

A help leaf declares a stream with `Leaf::stream`. Its `-h` and `--help`
print the line `Output: one row per line as each arrives, and --json prints
one JSON document per line.` after the effects line. `commands list` and
`schemas list` show the form `stream`
([commands list](../help/commands-list.md),
[schema commands](../schemas/commands.md)).

crumbkit-test's `check_examples` stops a run that does not end within 10
seconds, so a stream command's example reads a finite input and never
follows one. For each `--json` example of a document with the envelope, the
check requires every line to be a JSON object whose `schema` is the leaf's
document id, and a leaf of one result to print exactly one line, or one
indented document under the [indented layout](json-layout.md).

Why: ADR-0041.

## Text and JSON lines

In text, the stream prints the table's header when it opens and flushes it.
Each column is as wide as its header, because the stream cannot know the
widths of rows still to come. Two spaces separate the cells, the last cell
is not padded, and a cell wider than its header pushes the rest of its line
to the right. `stream` fails to compile for a row type with an empty
`Row::COLUMNS`, because each line would print nothing of its row. A stream of
rows with [their own text line](shapes/README.md#rows-with-their-own-text-line)
prints those lines and no header. A row whose type names
[children](shapes/README.md#rows-with-children) prints its child lines after
its own line.

With `--json`, each row prints as a whole envelope document on its own line,
with `schema` and the row under `data`, so the output is JSON Lines and a
reader can process each line alone. `--fields`, `--long` and `--full` shape
each row as they shape a single row in `finish`. A stream prints one document per line in
every [JSON layout](json-layout.md). A document with
`Envelope::Without` prints each row alone, and its failures print as
text on standard error. A dry run marks each JSON line with
`"dry_run": true`. A stream document's JSON Schema, which `schemas show`
serves, describes one line: the envelope of a document of one row.

The stream flushes standard output after every line, so a reader sees each
row as soon as it prints. Standard output stays locked while the stream is
open.

Why: ADR-0041, ADR-0052, ADR-0063, ADR-0064, ADR-0120.

## Failures and the exit code

A failure in the middle prints on standard error in text, and as one
`<tool>/failure/v1` line between the rows with `--json`. A stream that printed a failure or ended with an `Err`
exits with the code of its first failure, as `finish` does
([exit codes](../failures/exit-codes.md)). A closed pipe keeps the code. Any
other failed write prints once on standard error and exits with code 1 unless
a failure chose the code. A stream without a failure whose printed items
report a problem exits with 1 and, in text, ends with the note that counts
them ([rows that report a problem](listings/README.md#rows-that-report-a-problem)).

A stream prints at most the [output budget](budget/README.md#streams).

Why: ADR-0041.

## Notes

A stream's notes print on standard error after the stream has flushed
standard output, in text and with `--json`. The empty note comes first, then
the dry-run line in text or for a document without the envelope.

A stream that ends with `Ok`, printed no rows and no failure, and still has
an open pipe prints the empty note on standard error, in text and with
`--json`. Its default wording is `no rows`, and `Document::when_empty` sets
its own.

`stream` records its run in the [usage log](../usage-log/README.md) and on
the [`crumbkit.finish` span](../usage-log/trace-context.md) as `finish` does.

Why: ADR-0041, ADR-0050.
