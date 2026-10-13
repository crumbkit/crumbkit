# ADR-0050. Listings

These decisions cover where the lines that describe a result print, what a listing prints when it has no rows, what a listing prints when the tool did not count its source, the upper bound that a tool can set on `--limit`, and the exit of a result whose rows report a problem.

## Notes on standard error

- In text, the lines that describe a result print on standard error, and standard output holds the result alone. These lines are a listing's footer, its empty note, the note of the rows that report a problem, the tool's note, the `Next:` line, the output budget's cut line (ADR-0046), the dry-run line (ADR-0039) and a stream's notes (ADR-0041). A pipe such as `| wc -l`, `| grep` or `| jq` therefore sees only data.
- Coding agents commonly return standard error to the model in the same stream as standard output. When an agent filters standard output with `| head` or `| grep`, a note on standard error passes the filter, while a footer on standard output can be cut away.
- A listing's summary line stays on standard output above the table, because it is part of the result (ADR-0016).
- crumbkit flushes standard output before it writes the notes, and it writes all of a result's notes to standard error in one write ([`write_result`](../../../../../crates/crumbkit/src/output/result.rs)). A reader that merges the two streams, such as a terminal or `2>&1`, therefore sees the notes after the rows, in one piece.
- With `--json`, the document holds what the notes say, so no note prints. The exceptions are the lines that a document without the envelope has no field for, such as the cut line and the dry-run line, which print on standard error.

## A listing with no rows

- A listing whose total is 0 prints its header on standard output and a note on standard error in text: `note: no books match`. The default wording is `no {description} match`, from `Listing::described`, or `no rows match` for a listing without a description ([`write_empty_note`](../../../../../crates/crumbkit/src/output/listing.rs)).
- `Document::when_empty` sets the document's own wording, such as `the list has no entries` ([`Document`](../../../../../crates/crumbkit-core/src/document.rs)). The wording is one line. `self usage` prints `note: the usage log holds no findings`.
- On standard output, the text order is the summary and the header. On standard error, it is the empty note, the tool's own note, the `Next:` line and the dry-run line.
- A page that holds no rows from a total above 0 prints the footer in place of the note. A listing that the output budget cut to no rows prints the cut line and no note (ADR-0046).
- With `--json`, a listing's document is unchanged: `data` is `[]`, `total` is 0 and `truncated` is false. A listing printed without the envelope prints `[]`.
- A stream that ends with `Ok`, printed no rows and no failure, and still has an open pipe prints the note on standard error, in text and with `--json`. Its default wording is `no rows`, and `Document::when_empty` sets its own ([`Streaming::end`](../../../../../crates/crumbkit/src/output/stream.rs)).
- A list of rows, such as a `Vec`, prints no note, because it has no total and no description to word one from.
- The usage log marks a run whose listing or stream printed the note with `"empty": true` (ADR-0029).

## A listing without a total

- A tool whose source cannot be counted cheaply, such as a remote API that pages its answers or a log that grows while it is read, builds its listing with `Listing::uncounted(rows, more)`, where `more` says whether the source holds rows after the page ([`Listing`](../../../../../crates/crumbkit-core/src/listing.rs)). `Page::uncounted(rows)` takes an iterator or a `Vec`, reads at most one row past the limit and sets `more` from that row, so a lazy source stops there. The reading example's `entries list` reads its file line by line this way. `Page::fetch_limit` gives a query the number of rows to fetch, which is one past the limit ([`Page`](../../../../../crates/crumbkit/src/page.rs)).
- While more rows exist, the JSON document leaves out `total`. Every listing's JSON Schema therefore marks `total` as optional (ADR-0007). A listing without more rows holds every row, so its total is the number of rows, and it prints as the listing that `Listing::new` builds with that total.
- The footer of a listing without a total gives no count and keeps `--all` as the next step: `Showing 4 books, and more match. --all shows every one.` It names the bound when the page has one, and says `rows` when the listing has no description. The output budget's cut line of such a listing also gives no count, as in `Showing 812 books, because output stops at 1048576 bytes.`
- A page with no rows and more rows after it prints the footer and no empty note, as a counted page with no rows from a total above 0 does.
- Every listing's document holds `truncated`, which is true when `--limit`, the output budget or an uncounted source left rows out, and false when `data` holds every row. `cut` says that the budget was the cause and names the budget (ADR-0046), so a listing with `cut` also holds `"truncated": true`. A list has no `truncated`, because only the budget shortens a list, and `cut` marks that.
- The usage log marks a run whose listing left rows out with `"truncated": true` (ADR-0029).

