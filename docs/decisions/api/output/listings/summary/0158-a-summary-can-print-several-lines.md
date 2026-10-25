# ADR-0158. A summary can print several lines

- Date: 2026-10-25
- Status: accepted
- Supersedes: ADR-0050 and ADR-0142, which it amends
- Missed: ADR-0142 rendered the summary to a string but wrote that string unchanged, so the summary kept its control characters and a final line feed printed an empty line above the header.

## Context

ADR-0016 and ADR-0050 print a listing's summary as one line above the
table, and ADR-0142 prints no line for a summary whose `Display` writes no
text. An adopter's report opens with a few count lines above its rows, such
as the number of entries checked and the number that need attention. The
adopter's summary therefore writes several lines.

crumbkit wrote the summary's text unchanged and added a line feed. A
summary that wrote several lines did print them, but the documentation
promised one line, a summary that ended with a line feed printed an empty
line above the header, and the text kept its control characters. A summary
that holds a value from outside the tool, such as a name, could then write
a terminal escape sequence or a carriage return to the terminal, which a
table cell, a child line and a prose result never do.

## Decision

- Each line feed in the text of the summary's `Display` ends a line, and
  the text output prints every line above the table. A final line feed
  ends the last line and adds no empty line. An empty line inside the text
  still prints.
- Each line escapes its other control characters and its Unicode format
  characters as a table cell escapes them, so a tab prints as `\t`, an
  escape as `\u{1b}` and a carriage return as `\r`.
- crumbkit renders the summary into its escaped lines once, when it
  prepares the result, and the output budget measures the text with every
  line. `Room` measures only batches, which hold no summary, so it does
  not change.
- The rules of ADR-0142 and ADR-0153 hold: a summary whose `Display` writes
  no text prints no line, a summary that writes only whitespace prints its
  line, and a listing with a summary and no rows to print prints the
  summary lines with no header.
- The JSON document, the document's schema and the usage record do not
  change.
- crumbkit adds no public API, so API-1 asks for no ground. The change
  serves an ordinary need, because many reports open with more than one
  figure (ADR-0122).

## Consequences

A tool writes several summary lines with line feeds in its `Display`. A
tool whose summary relied on a tab or another control character reaching
the terminal now sees it escaped. A summary that writes only a line feed
prints one empty line, as a summary that writes only spaces prints its
line. `docs/behaviour/output/listings/README.md` describes the current
behaviour.

Considered: a `Listing` method that takes a list of lines, which would add
public API for what line feeds in `Display` already say; keeping tabs
unescaped as a prose result does, which would make the summary escape less
than the table cells and child lines printed under it, although a summary
often holds values from outside the tool; treating a
carriage return before a line feed as part of the line ending, which would
hide a character that a table cell shows; and dropping empty lines inside
the summary, which would remove a separator the tool wrote on purpose.
