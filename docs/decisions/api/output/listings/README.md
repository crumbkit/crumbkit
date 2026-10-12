# ADR-0050. Listings

These decisions cover where the lines that describe a result print, what a listing prints when it has no rows, and the upper bound that a tool can set on `--limit`.

## Notes on standard error

- In text, the lines that describe a result print on standard error, and standard output holds the result alone. These lines are a listing's footer, its empty note, the tool's note, the `Next:` line, the output budget's cut line (ADR-0046), the dry-run line (ADR-0039) and a stream's notes (ADR-0041). A pipe such as `| wc -l`, `| grep` or `| jq` therefore sees only data.
- Coding agents commonly return standard error to the model in the same stream as standard output. When an agent filters standard output with `| head` or `| grep`, a note on standard error passes the filter, while a footer on standard output can be cut away.
- A listing's summary line stays on standard output above the table, because it is part of the result (ADR-0016).
- crumbkit flushes standard output before it writes the notes, and it writes all of a result's notes to standard error in one write ([`write_result`](../../../../../crates/crumbkit/src/output/result.rs)). A reader that merges the two streams, such as a terminal or `2>&1`, therefore sees the notes after the rows, in one piece.
- With `--json`, the document holds what the notes say, so no note prints. The exceptions are the lines that a document without the envelope has no field for, such as the cut line and the dry-run line, which print on standard error.

## A listing with no rows

- A listing whose total is 0 prints its header on standard output and a note on standard error in text: `note: no books match`. The default wording is `no {description} match`, from `Listing::described`, or `no rows match` for a listing without a description ([`write_empty_note`](../../../../../crates/crumbkit/src/output/listing.rs)).
- `Document::when_empty` sets the document's own wording, such as `the list has no entries` ([`Document`](../../../../../crates/crumbkit-core/src/document.rs)). The wording is one line. `self usage` prints `note: the usage log holds no findings`.
- On standard output, the text order is the summary and the header. On standard error, it is the empty note, the tool's own note, the `Next:` line and the dry-run line.
- A page that holds no rows from a total above 0 prints the footer in place of the note. A listing that the output budget cut to no rows prints the cut line and no note (ADR-0046).
- With `--json`, a listing's document is unchanged: `data` is `[]` and `total` is 0. A listing printed without the envelope prints `[]`.
- A stream that ends with `Ok`, printed no rows and no failure, and still has an open pipe prints the note on standard error, in text and with `--json`. Its default wording is `no rows`, and `Document::when_empty` sets its own ([`Streaming::end`](../../../../../crates/crumbkit/src/output/stream.rs)).
- A list of rows, such as a `Vec`, prints no note, because it has no total and no description to word one from.
- The usage log marks a run whose listing or stream printed the note with `"empty": true` (ADR-0029).

## An upper bound on `--limit`

- `Page<N, MOST>` takes an optional second constant, the most rows that `--limit` takes ([`Page`](../../../../../crates/crumbkit/src/page.rs)). It defaults to `usize::MAX`, which means no bound. The compiler refuses a page whose default `N` is above `MOST`. shelf's `books list` uses `Page<4, 100>`.
- clap refuses a larger value as it parses the command line. The refusal has the code `crumbkit::cli::usage` and exit code 2, as every refused command line does (ADR-0001). Its fields are `argument`, `value` and `most`, the bound as an integer.
- The refusal's recovery runs the caller's own words with `--limit N` replaced by `--all`, so the caller's filters stay. Its alternative runs the same words with `--limit MOST`. A `--limit` spelled through an alias that the tool adds falls back to the `-h` recovery.
- The help of a bounded `--limit` reads `Print at most this many rows, up to 100`, and the page's JSON Schema gives the limit a `maximum`.
- `Page::first_of(rows)` and `Page::listing(rows, total)` build a listing that carries the bound. Its footer then reads `Showing 4 of 6 books. --limit takes up to 100, and --all shows every one.` A listing built with `Listing::from_all` or `Listing::new` has no bound and keeps the footer `Showing 4 of 6 books. --all shows every one.`

Considered: notes on standard output after the result, a note on standard error only when standard output is not a terminal, the summary line on standard error, a `note` field in the JSON document, a line in a `--json` stream for the note, a note for a `Vec`, an empty note in the help of each leaf, a new usage record id for `empty`, a bound checked in the handler after parsing, a refusal code of its own for the bound, a recovery that runs `--limit MOST` first, a bound as a runtime setting of `Output`, a `--limit` that clamps a larger value to the bound without a refusal.
