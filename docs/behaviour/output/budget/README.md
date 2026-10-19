# Output budget

This page covers the output budget: the most bytes that `finish` and `stream`
print on standard output in one run, how a tool and a caller change it, and
what each shape of result prints when it does not fit.

## The budget

The budget counts the bytes of the result as they print, indented when the
guide sets `JsonLayout::Pretty` ([JSON layout](../json-layout.md)). It counts the
result alone, because the lines that describe it, such as the footer and the
cut line, print on standard error. The budget is 1 MiB (1048576 bytes) unless
the tool sets another.

A tool sets its budget with
[`Guide::output_budget(bytes, reason)`](../../../../crates/crumbkit/src/help/output.rs).
The budget can be any number of bytes, as small as the one that
`<TOOL>_OUTPUT_BUDGET` lowers a run to, so a tool's tests can show a cut
result with a few short rows. A result that the budget refuses prints its
over-budget failure whatever the failure's size, and a document marked
with `Document::always_print_first` prints its first answer. The reason says why the tool's results need the bytes.
crumbkit-test's `check_opt_outs` pins the budget, its reason and its cost in
the [opt-out report](../../adoption/opt-outs.md) as `output-budget/run`. The
cost line says how many bytes, and about how many tokens, a run can print
beyond crumbkit's default. No opt-out removes the
budget. A tool whose results are larger raises its budget and gives the
reason.

The environment variable `<TOOL>_OUTPUT_BUDGET` lowers the budget of one
run to a whole number of bytes. A value larger than the tool's budget, 0, or
a value that is not a whole number leaves the tool's budget unchanged. The
variable never raises the budget, so no one can read more than the tool's
budget in one run. A reader who needs more asks for less with `--limit`,
`--fields` or a filter and runs the command more than once.

`Output::budget` returns the budget, so a handler can fetch no more rows than
fit before it passes the `Output` to `finish`.

Why: ADR-0046, ADR-0081, ADR-0084.

## Measuring a result

`finish` measures a result before it prints anything. It serializes the
result once into a buffer that stops at the budget, and it prints the buffer
when the result fits. A result that fits costs one serialization and one
buffer. When it does not fit, the result's shape decides what happens, as
the sections below describe.

The cut line and the over-budget message name a budget of 1 as `1 byte` and
any other budget in bytes, such as `400 bytes`.

The cut line, the `cut` field and the over-budget failure depend only on the
result and the budget. None of them depends on whether standard output or
standard error is a terminal (OUTPUT-1).

Why: ADR-0046.

## Listings and lists

