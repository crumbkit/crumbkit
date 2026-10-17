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
The budget is at least 1024 bytes, and a smaller one panics when the guide is
built. The reason says why the tool's results need the bytes.
crumbkit-test's `check_opt_outs` pins the budget, its reason and its cost in
the [opt-out report](../../adoption/opt-outs.md) as `output-budget/run`. The
cost line says how many bytes, and about how many tokens, a run can print
beyond crumbkit's default. No opt-out removes the
budget. A tool whose results are larger raises its budget and gives the
reason.

The environment variable `CRUMBKIT_OUTPUT_BUDGET` lowers the budget of one
run to a whole number of bytes. A value larger than the tool's budget, 0, or
a value that is not a whole number leaves the tool's budget unchanged. The
variable never raises the budget, so no one can read more than the tool's
budget in one run. A reader who needs more asks for less with `--limit`,
`--fields` or a filter and runs the command more than once.

`Output::budget` returns the bytes that the run can still print, so a handler
can fetch no more rows than fit.

Why: ADR-0046.

## Measuring a result

`finish` measures a result before it prints anything. It serializes the
result once into a buffer that stops at the budget, and it prints the buffer
when the result fits. A result that fits costs one serialization and one
buffer. When it does not fit, the result's shape decides what happens, as
the sections below describe.

The cut line, the `cut` field and the over-budget failure depend only on the
result and the budget. None of them depends on whether standard output or
standard error is a terminal (OUTPUT-1).

Why: ADR-0046.

## Listings and lists

A listing or a list prints the most rows that fit, found by a binary search
over the row count, and the run exits with code 0. The search measures each
count of rows with a counter that keeps no bytes.

- In text, a line such as `Showing 812 of 5000 books, because output stops at
  1048576 bytes. Ask for less with --limit or --fields.` prints on standard
  error in place of the footer. A list, which has no paging flags, names only
  `--fields`.
- With `--json`, the document holds
  `"cut": {"code": "crumbkit::output::over_budget", "budget": 1048576}`.
- A document from `Document::without_envelope` has no place for `cut`, so the
  line also prints on standard error with `--json`.
- A listing that holds `cut` also holds `"truncated": true`, and its `total`
  still counts every row that matched when the tool counted them.

Why: ADR-0046.

## Batches

A batch prints the rows that fit in input order and always its first row,
then ends with one failure, `crumbkit::output::over_budget`. The run exits
with code 1. The failure has these fields:

- `left_out` lists the keys of the answers it left out as text, in the
  caller's order. A key that the caller gave twice is listed twice. A key
  whose lookup failed is never listed, because its own failure reports it.
- `left_out_from` gives the index of the first answer left out among the
  answers in the caller's order, counted from 0. Every later answer without
  its own failure was left out too, so a reader whose keys repeat matches
  `left_out` to its keys by position.
- `unlisted` counts the answers left out that `left_out` does not name. It is
  present only when it is above 0.

A batch's handler can stop fetching once its rows fill the budget.
`Output::room(document)` returns a `Room` before the first fetch, and
`Room::has_room(&batch)` measures each row pushed since the last call once,
in the format that the run prints: the JSON document in the guide's layout,
the text table with each column as wide as its widest cell, or the headers
and bytes of a headed batch. `has_room` returns false once the rows take the
bytes that the run can print. The count leaves out the batch's failures,
which only add bytes, so a handler that stops when `has_room` returns false
never stops before the row at which `finish` cuts the batch. The keys that
the handler never fetched follow the keys of the cut rows in `left_out`, and
a batch with such keys ends with the failure even when its rows fit, so
every key is either answered or listed.

The failure's recovery is a `run` step built with `Call::all`, such as
`{"kind":"run","call":"shelf books show <ID>...","all":"left_out"}`, which
runs the command once with every key of `left_out`, each as its own word in
place of `<ID>...`. The call text shows `<ID>...`, clap's usage form for an
argument that takes many values, so a reader that ignores `all` still sees
that several keys go in that place, and `check_recoveries` reads it as one
placeholder. In text the `help:` line prints the concrete command,
such as ``run `shelf books show 4 5 6` ``, with each key quoted for a shell
as other call arguments are. The run with every key may itself be cut, and
each run answers at least one key, so the next failure lists fewer keys.

When `unlisted` is above 0, the message adds that the recovery covers the
keys listed in `left_out` and that a later run covers the rest. When
`left_out` lists no key, the recovery is a hint to run the command again with
the keys that were not answered. The message always counts every answer left
out.

