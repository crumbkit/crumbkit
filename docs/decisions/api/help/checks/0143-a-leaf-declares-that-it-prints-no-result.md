# ADR-0143. A leaf declares that its command prints no result

- Date: 2026-10-22
- Status: accepted
- Supersedes: ADR-0016 and ADR-0105, which it amends
- Missed: ADR-0105 gave a command without a result only an opt-out from the document rule, and the opt-out keeps `--fields`, `--long` and `--full` in the command's help although the command can use none of them.

## Context

An adopting tool has a command that runs a local server until the caller
presses Ctrl-C. The command writes only log lines to standard error,
prints no result on standard output and never succeeds: it ends with a
failure such as an interrupt. crumbkit could not describe it. A leaf
without a document offered every view flag (ADR-0016), so the command's
help listed `--fields`, `--long` and `--full`, and `parse` accepted them
although they did nothing. A changing leaf without a document failed the
document rule of `check_guide` unless the tool opted out (ADR-0105), and
the opt-out kept the flags in help. The tool's workaround was a prose
document with an invented id. crumbkit then published a schema for a
document that never prints, the opt-out report said that the command
printed it, and the refusal of a view flag told the caller to use `--json`
to read every field of the result. The failure path already worked
without a document, through `Output::print_failure`,
`Output::failure_exit` and `events::ended`.

## Decision

- `Leaf::prints_no_result` declares that the command prints no result on
  standard output. `Leaf::get_prints_no_result` reports it.
- Such a leaf offers none of `--fields`, `--long` and `--full`. `parse`
  refuses each of them before the handler runs, with the refusal that a
  prose or bytes document gets. The message says that the command prints
  no result, and the hint tells the caller to drop the flag. Help leaves
  the three flags out. `--json` stays, so a failure prints as the failure
  document.
- The leaf's `-h` and `--help` print `Output: no result on standard
  output, and --json prints a failure as JSON.` where a stream or bytes
  print their `Output:` line.
- The leaf has no document, so it publishes no schema, and
  `check_document_ids` and the opt-out report pass over it.
- The document rule of `check_guide` accepts the leaf without an opt-out,
  and `check_guide` fails a leaf that declares both a document and
  `prints_no_result`, naming the leaf.
- `commands list` writes `form: null` for the leaf, as for any leaf
  without a document, because a new value of `form` would change a
  published schema.

## Consequences

`docs/behaviour/output/shapes/README.md` describes the current
behaviour, and the output, help, checks, schemas and declarations pages
link to it.
No example tool has a command that runs until it is interrupted, so no
example declares it. A `RuntimeLeaf` cannot declare it.

Considered: a stream that never ends, which publishes a schema for rows
that never print; a placeholder document, which is the workaround above;
a `finish` that takes only a failure, which adds a second way to end a
run beside `print_failure`; a `Rows` impl for `Infallible`, which gives
the leaf a document whose schema describes nothing; a new `form` value in
`commands list`.