A listing or a list prints the most rows that fit, found by a binary search
over the row count, and the run exits with code 0. The search measures each
count of rows with a counter that keeps no bytes. When the rows print
[their own text line](../shapes/README.md#rows-with-their-own-text-line), the search
counts the bytes of those lines, which `finish` builds once.

- In text, a line such as `Showing 812 of 5000 books, because output stops at
  1048576 bytes. Ask for less with --limit or --fields.` prints on standard
  error in place of the footer. A list, which has no paging flags, names only
  `--fields`.
- With `--json`, the document holds
  `"cut": {"code": "shelf::output::over_budget", "budget": 1048576}`.
- A document with `Envelope::Without` has no place for `cut`, so the
  line also prints on standard error with `--json`.
- A listing that holds `cut` also holds `"truncated": true`, and its `total`
  still counts every row that matched when the tool counted them.

Why: ADR-0046, ADR-0064.

## Batches

A batch prints the rows that fit in input order, then ends with one
failure, `<tool>::output::over_budget`. The run exits with code 1. In
text, the batch's note prints on standard error before its failures, and
the note of the rows that report a problem does not print, because the
failure decides the exit. The failure has these fields:

- `left_out` lists the keys of the answers it left out as text, in the
  caller's order. A key that the caller gave twice is listed twice. A key
  whose lookup failed is never listed, because its own failure reports it.
- `left_out_from` gives the index of the first answer left out among the
  answers in the caller's order, counted from 0. Every later answer without
  its own failure was left out too, so a reader whose keys repeat matches
  `left_out` to its keys by position.
- `unlisted` counts the answers left out that `left_out` does not name. It is
  present only when it is above 0.

A batch's first answer prints whole at any budget when its document is
marked with `Document::always_print_first`. Without the mark, a first answer
that passes the budget alone is refused. It passes the budget alone when the
bytes that the run prints for that one row are more than the budget: in
text, the table or the line of that row, or its header and bytes in a
headed batch, and with `--json`, the document that holds that row and no
failures. A refused first answer gets its own over-budget failure under its
key, among the batch's own failures in the order of the keys. The failure
of a table row asks for fewer fields with `--fields`, because a row with
fewer fields is smaller. The failure of a headed answer has no next step,
because a headed answer prints whole whatever the fields. No row prints,
and every later answer goes into the one failure above. A one-key command that shows a file therefore refuses a file larger
than the budget, as a single bytes result does.

A batch's handler can stop fetching once its rows fill the budget.
`Output::room(document)` returns a `Room` before the first fetch, and
`Room::has_room(&batch)` measures each row pushed since the last call once,
in the format that the run prints: the JSON document in the guide's layout,
the text table with each column as wide as its widest cell, or the headers
and bytes of a headed batch. `has_room` returns false once the rows take the
bytes that the run can print, and returns true until the batch holds a row,
because `finish` decides on the first row by that row's own bytes. The count leaves out the batch's failures,
which only add bytes, so a handler that stops when `has_room` returns false
never stops before the row at which `finish` cuts the batch. The keys that
the handler never fetched follow the keys of the cut rows in `left_out`, and
a batch with such keys ends with the failure even when its rows fit, so
every key is either answered or listed.

The failure's recovery is a `run` step, such as
`{"kind":"run","call":"shelf books show <ID>... --json","all":"left_out"}`,
which runs the command once with every key of `left_out`, each as its own
word in place of `<ID>...`. The call keeps every other word the caller typed
in its place, including `--json`, `--fields` and global options before the
subcommand, and puts the placeholder where the caller's first key was. A
key argument that is an option keeps its flag in front of the placeholder. The call text shows `<ID>...`, clap's usage form for an
argument that takes many values, so a reader that ignores `all` still sees
that several keys go in that place, and `check_recoveries` reads it as one
placeholder. In text the `help:` line prints the concrete command,
such as ``run `shelf books show 4 5 6` ``, with each key quoted for a shell
as other call arguments are. The run with every key may itself be cut, and
each run answers at least one key, so the next failure lists fewer keys.

crumbkit finds the keys among the caller's words from clap's parse of those
words. The recovery falls back to the batch's command with only the keys,
such as `shelf books show <ID>...`, when `Output` holds no words, when the
words run another command than the batch, when the keys are not among the
words (such as ids read from a file), or when the words with the keys
replaced parse with any other argument changed. The fallback also applies to
keys typed in a cluster of short flags and to keys split from one word by a
value delimiter.

When `unlisted` is above 0, the message adds that the recovery covers the
keys listed in `left_out` and that a later run covers the rest. When
`left_out` lists no key, because every answer left out has no key, the
recovery is a hint to run the command again with
the keys that were not answered. The message always counts every answer left
out.

In a JSON document with the envelope, the failure prints on standard output
and counts against the budget. The rows that print are the most that fit
beside a failure that lists every key left out after the first row. When even
one row and that full list do not fit, one row prints, `left_out` lists the
most first keys that fit, and `unlisted` counts the rest and any cut answer
that has no key. When one row and a failure that lists no key already pass
the budget, the document passes it whatever the failure lists, so `left_out`
lists every key left out and the recovery runs the command with them. In text, and in any other document without the envelope, the failure
prints on standard error, which the budget does not count, so it lists every
key.

Why: ADR-0046, ADR-0068, ADR-0070, ADR-0077, ADR-0084.

## Headed bytes

The text of a [headed batch](../shapes/README.md#headed-bytes) prints each row whole
or not at all, because a file cut short would look whole. Each row costs its
header and its bytes, and 1 more byte for the line feed before its header
when it is not the first row that prints.

- The first row prints when its header and bytes fit in the budget. When
  they pass it, the first row prints anyway if the document is marked with
  `Document::always_print_first`, and is refused with its own failure
  otherwise, as the first row of every batch is.
- The later rows print in order while they fit. From the first later row
  that does not fit, alone or beside the rows before it, that row, every
  later row and every key that the handler never fetched are left out, and
  one over-budget failure names them, as the failure of a batch in text
  does, with the recovery that runs the command with every key left out.
  That run starts with the first of those rows as its first answer.

The batch's own failures print on standard error in the order of their
keys, and the failure that names the rows left out prints last. The batch's
note, when it has one, prints before these failures. The run exits with the
code of the first failure. With `--json` a headed batch is budgeted by the
same rule as any other batch. The JSON form is larger, because each row
carries its field names and the failure counts against the budget, so at
the same budget the JSON form can refuse a first answer that the text
prints, or leave out answers that the text prints.

Why: ADR-0062, ADR-0084.

## Single results

A single row, or a listing or list whose first row alone does not fit, is
refused with `<tool>::output::over_budget`, whose recovery is a hint to ask
for fewer fields with `--fields`. A prose result is refused with the same
code and no recovery. A bytes result is refused in the same way and is never
cut. Its text form is measured as the raw bytes and its JSON form as the
document with the base64 text, so `--json` can refuse a result whose text
fits. Each refusal exits with code 1.

A document marked with `Document::always_print_first` always prints its first
answer: a single row, a prose result or a bytes result prints whole even past
the budget, a listing or a list prints at least its first row, and a batch
prints its first answer, as [Batches](#batches) describes. A tool
uses it when the one answer is the point of the run, such as a file that the
caller named.

Why: ADR-0046, ADR-0084.

## Streams

A stream counts the bytes of each line. The first item always prints. A later
item whose line would pass the budget does not print, the stream prints the
over-budget failure, `Streaming::item` returns `Break`, and `Streaming::end`
exits with code 1 when that failure is the stream's first and the guide
declares no code for it. The failure line that ends the stream prints after
the budget is reached, so the stream's output can pass the budget by that one
line. `Streaming::budget` returns the bytes left, so a tool can stop
producing items early. A tool whose stream runs for a long time, such as a
watch, raises its budget.

Why: ADR-0046, ADR-0041.

## The over-budget code

A tool sets its own over-budget code with a `Rename` of
`<tool>::output::over_budget` in `Guide::codes`, which `cut` and the
failure report in place of crumbkit's for every document. A `Rename` in
`Leaf::codes` sets the code for the runs of one command, and
`check_output_budget` expects the exit that the guide gives that command's
code. An `Exit` that
lists the code gives the failure its own exit code in place of 1
([exit codes](../../failures/exit-codes.md)). The failure keeps crumbkit's
message and recovery.

The code is in `codes::ALL`, the list of crumbkit's own codes. The
[usage log](../../usage-log/README.md) records a cut result as
`truncated: true` with the over-budget code, and records a refusal as a
failure with the code. The
[`crumbkit.finish` span](../../usage-log/trace-context.md) records the code as
it records a failure's code.

Why: ADR-0046, ADR-0060.

## The budget check

crumbkit-test's `check_output_budget` runs each `--json` example of the guide
once, then again with `<TOOL>_OUTPUT_BUDGET` set to half the bytes it
printed. The second run must do one of these:

- print a result marked with `cut` or the cut line;
- exit with code 0 and print one answer past the budget from a document that
  always prints its first answer, as one object or as a batch of one row;
- exit with the code that the guide gives the over-budget failure, 1 by
  default, and print an over-budget failure that names the budget last.

The check also runs each text example of a leaf whose document is headed in
the same way. The second run must either exit with the over-budget
failure's code with a message on standard error that names the budget, or
exit with code 0 and print one answer past the budget from a document that
always prints its first answer. A run that exits with code 0 and prints two
or more answers past the budget breaks the contract. A run whose document
does not always print its first answer must print within the budget,
whatever its exit code. The check counts each line of the form
`==> … <==` as one answer's header, because it does not know which words
of the example are keys.

Why: ADR-0046, ADR-0062, ADR-0084.
