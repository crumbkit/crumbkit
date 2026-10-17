# ADR-0068. One failure for each answer left out

- Date: 2026-10-17
- Status: accepted
- Supersedes: ADR-0046, which it amends

## Context

A batch that did not fit in the output budget ended with one over-budget
failure that named every key left out (ADR-0046). Some tools already
answered each key of a batch with its own row or its own failure, and their
readers matched each failure to its key by index. Those readers could not
read one failure that held a list of keys, so the tools could not adopt the
budget for their batches without changing what their readers parse.

`Batch::has_room` let a handler stop fetching once its rows filled the
budget. It measured each row as compact JSON, so it stopped too early for a
pretty document and could stop too late for a text table, whose columns pad
each cell to the widest one. It also put serde_json into crumbkit-core for
this one measurement.

## Decision

- `Document::left_out_per_key` marked a batch's document to answer each
  answer left out with its own `crumbkit::output::over_budget` failure, at
  its key's position and with its key and index. The failure held only the
  field `budget`, its message said that the answer did not fit in the
  output budget, and its recovery ran the command with that key alone,
  because one run for each key is how such a reader already fetches an
  answer. An answer without a key got a hint to run the command again with
  the keys that were not answered.
- The failures merged with the batch's own failures in the order of their
  keys, and the run exited with the code of the first failure in key order,
  as a batch with failures already did.
- In a JSON document that holds its failures, each failure counted against
  the budget, and a row that printed took the place of its failure. A row
  could be shorter than its failure, so the size of the document did not
  grow with the row count and a binary search could not find the count.
  `finish` measured the document without rows once and each row and each
  failure once, and took the most rows whose sum fit. When even the first
  row and the failure of every later answer passed the budget, the batch
  ended with the one failure of ADR-0046, which names the keys more
  compactly.
- In text, and in a document whose failures print on standard error, the
  rows were cut as before and the failures printed on standard error, which
  the budget does not count.
- A headed batch's text gave each row left out its own failure in place of
  the one that named them.
- The mark was a declaration (ADR-0056). `check_opt_outs` listed it as a
  `left-out-per-key` customisation of the leaf's command, and
  `check_output_budget` accepted an over-budget failure that other failures
  follow.
- `Output::room(document)` returned a `Room` that measured each row once in
  the format that the run prints: the JSON document in the guide's layout,
  the text table with its column widths, or the headers and bytes of a
  headed batch. `Room::has_room` replaced `Batch::has_room`, because only
  crumbkit knows the format, and a measurement in that format never stops a
  handler before the row at which `finish` cuts the batch.

## Consequences

A tool whose readers match each failure to its key adopts the budget for its
batches with one declaration. A batch without the mark prints as before. A
handler that stopped with `Batch::has_room` asks a `Room` from its `Output`
instead. crumbkit-core keeps serde_json for `FieldValue::from_json` alone.

Considered: one failure per key for every batch, a failure per key that also
lists the keys after it, a recovery that runs the command with every key
left out, a binary search over the row count, a count of the failures in the
budget of text runs, keeping `Batch::has_room` beside `Room`, a `has_room`
on `Output` that takes the batch and the document on each call.
