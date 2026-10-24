# ADR-0016. Result formats

- Date: 2026-10-08
- Status: amended by ADR-0059, ADR-0071, ADR-0082, ADR-0105, ADR-0114, ADR-0142, ADR-0143 and ADR-0149

## Context

A tool's result is read by people, by agents and by programs. Agents pay
for every token they read, programs need a schema, and a pipe needs data
without decoration. Commands also answer in different shapes: rows, a
page of a larger set, the answers for many ids, a few sentences, or the
bytes of a file.

## Decision

- Results came in two formats: compact text for people and agents, and
  JSON for programs. The text table already has a header, one record per
  line and no repeated keys, so no command prints Markdown or another
  format such as TOON.
- A row type implemented `Row` to name its compact and `--long` columns.
  The table pads by characters and escapes control characters, so each
  row stays on one line.
- The `Output` flags were global, but a leaf with a document offers only
  the view flags its document can use, and `parse` refuses the others
  before the handler runs. crumbkit checked the conflicting flag pairs
  itself, because clap's rules refuse them in clap's words with a recovery
  that only opens help. A leaf that changes something must declare a
  document, because otherwise a refusal comes after the write.
- Progress was the tool's own, drawn only when `Output::progress_allowed`
  is true, so programs, agents, pipes and checks never receive progress
  lines and crumbkit takes no progress library.
- A command that may show fewer rows than matched flattened `Page` and
  returned a `Listing` with a total it counted itself, so a handler never
  loads a row it will not print. crumbkit owns the footer's wording.
- A failure printed as text on standard error, or as one failure document
  with `--json`. With the `miette` feature, crumbkit drew each label's
  source lines itself, with long spans shortened to their first and last
  lines, because a failure stays short and the end of a span is often
  where the fix goes.
- A command that takes many ids returned a `Batch`, which keeps each key
  as the word a caller types and gives each failure its key and index, so
  a reader matches failures to keys even when keys repeat. One failing id
  does not fail the whole command.
- A few sentences returned `Prose`, which prints its `Display` as text and
  its value as JSON, so a person reads sentences and a program reads
  fields. Its text escapes control characters except line feeds and tabs,
  so data cannot send a terminal escape sequence.
- The bytes of a file returned `Bytes`, written unchanged as `cat` does,
  so a caller can pipe a file through the tool. JSON holds text or
  base64. crumbkit encodes base64 itself, because it needs one alphabet
  and no decoder.
- `Document::without_envelope` let a command keep an existing document's
  shape, and `failures_in_order` let an unwrapped batch print each failure
  among its rows, opt-in because readers of an existing document expect
  rows alone.
- `finish` wrote through a `Sink`, so tests run in memory, and a closed
  pipe keeps the result's exit code.

## Consequences

`docs/behaviour/output/README.md` describes the current formats, flags
and result shapes.

Considered: clap's `conflicts_with` and `requires` for the pairs of view flags, rewriting clap's error for those pairs, clearing clap's rules for those pairs inside `parse` only, a progress API in crumbkit, progress that also checks `TERM`, a failure on standard output without `--json`, TOON, a Markdown result format, unicode-width for cell widths, serde_json's `preserve_order` feature for column order, global paging flags, the description in the JSON document, the note in the JSON document, a summary as text only, a summary held as a JSON value, deriving `Serialize` and `JsonSchema` for `Page`, failing a whole many-id command when one id fails, a batch built from rows and failures without its keys, keys kept as field values, a grouped failure for several keys in a batch, a summary line after a batch's failures, a batch's failures on standard output in text, a separate exit code for a batch in which some ids worked, carets counted by characters, a width table of crumbkit's own, miette's graphical form for every reader, a span printed as its first line only, a gutter mark on each line of a span, a span cut after its first lines, one block per label with repeated lines, a one-column row for an answer in prose, a listing with a summary and no rows, a plain string result, `--fields` selecting the fields of a prose value, `failures` only in a prose document that can fail, a prose text printed without escaping, a separate `finish` for unwrapped documents, a batch failure's key inside `fields`, a key without its index, the failures of every unwrapped batch among its rows by default, the failures of an unwrapped batch in a second array after its rows, failure documents in input order without `schema`, a wrapper result type for them, every failure of an unwrapped command as text, `--fields` refused for an unwrapped document, a bytes result read from a stream, a bytes result that borrows its bytes, a bytes result refused at a terminal, the base64 crate, a bytes result cut to the budget, a convention check against a `Bytes` column, an unoffered view flag accepted as a no-op, clap's unknown-argument error for an unoffered view flag, `--full` offered only for a row type with fields outside its columns, the offered view flags of each leaf in `commands list`, refusing every view flag on a leaf with effects and no document, a document required on read-only leaves, an opt-out for a changing leaf whose output a tool cannot describe, an opt-out in place of the exemption for a leaf that passes on a program's exit code.
