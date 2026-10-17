# ADR-0046. Output budget

- Date: 2026-10-12
- Status: amended by ADR-0068, ADR-0069 and ADR-0070

## Context

An agent that runs a tool reads the whole of standard output into its
context, so one run that prints a large result can use up the context
that the rest of a task needs. A cut result must say that it was cut and
how to get the rest, or the agent works from incomplete data without
knowing it.

## Decision

- Each run got an output budget: the most bytes that `finish` and
  `stream` print on standard output in one run, 1 MiB unless the tool
  sets another with a reason. A tool's budget is at least 1024 bytes, and
  a smaller one panics when the guide is built.
- No opt-out removes the budget. `CRUMBKIT_OUTPUT_BUDGET` may lower it
  for one run and never raises it, so no one reads more than the tool's
  budget in one run. A reader who needs more asks for less and runs the
  command more than once.
- `finish` measured a result before printing anything, and the result's
  shape decided what happens when it does not fit:
  - A listing or list prints the most rows that fit and exits 0, with a
    cut line in text and a `cut` field in JSON, so the reader knows it was
    cut.
  - A batch prints the rows that fit, always its first, and ends with one
    over-budget failure that lists the keys left out. Keys are listed
    because an agent that sees only a count must work out which keys are
    missing. One failure per key would itself pass the budget. Its
    recovery runs the command once with every key left out, because a run
    for each key would start hundreds of processes. Each such run answers
    at least one key, so a chain of them always makes progress.
  - In JSON, rows keep priority over keys down to one row, because a row
    is the answer and a key only says where to find one.
  - A single row, a prose result or a bytes result is refused. A bytes
    result is never cut, because a program that reads part of a file
    cannot tell that bytes are missing.
- `Batch::has_room` let a handler stop fetching early, and keys it never
  fetched are listed too, so every key is answered or listed whatever the
  estimate.
- `Document::always_print_first` let a document whose one answer is the
  point of the run print it whole past the budget.
- A stream ends at the budget, so a long-running stream raises its
  budget.
- The cut and the failure never depend on whether a stream is a
  terminal (OUTPUT-1).
- `check_output_budget` reruns each `--json` example with half its bytes
  as the budget and checks that the run marks the cut or fails with the
  over-budget code.

## Consequences

A tool can rename the over-budget code and give it its own exit code.
`docs/behaviour/output/budget/` describes the current behaviour.

Considered: no budget, a budget counted in rows, a budget counted in tokens, a budget that a flag raises, an opt-out that removes the budget, a variable that also raises the budget, a variable that refuses an invalid value, a page of the result with a cursor to the next page, a cut listing that exits with code 1, one failure per id that a batch left out, a count of the rows that a batch left out, keys from a `Row` method, keys that only `fetch_each` records, a fixed share of the budget for a batch's keys, one run for each key left out (`each`), a call text that shows `<ID>` without `...`, a concrete call for the first key left out, a room check on `Output` that measures rows in the chosen format, `fetch_each` that stops at the budget, a cut field that replaces `truncated`, a cut line on standard output in text, a budget that also counts the notes on standard error, a refused stream that prints nothing, a stream that drops items silently past the budget, a counting pass before every write, a new document id for the documents that hold `cut`, an over-budget code set on each `Document`, the keys left out listed one by one in the over-budget failure.
