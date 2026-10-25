# ADR-0157. A row type chooses which rows the default text lists

- Date: 2026-10-25
- Status: accepted
- Supersedes: ADR-0138 and ADR-0153, which it amends
- Missed: ADR-0138 let only a document choose which rows the default text lists, and only by whether a row reports a problem, although a tool can also want to leave out rows that report none. ADR-0153 dropped the header only for a listing that holds no rows or lists only its problem rows, and had no case for a listing whose default text leaves out its rows for another reason.

## Context

An adopting tool has a cleanup command that returns a listing of the items
it handled. A dry run removes nothing, and its text should list every item.
A real run removes most items, and its text should list only the items that
carry their own next step for the caller, such as a lock that the caller
must release first. `--json` should hold every item in both runs, so a
script sees what the run did. `Document::only_problems` cannot express
this, because the listed items report no problem, and the choice depends on
each row. A handler that filtered its own rows would also filter the JSON.

## Decision

- `Row::in_default_text`, a method with a default that returns true,
  reports whether the default text of a listing lists the row. The default
  text is the compact text without `--json`, `--long` or `--fields`.
  `--long` and `--fields` print every row, and `--json` holds every row
  unchanged, as ADR-0138 decided for problem rows.
- The default text lists a row when the row reports a problem, or when the
  method returns true and the document is not marked with
  `Document::only_problems`. A row that reports a problem is always listed,
  so the note that counts those rows and names the exit never counts a row
  that the text leaves out (OUTPUT-3). `only_problems` becomes the
  strictest case of the rule: it leaves out every row that reports no
  problem. Its behaviour does not change.
- The rule applies only to a listing, as `only_problems` does. A list, a
  batch, a single row and a stream print every row, so `Room`, which
  measures a batch, does not change.
- The header follows ADR-0138 and ADR-0153 with "rows that the default text
  lists" in place of "rows". The text prints a header when it lists a row.
  When it lists none, it prints a header only if it left no row out, the
  document does not list only its problem rows, and no summary line stands
  in the header's place. A listing whose rows the method all leaves out
  therefore prints its summary line alone, or nothing on standard output
  when it has no summary, as a listing that lists only its problem rows
  does when none reports one. A listing that holds no rows keeps the rule
  of ADR-0153.
- The footer, the empty note and the cut line count as they do under
  `only_problems`. The footer counts every row of the page, the empty note
  prints only for a listing that holds no rows, the output budget's cut
  line counts the printed rows against the listing's total, and no line
  counts the rows that the text left out. One rule then holds for every
  row that the default text leaves out, whatever left it out.
- The help prints no `Rows:` line and the leaf offers `--long` only when the
  row type has long columns. `parse` decides the help and the view flags
  before the handler runs, and the method answers for a row, so `parse`
  cannot know whether it will leave any row out. A tool says in the
  command's about text which views list every row.
- The API-1 ground is one adopter's request for an ordinary need, because
  any tool whose run handles many items can want its default text to show
  only the items that need the reader, while its JSON keeps every item
  (ADR-0122).
- ADR-0138 and ADR-0140, which decided the rows of the default text before
  this record, move under it, because the listings directory reached its
  limit of 5,000 words (DOCS-2).

## Children

- [ADR-0138. A listing can list only its problem rows](0138-a-listing-lists-only-its-problem-rows.md)
- [ADR-0140. The `Rows:` line names every view that lists every row](0140-the-rows-line-names-every-view-that-lists-every-row.md)

## Consequences

A row type chooses its default rows without a new document option, and
every existing row type prints the same bytes. A row type that leaves out
rows and has no long columns lists every row only through `--fields` and
`--json`, and its help does not say so unless the tool's about text does. A
footer such as `Showing 3 of 5 items.` can count rows that the text did not
list. `docs/behaviour/output/listings/README.md` describes the current
behaviour.

Considered: a `Document` option that names the row method, which a row type
whose rows always choose would have to repeat on each document; a `Row`
constant that declares that the method can leave rows out, so the help could
print a `Rows:` line and offer `--long`, which waits for a tool that needs
it; letting the method also leave out rows that report a problem, which
would let the problem note count rows that the text does not show; a footer
that counts only the listed rows; an empty note when the text lists no row;
applying the rule to a list and a stream; replacing `only_problems` with a
row method, which would stop one row type from serving a listing of every
row and a listing of its problem rows; and the names `listed_by_default`,
`shown_by_default` and `in_short_text`.
