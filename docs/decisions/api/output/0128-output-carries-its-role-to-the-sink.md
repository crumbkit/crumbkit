# ADR-0128. Output carries its role to the sink

- Date: 2026-10-20
- Status: accepted

## Context

crumbkit prints two kinds of output. The result is what the caller asked
for: the rows or the document of a command, help, the version, a topic, a
completion answer, and the failure document when the form is JSON
(ADR-0098, ADR-0100, ADR-0125). A message is anything crumbkit says about
the run: a refusal and the group page it prints (ADR-0118), the text form
of a failure, a note under a listing (ADR-0050), the dry-run line, a
question, a picker's echo, a debug-build warning, and the report of a
failed write. The result goes to standard output and every message goes to
standard error (ADR-0049, OUTPUT-1).

Before this record, each printing site chose the stream itself by naming
the sink's output writer or its error writer, and several sites chose it
before the final write. `parse` rendered a group's page as plain text at
parse time because `Sink::styled` described the output writer alone.
`finish` read the width of standard output and threaded it through
`Output` to the table writer. `finish_to` counted the bytes of the output
writer through a wrapping writer. `stream` split the sink into two writers
when it opened. Whether a refusal on standard error may carry colour is
undecided, and a decision either way would have touched every site that
prints one.

## Decision

- Each piece of output carries a `Role` from the site that produces it to
  the sink. A role is the result or a message, and a message has a kind:
  refusal, refusal help, failure, note, debug warning, dry run, prompt,
  echo or failed write.
- A crate-private `Deliver` trait takes each piece with its role.
  `Sink` implements it, and every function below a printing site takes a
  `&mut dyn Deliver` in place of a pair of writers. A site composes each
  piece whole and delivers it in one call, so a failure's text reaches the
  error writer in one write.
- The sink's implementation of `Deliver::deliver` is the one place that
  maps a role to a stream. The result goes to the output writer and every
  message goes to the error writer. The sink counts the bytes of the
  result that the output writer took, and `finish_to` reports them in the
  `finished` event (ADR-0057).
- The sink answers the styling and width questions per role.
  `Deliver::styles` is true for the result when `Sink::styled` asked for
  styles, and false for every message. `Deliver::width` is the terminal
  width for the result when `finish` gave the sink one, and `None`
  otherwise. A site that renders a page asks the sink before it renders,
  so a refusal's group page reaches the printing site as clap's styled
  text and prints plain.
- The form of a failure decides its role. A failure document in JSON form
  is the result. The text form of a failure is a failure message, and the
  text form of a refusal is a refusal message. A form that the tool fixed
  with `Refused::as_json` (ADR-0125) therefore fixes the role.
- `Sink::standard` stays the only opener of the standard streams, and the
  `sink` module answers every question about them: whether a stream is a
  terminal and how wide the output terminal is. clap's `Error::print`
  keeps its exception for help at a styled standard sink (ADR-0021).
- The public API gains no item. `Role`, `Message` and `Deliver` are
  private to the crate, because no adopter prints a labelled message
  through the sink (API-1). `Picker::pick` keeps its writer argument, and
  the sink hands it the writer of the prompt role with its input.
- This record changes no printed byte. Every golden file and every test's
  expected bytes stay as they were.

## Consequences

A site that prints names what it prints and never names a stream. A later
record that colours a message kind on standard error changes
`Deliver::styles` in the sink for that kind and touches no printing site.
`Output` no longer carries a terminal width, and the wrapping writer that
counted bytes is gone. A test asserts on the role and the bytes of each
piece through a recording `Deliver`.

Considered: a writer per role on the sink, which needs two writers borrowed
at once in the result writer and the stream; a stream enum passed from the
caller, which keeps the choice at the caller; a public `Role` so that a
tool prints its own messages through the sink, which no adopter has asked
for; mapping every failure to the error stream, which moves the JSON
failure document and changes the bytes that ADR-0098 and ADR-0100 fixed;
warning and progress kinds, which no site prints today and which a site
adds when it prints one.
