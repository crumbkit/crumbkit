# Failure forms

This page covers how a failure prints in text and as a document: the source
excerpts that the `miette` feature draws, and the failure form that a tool
sets in place of crumbkit's failure document.
[`FailureForm`](../../../crates/crumbkit/src/form.rs) and `PreparedFailure`
describe the trait and what a form receives.

## Source excerpts

With the `miette` feature, each label of a failure's diagnostic prints an
`at:` line, then the lines its span covers with one line before and after,
drawn by crumbkit, so a reader who cannot open the source sees what the label
means. Carets mark the span on its first and last line. The `code:` line
stays last, so `| tail` keeps it.

- The carets measure terminal columns with unicode-width, so they line up
  after wide characters such as CJK text. The `at:` column counts
  characters, as an editor does.
- A span over more than 7 lines prints its first 3 and last 3 lines and one
  row that counts the lines left out. A row that would count one line prints
  that line.
- Labels that follow each other and whose lines overlap or touch print as one
  block: their `at:` lines, then each source line once, with one underline
  row per label that ends with the label's text, so no line prints twice.

Why: ADR-0016, ADR-0001.

## Setting a form

A tool that already publishes its own failure document, which its callers
read, keeps that document by setting a `FailureForm` with
`Guide::failure_form`. Its provided method `write_text` writes crumbkit's
text, so a form that changes only the JSON keeps crumbkit's `error:`, `help:`
and `code:` lines. crumbkit re-exports schemars as `crumbkit::schemars`, so a
form builds its schema without a schemars dependency of its own.

crumbkit prepares each failure before the form sees it. The form receives a
`PreparedFailure`, with the code in the tool's spelling and each `Run` call
already rendered against the tool's command tree. The failure reference is
not `'static`, so a form cannot downcast it to the tool's error type.
A failure of a batch also carries its key and the key's index. A form
that names the key only when the caller gave more than one key counts the
keys from its own arguments.

`parse` reads the form from the guide and passes it on in `Parsed`, so
`Output::set_command` gives it to `finish` and `stream`. A tool that parses
with clap alone builds its `Parsed` with `Parsed::guided(command, &GUIDE)`.
`Parsed::from(command)` keeps working for a tool without a guide and sets no
form.

Why: ADR-0045, ADR-0071.

## What a form prints

The form writes every failure that prints alone:

- the failure of a whole command from `finish`;
- each failure line of a `--json` stream;
- a command line that `parse` refuses;
- a `--fields` name that `finish` or `stream` refuses;
- the failures of a picker's source and of a child process;
- the refusal of a result that passes the [output budget](../output/budget/README.md).

Its text form also prints each failure of a batch or a prose result on
standard error, including those of a document without the envelope. Exit
codes do not change.

crumbkit writes the form's JSON on one line, or in the guide's
[JSON layout](../output/json-layout.md) outside a stream, with its keys
sorted when the guide sets [`KeyOrder::Sorted`](../output/json-layout.md#key-order),
and flushes it, so
each failure line of a `--json` stream stays one line and a closed pipe keeps
the exit code. crumbkit flushes standard error after the form's text. A form
that writes its own text escapes the control characters of the values
itself.

The provided method `entry` writes each entry of a batch's or a prose
result's `failures`, including the over-budget failure that ends a batch, and
`entry_schema` returns the schema that `schemas show` serves as the `items`
of `failures`. The defaults return crumbkit's entry and its schema. A form
that implements both gives its tool one failure shape in all of its JSON, and
crumbkit still decides which failures an envelope holds, their order, their
recovery and the exit. A document that prints its failures in input order
holds each failure as the form's `document`, and its schema takes the form's
`schema` as the failure item.

Why: ADR-0045, ADR-0046, ADR-0052, ADR-0067.

## Schemas and declarations

`schemas list` serves the form's document first, in place of
`crumbkit/failure/v1`, with the kind `unwrapped` and no commands, because
every command prints it when it fails. `schemas show` serves the form's
schema with the document's id inserted as `title`. crumbkit-test's
`check_schema_goldens` pins the schema, because the form's document is in the
tool's component ([schema commands](../schemas/commands.md)).

A failure form is a [declaration](../adoption/declarations.md), because
crumbkit still decides when each failure happens, its recovery and its exit.
It needs no opt-out, and `check_opt_outs` lists it under the customisations
with no reason and no cost. crumbkit-test's `check_document_ids` fails on a
form whose document is in the component `crumbkit`, because crumbkit owns
that component.

Why: ADR-0045, ADR-0056.

## Reading failures back

crumbkit-test's `check_twin`, `check_recoveries` and `check_examples` read a
failed run through one function. It tries the tool's reader first, then
crumbkit's failure document, then its `code:` line, and returns every failure
that the first of them reads.

- The tool's reader is the form's `read`, which each check takes from the
  guide, so a tool names its form once. The default `read` returns `None`.
  A form that implements it gets the same checks as crumbkit's own failure
  document.
- A form can write another code in place of one of crumbkit's, because it
  receives the code and writes both the JSON and the text. A form that does
  this maps the code back to crumbkit's in `read`, so the checks still match
  it.
- The checks pass each entry of a batch's `failures` to the form's `read` as
  one line of standard output, and read an entry that the form cannot read in
  crumbkit's shape.
- A tool that prints failures in a shape of its own without a form
  implements crumbkit-test's
  [`FailureReader`](../../../crates/crumbkit-test/src/failures.rs) and sets
  it with `Examples::failure_reader`. Each check fails when the guide sets a
  form and `Examples` sets a reader, because a tool reads its failures
  through one of them.
- A failed example's problem names the code and steps that the check read.

`check_recoveries` follows the `Run` recoveries that it reads, and it runs a
guide with a form like any other guide. When the form or reader reads no
failure from a run that failed, the check stops with one problem. The problem
says that the tool makes it read the failure, or names the root in an opt-out
of `Rule::Recoveries`, id `recoveries/follow`. The first unreadable run stops
the check, because every later run would fail the same way.

Why: ADR-0045.
