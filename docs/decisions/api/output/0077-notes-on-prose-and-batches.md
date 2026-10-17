# ADR-0077. Notes on prose and batch results

- Date: 2026-10-17
- Status: accepted
- Supersedes: ADR-0050, which it amends
- Missed: ADR-0050 gave the tool's note only to a listing, so a prose or batch result had no way to print a line that describes it, and a tool wrote that line to standard error itself.

## Context

`Listing::note` set a line that the text output printed on standard error
with the other lines that describe a result, and the JSON document left it
out. A command that returned a `Prose` or a `Batch` had no such line. A tool
that needed one wrote to standard error itself, so the line fell outside
crumbkit's order of standard error, outside the single write of a result's
notes, and could print before standard output was flushed.

## Decision

- `Prose::note` and `Batch::note` set the same note as `Listing::note`.
  `Rows::note` returns it, and `finish` prints it through the code that
  prints a listing's note, in the same place: after any footer and the note
  of the rows that report a problem, and before the `Next:` line. With
  `--json` the document leaves it out.
- The note prints before the failures of a batch or a prose result, because
  every note reaches standard error in one write after standard output is
  flushed, and the failures print after that write.
- The note of a batch prints also when the output budget leaves out some of
  its answers, as a listing's note prints beside the cut line. A result that
  the budget refuses prints no note, as a refused listing prints none.
- The second use under API-1 is shelf's own. `books summarize` notes the
  removed books that a summary of the whole shelf leaves out, and
  `books show` notes which of the ids it cannot find name books that were
  removed from the shelf.

## Consequences

`docs/behaviour/output/README.md`, the listings page and the shapes page
describe the note of each result.

Considered: a note printed after a batch's failures, a note that a budget
cut leaves out, a note for a single row and a list, a note for bytes, a
separate `notes` field in the JSON document.