## An upper bound on `--limit`

- `Page<N, MOST>` takes an optional second constant, the most rows that `--limit` takes ([`Page`](../../../../../crates/crumbkit/src/page.rs)). It defaults to `usize::MAX`, which means no bound. The compiler refuses a page whose default `N` is above `MOST`. shelf's `books list` uses `Page<4, 100>`.
- clap refuses a larger value as it parses the command line. The refusal has the code `crumbkit::cli::usage` and the exit code 2, or the guide's rename and exit for them, as every refused command line does (ADR-0001, ADR-0056). Its fields are `argument`, `value` and `most`, the bound as an integer.
- The refusal's recovery runs the caller's own words with `--limit N` replaced by `--all`, so the caller's filters stay. Its alternative runs the same words with `--limit MOST`. A `--limit` spelled through an alias that the tool adds falls back to the `-h` recovery.
- The help of a bounded `--limit` reads `Print at most this many rows, up to 100`, and the page's JSON Schema gives the limit a `maximum`.
- `Page::first_of(rows)` and `Page::listing(rows, total)` build a listing that carries the bound. Its footer then reads `Showing 4 of 6 books. --limit takes up to 100, and --all shows every one.` A listing built with `Listing::from_all` or `Listing::new` has no bound and keeps the footer `Showing 4 of 6 books. --all shows every one.`

## Rows that report a problem

- A row type names the field that marks a problem with `Row::PROBLEM` ([`Row`](../../../../../crates/crumbkit-core/src/row.rs)). A row reports a problem when that field serializes to a value other than null, false or an empty string. crumbkit-aliases' `AliasRow` names its `problem` column, so `aliases list` exits with 1 while an entry of the aliases file does not work (ADR-0027).
- A run whose rows report a problem and that has no failure exits with 1, in `finish` for every result with rows and in a stream for the items it printed. A run with a failure exits as its first failure decides (ADR-0037). A reader that closed the pipe early does not change the exit. A CI job or an agent can then gate on the exit code alone, without parsing the rows.
- [`exit::problem_rows`](../../../../../crates/crumbkit/src/exit.rs) is the one place that chooses that exit code, and the note and the `Problems:` line read it there.
- The exit reads every row that the result holds, including rows that the output budget left unprinted, because the handler already loaded them. A listing holds one page, and OUTPUT-3 puts the rows that report a problem first, so a page from a source with a problem holds at least one of them.
- In text, a note on standard error counts the rows after the footer or empty note and before the tool's note: `note: 2 of the aliases report a problem, so the command exits with 1.` A run with a failure prints no such note. With `--json`, each row's own field holds the problem, so nothing new prints.
- The `-h` and `--help` of a leaf whose row type names the field print a `Problems:` line after the `Output:` line, and `help exit-codes` says that 1 also means a row reported a problem. crumbkit-test's `check_guide` fails when the field is not one of `COLUMNS`, because the compact table would hide the problem, and when its type has no value that means no problem.

Considered: notes on standard output after the result, a note on standard error only when standard output is not a terminal, the summary line on standard error, a `note` field in the JSON document, a line in a `--json` stream for the note, a note for a `Vec`, an empty note in the help of each leaf, a new usage record id for `empty`, a bound checked in the handler after parsing, a refusal code of its own for the bound, a recovery that runs `--limit MOST` first, a bound as a runtime setting of `Output`, a `--limit` that clamps a larger value to the bound without a refusal, a listing without a total that leaves out `total` on its last page too, a footer that reads `Showing 4 of at least 5 books`, `truncated` on a list, `truncated` in place of `cut`, an uncounted listing that only takes a `Vec`, a problem method on `Row` that returns a code, an exit number that a guide's `Exit` claims for problem rows, a problem exit without a note, examples that may exit with 1 for a problem.
