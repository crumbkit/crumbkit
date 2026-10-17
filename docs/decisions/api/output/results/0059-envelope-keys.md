# ADR-0059. Envelope keys

- Date: 2026-10-15
- Status: amended by ADR-0066 and ADR-0071
- Supersedes: ADR-0016, which it amends

## Context

crumbkit's envelope held a document's id under `schema` and its result under
`data` (ADR-0016). Some tools already printed documents whose readers
expected other names: a type key such as `$type` that held a fixed string
such as `tool/get-report/v1`, and a data key that differed by command, such
as `answers` for a batch, `entries` for a listing and `rows` for a query.
Some of those documents also held fields beside the result at the top, such
as a listing's `location` object or a check report's `degraded` flag and
`degraded_total` count.

Such a tool could only print the document with `Document::without_envelope`.
That opt-out lost `total`, `truncated`, `cut` and `--full`, and needed an
opt-out with a reason and a cost (ADR-0036), although the tool kept every
behaviour and only named the keys.

## Decision

- A document named its two keys with `Document::with_keys(type_key,
  data_key)`. The document printed its id under the type key and its result
  under the data key, and kept every other part of the envelope: `total`,
  `truncated`, `summary`, `cut`, `failures` and `dry_run` printed as before,
  and the view flags, the output budget and streams worked as before. The
  keys were set per document, because one tool used a different data key
  in different commands.
- `Document::with_type` put a fixed string under the type key in place of
  the id. The string replaced the id, because the readers matched the whole
  string. The id stayed the name that `schemas list` and `schemas show`
  used.
- A const panic rejected a key that was one of the names crumbkit writes
  beside the result (`dry_run`, `total`, `truncated`, `summary`, `cut` and
  `failures`), two keys that were the same, a key with a line break, and
  `with_keys` together with `without_envelope`. crumbkit kept its own names,
  so a reader never met one of them with another meaning.
- A batch that also called `Document::failures_in_order` printed each row
  and each failure under the data key in the order of the keys, repeats
  included, with the over-budget failure last, and printed no `failures`.
  Each failure printed in the guide's failure form when it had one.
- A listing whose document called `Document::summary_at_top` printed the
  fields of its summary at the top of the document beside the keys, in
  place of a `summary` object. The summary already held the fields that a
  listing adds beside its rows, so this was the smallest seam. A summary
  field whose name was one of the document's keys or one of crumbkit's
  names failed the write, which the run reported as a failed write that
  exits with 1 and prints nothing on standard output. The check ran when
  the document printed, because the summary's field names come from its
  `Serialize` implementation. A check report that needed top-level fields
  printed a listing.
- Rows followed crumbkit's column rules. A row type whose every field was a
  compact column printed every field with `--json`, `--full` printed the
  same output, and `--long` was refused as a flag that the command does not
  offer, because the row type had no long columns.
- `schemas list` showed such a document with the kind `own-envelope`. Its
  schema named the declared keys, held the type or the id as the type key's
  `const`, and held the summary's schema under `allOf` when the summary
  printed at the top.
- The keys were a declaration (ADR-0056). `check_opt_outs` listed them as
  an `envelope-keys` customisation of the leaf's command, and
  `check_examples`, `check_output_budget` and the failure reader read the
  declared keys.

## Consequences

A tool whose readers expect other key names keeps every behaviour of the
envelope with no opt-out. A reader of crumbkit's own documents is
unaffected, because a document without `with_keys` prints as before. A
document with a summary at the top allocates one JSON value for the summary
on each write. A listing that prints its summary at the top also prints
`total` and `truncated`.

Considered: one pair of keys for the whole guide, the type string beside the
id under a second key, a `Document::extra` seam that holds a JSON map, a
`TopFields` trait on the result type, `#[serde(flatten)]` on the summary,
extra fields on every result shape, reserved names checked by a crumbkit-test
check in place of the failed write, the kind `output` for a document with
keys, an opt-out in place of a declaration.
