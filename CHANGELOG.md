# Changelog

This file records the notable changes to the published crumbkit crates. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the crates follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- crumbkit: [`Row`] marks a type that prints as one table row.
- crumbkit: [`Document`] is the typed id of a JSON document.
- crumbkit: [`Output`] adds the global flags `--json`, `--long`, `--full`
  and `--fields`, and [`finish`] prints a handler's result and returns the
  exit code.
- crumbkit: [`Page`] adds `--limit` and `--all` to one command.
- crumbkit: [`Listing`] holds one page of rows and the total, with an
  optional footer, note and next step.
- crumbkit: [`Call`] names a command by its path and renders it against the
  tool's command tree.
- crumbkit: [`Failure`], [`Recovery`], [`Code`], [`Spelling`] and [`Error`]
  form the failure model ([ADR-0001]).
- crumbkit: [`Batch`] holds the rows and the failures of a per-id command,
  and [`fetch_each`] runs the per-id lookups on scoped threads.
- crumbkit: the `miette` feature adds `Failure::diagnostic` and converts a
  `miette::Report` into [`Error`] ([`report`]).
- crumbkit: [`parse`] parses the command line, prints help and refusals,
  and never exits the process.
- crumbkit: [`help`] adds goal headings, help topics, and leaf examples and
  columns to help pages ([ADR-0021], [ADR-0023], [ADR-0024]).
- crumbkit: [`commands`] gives a tool the `commands list` command
  ([ADR-0026]).
- crumbkit: [`help::quote`] quotes one word for a POSIX shell.
- crumbkit: the [schemas] directory holds the JSON Schemas of the
  `crumbkit/failure/v1`, `crumbkit/commands/v1` and `crumbkit/usage/v1`
  documents ([ADR-0007]).
- crumbkit: the `usage-log` feature appends one line per run to a local
  usage log ([`usage`], [ADR-0029]). A run joins its caller's W3C trace,
  and [`trace::child_env`] passes the trace to a child process
  ([ADR-0030]).
- crumbkit: the `tracing` feature runs `parse` and `finish` inside spans
  ([`trace`]).
- crumbkit-test: the checks [`check_codes`], [`check_names`],
  [`check_grammar`], [`check_listings`], [`check_filters`],
  [`check_guide`] and [`check_help_sizes`] fail a test on a tool that
  breaks crumbkit's conventions ([ADR-0025]).
- crumbkit-test: [`check_help_goldens`] and [`check_examples`] compare help
  pages and example output with golden files ([ADR-0004]).
- crumbkit-test: [`check_guide`] fails on two groups with one path.
- crumbkit-aliases: the new crate expands a person's command aliases and
  lists them ([crumbkit-aliases], [ADR-0027]).
- crumbkit-core: the new crate holds the types that need no clap, and
  crumbkit re-exports each at its old path. `Call::render` comes from
  crumbkit's [`RenderCall`] trait ([crumbkit-core], [ADR-0032]).

### Changed

- crumbkit-test: [`Examples`] borrows its paths for a lifetime, so a test
  can build one from a path it computes at runtime.

### Fixed

- crumbkit: [`parse`] keeps a command's own `after_help` and
  `after_long_help`, and prints a leaf's examples once.
- crumbkit: `commands list <PATH>` lists the global arguments of the
  commands above the branch ([`commands`]).
- crumbkit-test: [`check_help_goldens`] fails on two pages whose golden
  files have one name.
- crumbkit-test: [`check_guide`] reports a goal that names one subcommand
  twice.
- crumbkit-test: [`check_guide`] and [`check_examples`] quote an example's
  words as help prints them.
- crumbkit-test: [`check_examples`] reports failed runs, and a binary that
  cannot start, under their own heading.
- crumbkit-test: [`check_help_sizes`] reports a page that fails to render as
  one problem ([ADR-0025]).
- crumbkit: a trim of the usage log keeps the newest whole lines within a
  size budget, drops a torn line, and replaces the log in one step under a
  separate lock file ([`usage`], [ADR-0029]).
- crumbkit: a usage-log key file goes into place only when complete, and a
  key of the wrong length is replaced ([ADR-0029]).
- crumbkit: a signal or the end of `main` during the usage log's write
  waits a bounded time for the line to finish ([ADR-0029]).
