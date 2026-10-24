# ADR-0062. Headed bytes

- Date: 2026-10-15
- Status: amended by ADR-0084 and ADR-0149

## Context

A command such as `show <NAME>...` reads several files and returns one
answer for each name. A `Bytes` result printed one file unchanged, and a
`Batch` printed its answers as a table, so a command that returned the
bytes of several files had no text form in which a person or an agent could
read each file. Readers of such a command's JSON also expected each answer
to hold `text` or `base64` beside its other fields, while the schema of
`Bytes` described two separate objects that a row could not flatten into
its own properties.

## Decision

- A row type implemented `Headed` to return its bytes, or `None` for a row
  that holds none. `Document::headed` marked the document of a batch of
  such rows. The marker is a bound on the row type, so a document whose
  row has no bytes fails to compile.
- The text of a headed batch printed each row's bytes unchanged, as `head`
  prints several files: a line `==> KEY <==` before each row when the
  caller gave more than one key, with the key's control characters escaped,
  and a line feed before every header but the first. With one key the
  bytes printed alone. A key whose lookup failed got no header, and its
  failure printed on standard error.
- A row without bytes printed under its header as one line of compact JSON
  with every field, because the row's JSON is the one form that every row
  type already has.
- `Bytes` serialized as one object with `text` or `base64`, and its schema
  listed both properties at the top level with a `oneOf` that requires one
  of them. A row field marked `#[serde(flatten)]` therefore put `text` or
  `base64` at the top level of the row, in the JSON and in the schema.
- The headed text kept each answer whole. A row whose header and bytes
  alone passed the output budget failed under its own key with the
  over-budget failure, unless it was the first row of a document that
  always prints its first answer. The other rows printed in order while
  they fit. From the first row that did not fit, every later row and every
  unfetched key went into one failure that names them and the command that
  fetches them.
- With `--json` a headed batch printed every field of each row unless
  `--fields` named some, so a reader gets the bytes without a flag. The
  leaf offered neither `--long` nor `--full`, and `parse` refused
  `--fields` without `--json`, because the text has no columns.
- A headed leaf's `--help` listed the row's columns, then `text` and
  `base64`, under `JSON fields that --fields selects:`, because those are
  the names that `--fields` selects and the text has no columns. `commands
  list` gave the leaf the form `headed` and the same fields as its
  columns, and crumbkit-markdown's reference printed the same heading and
  help's `Output:` line from that form. `check_guide` failed a headed leaf
  whose row had no top-level `text` or `base64` field.
- `check_output_budget` ran the text examples of a headed leaf as well as
  its JSON examples, and accepted a JSON batch of one answer past the
  budget that exits 0, because a batch always prints its first answer.

## Consequences

`docs/behaviour/output/README.md` describes the headed text and
`docs/behaviour/output/budget/README.md` describes its budget. A reader of
a headed command's JSON finds `text` or `base64` at the top level of each
row, and a reader that already took a `Bytes` result's JSON sees the same
object. A reader of the text cannot tell a header line from a file line
that looks like one, as with `head`, so a program reads the JSON.

Considered: a marker on a field of the row, a derive macro for the marker,
a check in `check_guide` that the marked field is `Bytes`, a line feed
added after bytes that do not end in one, a header for a single key, a
header for a failed key, a row without bytes refused, a row without bytes
printed as its columns, cutting a row to the budget, stopping at the first
row larger than the budget, `--fields` selecting the text of a headed
batch, `--json` printing only the columns of a headed batch, a separate
`Bytes` schema for flattened fields, no columns section for a headed leaf,
a headed leaf's columns without `text` and `base64`, a `commands list`
field for the `Output:` line in place of the `headed` form.
