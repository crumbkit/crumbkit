# Changelog

This file records the notable changes to the published crumbkit crates. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the crates follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Row` marks a type that prints as one table row and names its compact
  columns and the columns that `--long` adds.
- `Document` is the typed id of a JSON document, such as
  `shelf/books-list/v1`, and `Document::new` rejects a malformed id at compile
  time.
- `Output` adds the global flags `--json`, `--long`, `--full` and `--fields`
  to a tool's clap parser.
- `finish` takes the tool's `Spelling` and the handler's `Result`, and
  returns the exit code. It prints an `Ok` result as a text table or as a
  JSON document with `schema` and `data`. It prints an `Err` as a failure on
  standard error, with an `error:` line, a `cause:` line for each error in
  its source chain, and `help:`, `code:` and `url:` lines, and with `--json`
  as one `crumbkit/failure/v1` document on standard output. A failure exits
  with code 1. A `Recovery::Run` call that does not resolve, or that has no
  command to resolve against, prints as `None` with the reason `the
  suggested command is not available`. `finish_to` does the same into a
  `Sink` over any two writers.
- `Page` adds the flags `--limit <N>` and `--all` to one command. Its const
  parameter sets the default limit, which is 25 unless the command names
  another. `--limit` parses as a `NonZeroUsize`, so clap refuses 0.
- `Listing` holds one page of rows and the total number of rows that matched.
  `Listing::new` takes a page and a total that the tool counted, and raises
  a total smaller than the page to the number of rows.
  `Listing::from_all` keeps the first page of rows that the tool already
  holds in memory.
- A listing's JSON document holds the total under `total`, beside `data`.
  When a listing shows fewer rows than its total, the text table ends with a
  footer such as `Showing 4 of 6 books. --all shows every one.`
- `Call` names a command by its path from the root and the clap ids of the
  arguments to show, and `Call::render` turns it into text, such as
  `shelf books show <ID>...`, against the `clap::Command` the tool built at
  runtime. `CallError` names the path segment or argument that is missing or
  hidden.
- `Output::set_command` keeps the command tree that the tool parsed against,
  so `finish` can render calls.
- `Listing::next` gives a listing a next step. Its text output ends with a
  `Next:` line after the table and any footer, such as `Next: shelf books
  show <ID>... takes up to 100 ids in one call.` The line is left out when no
  command is set or the call does not resolve, and the JSON document is
  unchanged.
- `Failure` extends `std::error::Error` with a stable `Code`, a `Recovery`
  and an optional url, and `Recovery` names the caller's next step: `Run` a
  `Call`, `Retry`, a `Hint`, or `None` with a reason.
- `Code` holds a failure code's component, area and reason, and
  `Code::new` rejects a malformed segment at compile time. `Spelling` turns a
  code into text, with `Spelling::COLONS`, `DOTS`, `SLASHES` and
  `Spelling::custom`.
- `Error` holds any `Failure`, so `?` converts each of a tool's error types
  into one type. `Error` implements `std::error::Error`, so a tool's `main`
  can pass it on to `Box<dyn std::error::Error>`. `AsFailure` is the sealed
  bound through which `finish` accepts a `Failure` or an `Error`.
- crumbkit-test's `check_codes` fails a test, naming every problem, when two
  of a tool's codes spell the same, or when a spelled code is empty or holds
  a character other than printable ASCII, `!` to `~`.
- `crates/crumbkit/schemas/failure-v1.json` holds the JSON Schema of the
  `crumbkit/failure/v1` document.
