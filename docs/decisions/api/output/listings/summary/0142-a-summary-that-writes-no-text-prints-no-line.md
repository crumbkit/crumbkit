# ADR-0142. A summary that writes no text prints no line

- Date: 2026-10-22
- Status: amended by ADR-0158
- Supersedes: ADR-0016 and ADR-0050, which it amends

## Context

ADR-0016 and ADR-0050 gave a listing an optional summary through
`Listing::summary`. The text output printed the summary's `Display` as one
line above the table, and the JSON document held the summary as an object
under `summary`. crumbkit wrote that line with its newline whenever the
listing held a summary.

An adopting tool set a summary on every run, so that its JSON document
always held the counts. On some runs the tool had nothing worth saying in
text, so the summary's `Display` wrote nothing. crumbkit then printed an
empty line above the table. A reader that takes the first line of the text
output as the header read the empty line instead, and a person saw a blank
line before the table.

## Decision

- crumbkit renders the summary's `Display` to a string before it writes the
  text output. When the string is empty, the text output prints no summary
  line, and the header is the first line.
- A summary whose `Display` writes only whitespace still prints its line.
  Only empty text means that the summary has nothing to say, and crumbkit
  does not judge whether other text is worth printing.
- The JSON document, the document's schema and the usage record keep their
  shape. The JSON document still holds the summary under `summary`.
- The output budget measures the text that crumbkit writes, so an empty
  summary spends none of the budget. Streams and batches hold no summary,
  and the footer and `--fields` do not read it, so none of them changes.
- crumbkit adds no public API.

## Consequences

A tool can set a summary on every run so that its JSON always holds the
figures, and write text only on the runs that have something to say.
`docs/behaviour/output/listings/README.md` describes the current behaviour.

Considered: a trait method on the summary that returns an optional line,
which would add public API for what an empty `Display` already says; a
summary that is skipped when it serializes to an empty object, which would
tie the text output to the JSON and would not cover a summary that holds
counts but has nothing to say in text; leaving the line and asking the tool
to build a listing without a summary on those runs, which would make the
JSON document's `summary` depend on the text.
