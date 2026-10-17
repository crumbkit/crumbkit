# JSON layout

This page covers how a JSON document is laid out: one line by default, or
indented when the tool sets a layout, and the order of the keys of each
object. A tool sets the layout with a
[`JsonLayout`](../../../crates/crumbkit/src/output/layout.rs), which
[`Guide::json_layout`](../../../crates/crumbkit/src/help/output.rs) takes,
such as `Guide::new().json_layout(JsonLayout::Pretty { reason })`. Their
rustdoc describes the settings themselves.

## One line by default

With `--json`, every whole JSON document prints on one line by default,
whether or not standard output is a terminal, so a run prints the same bytes
at a terminal and through a pipe ([terminal detection](terminal.md)). An
agent reads the fewest tokens, and a program reads each document with one
line read.

The `--json` help text says `Print the result as a JSON document` and names
no layout, because the layout belongs to the tool.

Why: ADR-0052, ADR-0081.

## The indented layout

A tool whose readers need indented JSON sets
`Guide::json_layout(JsonLayout::Pretty { reason })`. crumbkit then puts each
field and each item of a whole document on a line of its own, indented by
two spaces for each level, byte for byte as serde_json's pretty formatter
writes it. The setting applies at a terminal and through a pipe alike, so
OUTPUT-1 holds.

The setting covers the envelope, a document with
`Envelope::Without`, the failure document and a refusal's failure
document. A
[stream](streams.md) prints one document per line in every layout, including
a failure that ends the stream, because a reader of JSON Lines reads one line
at a time.

crumbkit serializes every document on one line and re-indents those bytes
for `Pretty`. The compiler then generates each document's serialization code
once for both layouts, so a tool that never sets the layout pays nothing in
binary size for it.

Why: ADR-0052.

## Costs and checks

The [output budget](budget/README.md) counts the bytes as printed, so with `Pretty`
it counts the indented bytes, and a listing cut by the budget fits fewer
rows. Indenting made the JSON documents of shelf's examples 20 to 62 percent
larger.

- crumbkit-test's `check_opt_outs` lists the layout as `json-layout/pretty`
  with the tool's reason and that cost ([opt-outs](../adoption/opt-outs.md)).
- `check_examples` reads the guide's layout. With `Pretty`, a `--json`
  example of a leaf of one result must print exactly one document whose
  second line is indented by two spaces, and a stream keeps one document per
  line.
- The recovery and twin checks, when they read a failed run's failure
  document, and `check_output_budget` read a document on one line or
  indented.

Why: ADR-0052, ADR-0036.

## Key order

Each JSON object prints its keys in the order that its value serializes
them, so the envelope's `schema` comes first and a row's fields follow their
declaration order. crumbkit offers no other order.

Why: ADR-0081.
