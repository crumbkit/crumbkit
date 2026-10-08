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
- `finish` prints a result as a text table or as a JSON document with
  `schema` and `data`, and returns the exit code. `finish_to` does the same
  into a `Sink` over any two writers.
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
