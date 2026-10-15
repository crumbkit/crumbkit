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

A batch's handler can stop fetching once `Batch::has_room` returns false.
`has_room` measures each new row once as compact JSON, which is an estimate
of what prints. The keys that the handler never fetched follow the keys of the cut rows in
`left_out`, and a batch with such keys ends with the failure even when its
rows fit. An estimate that stops too early or too late therefore still leaves
every key either answered or listed.

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
- print one answer from a document that always prints its first answer;
- exit with the code that the guide gives the over-budget failure, 1 by
  default, and end with the over-budget failure that names the budget.

Why: ADR-0046.