- crumbkit: `--fields` and a leaf's columns accept the fields of each
  variant of an enum row type ([`Row`]).
- crumbkit: [`parse`] skips each option's values when it walks the typed
  words, so a value never names a subcommand.
- crumbkit: [`parse`] treats a subcommand's global flags as it treats the
  root's on `-h` and the `Global flags:` line.
- crumbkit: when a lookup of [`fetch_each`] panics, the other workers stop
  before they take another id.
- crumbkit-aliases: `aliases list` says that an entry named `help` loses to
  the tool's own help ([crumbkit-aliases]).
- crumbkit: a refusal from [`parse`] names the real flag or subcommand
  where clap suggests a hidden alias, and names it once.
- crumbkit-test: [`check_names`] skips hidden commands and everything
  below them, because no help page shows them.

<!-- Each label below points at the source file that holds the item's
rustdoc, or at its decision record. -->

[`Row`]: crates/crumbkit-core/src/row.rs
[`Document`]: crates/crumbkit-core/src/document.rs
[`Output`]: crates/crumbkit/src/output.rs
[`finish`]: crates/crumbkit/src/output.rs
[`Page`]: crates/crumbkit/src/page.rs
[`Listing`]: crates/crumbkit-core/src/listing.rs
[`Call`]: crates/crumbkit-core/src/call.rs
[`Failure`]: crates/crumbkit-core/src/failure.rs
[`Recovery`]: crates/crumbkit-core/src/failure.rs
[`Code`]: crates/crumbkit-core/src/code.rs
[`Spelling`]: crates/crumbkit-core/src/code.rs
[`Error`]: crates/crumbkit-core/src/failure.rs
[`Batch`]: crates/crumbkit-core/src/batch.rs
[`fetch_each`]: crates/crumbkit/src/fetch.rs
[`report`]: crates/crumbkit-core/src/report.rs
[`parse`]: crates/crumbkit/src/parse.rs
[`help`]: crates/crumbkit/src/help.rs
[`help::quote`]: crates/crumbkit/src/help.rs
[`commands`]: crates/crumbkit/src/commands.rs
[`usage`]: crates/crumbkit/src/usage.rs
[`trace`]: crates/crumbkit/src/trace.rs
[`trace::child_env`]: crates/crumbkit/src/trace.rs
[schemas]: crates/crumbkit/schemas
[`check_codes`]: crates/crumbkit-test/src/codes.rs
[`check_names`]: crates/crumbkit-test/src/names.rs
[`check_grammar`]: crates/crumbkit-test/src/grammar.rs
[`check_listings`]: crates/crumbkit-test/src/listings.rs
[`check_filters`]: crates/crumbkit-test/src/filters.rs
[`check_guide`]: crates/crumbkit-test/src/guide.rs
[`check_help_sizes`]: crates/crumbkit-test/src/sizes.rs
[`check_help_goldens`]: crates/crumbkit-test/src/goldens.rs
[`check_examples`]: crates/crumbkit-test/src/examples.rs
[`Examples`]: crates/crumbkit-test/src/examples.rs
[crumbkit-aliases]: crates/crumbkit-aliases/README.md
[crumbkit-core]: crates/crumbkit-core/README.md
[`RenderCall`]: crates/crumbkit/src/call.rs
[ADR-0001]: docs/decisions/api/0001-error-model.md
[ADR-0004]: docs/decisions/testing/0004-golden-and-snapshot-tests.md
[ADR-0007]: docs/decisions/api/0007-json-schemas.md
[ADR-0021]: docs/decisions/api/help/0021-help-pages.md
[ADR-0023]: docs/decisions/api/help/0023-help-command-and-topics.md
[ADR-0024]: docs/decisions/api/help/0024-leaf-examples-and-columns.md
[ADR-0025]: docs/decisions/api/help/0025-help-checks.md
[ADR-0026]: docs/decisions/api/help/0026-branch-export.md
[ADR-0027]: docs/decisions/api/0027-aliases.md
[ADR-0029]: docs/decisions/api/usage/0029-usage-log.md
[ADR-0030]: docs/decisions/api/usage/0030-trace-context.md
[ADR-0032]: docs/decisions/api/0032-core-crate.md