In a JSON document with the envelope, the failure prints on standard output
and counts against the budget. The rows that print are the most that fit
beside a failure that lists every key left out after the first row. When even
one row and that full list do not fit, one row prints, `left_out` lists the
most first keys that fit, and `unlisted` counts the rest and any cut answer
that has no key. A batch that prints its failures in input order ends its
array with the failure, at the top of an unwrapped document or under the data
key of a document from `Document::with_keys`, and the failure counts against
the budget in the same way. In text, and in any other document without the envelope, the failure
prints on standard error, which the budget does not count, so it lists every
key.

Why: ADR-0046, ADR-0059.

### One failure for each answer left out

A batch whose document is marked with `Document::left_out_per_key` answers
each answer left out with its own `crumbkit::output::over_budget` failure,
in place of the one failure that names them all. Each failure has the key and
index of its answer, the message `the answer did not fit in the output budget
of 1048576 bytes`, the field `budget`, and a `run` recovery that runs the
command with that key alone, such as `shelf books show 4`. An answer without
a key gets a hint to run the command again with the keys that were not
answered. The failures print among the batch's other failures in the order of
their keys, and the run exits with the code of the first failure in key
order.

In text, and in a document without the envelope that does not print its
failures in input order, the rows print as a batch's rows do and the
failures print on standard error. In a JSON document that holds its
failures, each failure counts against the budget, and a row that prints
takes the place of its failure. A row can be shorter than its failure, so
`finish` measures each row and each failure once and prints the most rows
that fit with the failures of every answer after them. When even the first
row and the failure of every later answer pass the budget, the batch ends
with the one failure that names the answers left out, as a batch without the
mark does.

The mark is a declaration that crumbkit-test's `check_opt_outs` lists as the
`left-out-per-key` customisation ([declarations](../../adoption/declarations.md)).

Why: ADR-0068.

## Headed bytes

The text of a [headed batch](../README.md#headed-bytes) prints each row whole
or not at all, because a file cut short would look whole. Each row costs its
header and its bytes, and 1 more byte for the line feed before its header
when it is not the first row that prints.

- A row whose header and bytes alone pass the budget does not print. It
  fails under its own key with `crumbkit::output::over_budget`, whose `key:`
  line names the key and whose `help:` line says that no flag makes the
  answer smaller. The rows after it still print. A document marked with
  `Document::always_print_first` prints its first row whole even past the
  budget.
- The other rows print in order while they fit. From the first row that does
  not fit, every later row and every key that the handler never fetched is
  left out, and one over-budget failure names them, as the failure of a
  batch in text does, with the recovery that runs the command with every key
  left out. A document marked with `Document::left_out_per_key` gives each
  of them its own failure in place of that one.

Each failure prints on standard error with the batch's other failures, in the
order of their keys, and the failure that names the rows left out prints
last. The run exits with the code of the first failure. With `--json` a
headed batch is budgeted as any other batch.

Why: ADR-0062, ADR-0068.

## Single results

A single row, or a listing or list whose first row alone does not fit, is
refused with `crumbkit::output::over_budget`, whose recovery is a hint to ask
for fewer fields with `--fields`. A prose result is refused with the same
code and no recovery. A bytes result is refused in the same way and is never
cut. Its text form is measured as the raw bytes and its JSON form as the
document with the base64 text, so `--json` can refuse a result whose text
fits. Each refusal exits with code 1.

A document marked with `Document::always_print_first` always prints its first
answer: a single row, a prose result or a bytes result prints whole even past
the budget, and a listing or a list prints at least its first row. A tool
uses it when the one answer is the point of the run, such as a file that the
caller named.

Why: ADR-0046.

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
`crumbkit::output::over_budget` in `Guide::codes`, which `cut` and the
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
once, then again with `CRUMBKIT_OUTPUT_BUDGET` set to half the bytes it
printed. The second run must do one of these:

- print a result marked with `cut` or the cut line;
- print one answer, either from a document that always prints its first
  answer or as a batch of one answer;
- exit with the code that the guide gives the over-budget failure, 1 by
  default, and print an over-budget failure that names the budget, last or,
  for a batch whose document answers each answer left out, among its other
  failures. Such a batch exits with the code of its first failure in key
  order, so when that failure is one of the tool's own, the check expects
  the code that the guide gives that failure. The check matches the
  failure's code to the guide's exits in the `::`, `.` and `/` spellings,
  and expects 1 for a code in a custom spelling.

The check also runs each text example of a leaf whose document is headed in
the same way. The second run must print at most the budget, unless its
document always prints its first answer, and must either exit with code 0 or
exit with the over-budget failure's code with a message on standard error
that names the budget.

Why: ADR-0046, ADR-0062, ADR-0068.
