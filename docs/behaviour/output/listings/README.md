# Listings

This page covers a command that may show fewer rows than matched: its paging
flags, the lines that describe its result, a listing with no rows, a listing
without a total, the upper bound on `--limit`, and the exit of a result whose
rows report a problem.

## Paging flags and the listing

A command that may show fewer rows than matched flattens
[`Page`](../../../../crates/crumbkit/src/page.rs), so the paging flags belong to
that command and its `--help` shows its own default limit. The command
returns a [`Listing`](../../../../crates/crumbkit/src/listing.rs). The
rustdoc of both types describes how a handler builds a counted page, an
uncounted page, or a listing from rows already in memory.

Why: ADR-0016.

## The lines that describe a listing

When a listing shows fewer rows than its total, or leaves out rows that the
tool did not count, a footer says so and names `--all`. crumbkit owns the
wording, and the tool supplies only the description.

A listing may carry a summary, a note and one next step. The summary prints
as one line above the table on standard output, and the JSON document holds
it under `summary`. The note and the next step print in text only.

On standard output, the text order is the summary, the header and the rows.
On standard error, it is the footer or the empty note, the note of the rows
that report a problem, the tool's note, the `Next:` line and the dry-run line.
[Output](../README.md#standard-output-and-standard-error) describes how these
lines reach standard error.

A [batch and a prose result](../shapes/README.md) carry the tool's note in
the same way, through `Batch::note` and `Prose::note`, and it prints in the
same place in this order. A list of rows, such as a `Vec`, has no note.

Why: ADR-0016, ADR-0050, ADR-0077.

## A listing with no rows

A listing with no rows and no more rows prints its header on standard output
and a note on standard error in text, such as `note: no books match`. This
holds for a counted listing whose total is 0 and for an uncounted listing,
which has no total. The default wording is `no {description} match`, from
`Listing::described`, or `no rows match` for a listing without a
description.
[`Document::when_empty`](../../../../crates/crumbkit/src/document.rs) sets
the document's own wording in one line. `self usage` prints
`note: the usage log holds no findings`.

- A page that holds no rows from a total above 0 prints the footer in place
  of the note.
- A listing that the output budget cut to no rows prints the cut line and no
  note ([budget](../budget/README.md#listings-and-lists)).
- With `--json`, the listing's document is unchanged: `data` is `[]`,
  `total` is 0 and `truncated` is false. An uncounted listing's document
  leaves out `total`. A listing printed without the
  envelope prints `[]`.
- A list of rows, such as a `Vec`, prints no note.
- A stream prints its own empty note ([streams](../streams.md#notes)).

The [usage log](../../usage-log/README.md) marks a run whose listing or stream
printed the note.

Why: ADR-0050, ADR-0087.

## A listing without a total

A tool whose source cannot be counted cheaply, such as a remote API that
pages its answers or a log that grows while it is read, builds its listing
with `Listing::uncounted` or `Page::uncounted`. The reading example's
`entries list` reads its file line by line this way.

- The JSON document never holds `total`, whether or not more rows exist and
  whatever left rows out: the output budget, the page's limit or the tool.
  Every listing's JSON Schema marks `total` as optional.
- A page that holds every row of its source, as with `--all`, has
  `"truncated": false` and prints no footer, unless the output budget cut
  it. A cut page has `"truncated": true` and prints the cut line.
- While more rows exist, the footer gives no count and keeps `--all` as the
  next step:
  `Showing 4 books, and more match. --all shows every one.` It names the
  bound when the page has one, and says `rows` when the listing has no
  description.
- The output budget's cut line of such a listing also gives no count, as in
  `Showing 812 books, because output stops at 1048576 bytes.`
- When the footer or the cut line of such a listing shows one row, it names
  the row with the phrase that `Listing::singular` sets, as in `Showing 1
  book, and more match.` A listing without one names the row with its
  description, or as `row` when it has no description. A counted listing's
  footer reads `Showing 1 of 6 books.` and needs no singular.
- A page with no rows and more rows after it prints the footer and no empty
  note, as a counted page with no rows from a total above 0 does. A page
  with no rows and none after it prints the empty note, and its JSON
  document leaves out `total`.

Every listing's document holds `truncated`, which is true when `--limit`, the
output budget or an uncounted source left rows out, and false when `data`
holds every row. `cut` says that the budget was the cause and names the
budget, so a listing with `cut` also holds `"truncated": true`. A list has no
`truncated`, because only the budget shortens a list, and `cut` marks that.
The [usage log](../../usage-log/README.md) marks a run whose listing left rows
out.

Why: ADR-0050, ADR-0087.

## An upper bound on `--limit`

`Page<N, MOST>` takes an optional second constant, the most rows that
`--limit` takes. It defaults to `usize::MAX`, which means no bound. shelf's
`books list` uses `Page<4, 100>`.

clap refuses a larger value as it parses the command line. The refusal has
the code `<tool>::cli::usage` and the exit code of a refused command line,
2 unless the guide renames the code or lists another exit for it
([refused command lines](../../parsing/README.md)). Its fields are `argument`,
`value` and `most`, the bound as an integer. The refusal's recovery runs the
caller's own words with `--limit N` replaced by `--all`, so the caller's
filters stay. Its alternative runs the same words with `--limit MOST`. A
`--limit` spelled through an alias that the tool adds falls back to the `-h`
recovery.

Every refused `--limit` value states the range that `--limit` takes, in the
same sentence for a value above the bound, 0, a word and a negative number.
On shelf's `books list` the refusal of `--limit 0` reads `invalid value '0'
for '--limit <N>': --limit takes a whole number from 1 to 100`. A page
without a bound says `--limit takes a whole number of at least 1`, or names
`usize::MAX` as the top of the range for a number too large for `usize`. On
a page with a bound, a number too large for `usize` counts as above the
bound and gets the `--all` recovery. Any other value below the range or
that is not a whole number keeps the `-h` recovery of a refused command
line. clap reads `--limit -1` as the flag `-1` and refuses it as an
unexpected argument, so only `--limit=-1` reaches the range sentence.

The help of a bounded `--limit` reads `Print at most this many rows, up to
100`, and the page's JSON Schema gives the limit a `maximum`. A listing built
with `Page::first_of` or `Page::listing` carries the bound, and its footer
reads `Showing 4 of 6 books. --limit takes up to 100, and --all shows every
one.` A listing built with `Listing::from_all` or `Listing::new` has no bound
and keeps the footer `Showing 4 of 6 books. --all shows every one.`

Why: ADR-0050.

## Rows that report a problem

A row type names the field that marks a problem with
[`Row::PROBLEM`](../../../../crates/crumbkit/src/row.rs), whose rustdoc
says which values report a problem. crumbkit-aliases' `AliasRow` names its
`problem` column, so `aliases list` exits with 1 while an entry of the
aliases file does not work.

- A run whose rows report a problem and that has no failure exits with 1, in
  `finish` for every result with rows and in a stream for the items it
  printed. A run with a failure exits as its first failure decides
  ([exit codes](../../failures/exit-codes.md)).
- A reader that closed the pipe early does not change the exit.
- The exit reads every row that the result holds, including rows that the
  output budget left unprinted. A listing from `Listing::from_all` or
  `Page::first_of` also counts the rows that its limit cut, so the exit and
  the note include a problem row that the page does not show. A listing
  from `Listing::new`, `Page::listing` or `Listing::uncounted` holds one
  page that the tool fetched, so crumbkit counts only that page's rows.
  OUTPUT-3 puts the rows that report a problem first, so such a page from a
  source with a problem holds at least one of them.
- In text, a note on standard error counts the rows after the footer or empty
  note and before the tool's note:
  `note: 2 of the aliases report a problem, so the command exits with 1.` A
  run with a failure prints no such note.
- With `--json`, each row's own field holds the problem, so nothing new
  prints.

The [help pages](../../help/README.md) describe the `Problems:` line and what
`help exit-codes` says about this exit.

Why: ADR-0050.
