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
- crumbkit: with the `miette` feature, the JSON failure document holds the
  diagnostic's related diagnostics and a source excerpt for each label.
- crumbkit: [`parse`] parses the command line, prints help and refusals,
  and never exits the process.
- crumbkit: [`help`] adds goal headings, help topics, and leaf examples and
  columns to help pages ([ADR-0021], [ADR-0023], [ADR-0024]).
- crumbkit: [`commands`] gives a tool the `commands list` command
  ([ADR-0026]).
- crumbkit: [`help::quote`] quotes one word for a POSIX shell.
- crumbkit: the [schemas] directory holds the JSON Schemas of the
  `crumbkit/failure/v1`, `crumbkit/commands-list/v1` and `crumbkit/usage/v1`
  documents ([ADR-0007]).
- crumbkit: the `usage-log` feature appends one line per run to a local
  usage log ([`usage`], [ADR-0029]). A run joins its caller's W3C trace,
  and [`trace::child_env`] passes the trace to a child process
  ([ADR-0030]).
- crumbkit: the `tracing` feature runs `parse` and `finish` inside spans
  ([`trace`]).
- crumbkit: with the `usage-log` feature, `self usage` reports where
  callers waste effort from the usage log, and a [`Listing`] can carry a
  summary that prints above its table ([ADR-0016], [ADR-0034]).
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
- cargo-crumbkit: the new crate's `cargo crumbkit weigh --bin <NAME>`
  prints the bytes of each command's help pages and example output, and
  each path's total ([cargo-crumbkit], [ADR-0033]). crumbkit-test's
  [`example_golden_name`] names each example's golden file.
- crumbkit: [`schemas`] gives a tool `schemas list`, which lists the JSON
  documents it prints, and `schemas show <ID>...`, which prints the JSON
  Schema of each ([ADR-0035]).
- crumbkit: [`Document`] takes a one-line description, and each document's
  name is the path of the command that prints it, such as
  `crumbkit/commands-list/v1` ([ADR-0007]).
- crumbkit-test: [`check_schema_goldens`] pins the JSON Schema of each
  document a tool prints, [`check_document_ids`] checks each document's
  name, and [`check_mounts`] checks the paths of crumbkit's commands
  ([ADR-0035]).
- crumbkit: with the `usage-log` feature, `self figures list` prints the
  figures of every command in the usage log, and `self usage --full` adds
  `help_before` to each command's figures ([`usage`], [ADR-0034]).
- crumbkit: `self usage` and `self figures list` take `--log <FILE>` to
  read a copy of a usage log ([ADR-0034]).
- crumbkit-test: [`check_path_weights`] budgets the bytes of each path
  through a tool's help, 8,000 by default, and [`path_totals`] and
  [`output_weight`] compute them ([ADR-0025]).
- cargo-crumbkit: `weigh --usage`, `--log` and `--since` add each
  command's runs, help pages read before it and 95th percentile output
  from a usage log ([cargo-crumbkit], [ADR-0033]).
- crumbkit: `Output::is_json` says whether the caller asked for JSON, and
  `Output::progress_allowed` says whether a tool may draw progress
  ([`Output`], [ADR-0016]).
- crumbkit: [`usage::ended`] records a tool's own exit number and output
  bytes for a run that never reaches `finish` ([ADR-0029]).
- crumbkit: `usage::start_with(Signals::Tool)` lets a tool handle SIGINT
  and SIGTERM itself while the usage log records the signal ([`usage`],
  [ADR-0029]).
- crumbkit: [`Failure`] has `alternatives`, other repairs that print as
  extra `help:` lines and under `alternatives` in the failure document
  ([ADR-0001]).
- crumbkit: `Call::value` adds an argument with a concrete value, such as
  `--author 'Le Guin'`, and rendering checks it against the tool's command
  tree ([`Call`], [ADR-0001]).
- crumbkit: [`Prose`] holds a command's answer as text with a typed value
  and optional failures. Text output prints the text, and `--json` prints
  the value under `data` and the failures under `failures` ([ADR-0016]).
- crumbkit: [`help::RuntimeLeaf`] and `Guide::with_runtime_leaves` give
  examples to commands that a tool adds at run time ([ADR-0021]).

- crumbkit-test: [`check_opt_outs`] pins a tool's opt-outs and budgets,
  with each reason and what it costs an agent, in a text and a JSON golden
  file ([ADR-0036]).

### Changed

- crumbkit: a guided command's help prints the tool's own `before_help`
  and `before_long_help` above the goal list ([ADR-0021]).
- crumbkit-test: [`Examples`] borrows its paths for a lifetime, so a test
  can build one from a path it computes at runtime.
- crumbkit: each argument row of `commands list` has a `one_of` field that
  names the arguments that can be given in its place: the other members of
  its required groups and, for a required argument, the arguments it
  conflicts with ([`commands`]).
- crumbkit-test: [`check_listings`] accepts a `show` command whose ids sit
  in a required argument group.
- cargo-crumbkit: `weigh` prints `long_help` only with `--long`
  ([ADR-0033]).
- crumbkit: [`finish`], `finish_to`, `Output::command`, `commands::list`,
  `schemas::list`, `schemas::show`, `usage::findings` and `usage::figures`
  take `&mut Output`, and `Output::set_command` no longer builds the command
  tree, so a run builds only the commands it reads ([ADR-0015]).
- crumbkit-aliases: `list` takes `&mut Output`, because it builds the
  tool's command tree when it runs.
- crumbkit: [`parse`] returns a [`Stop`] after help, the version or a
  refusal, which holds the exit code and an `Output` ([ADR-0002]).
- crumbkit-test: [`check_grammar`], [`check_names`], [`check_listings`],
  [`check_filters`] and [`Examples`] take opt-outs, each an [`Allow`] that
  names one rule, a command and a reason, in place of exempt command paths
  ([ADR-0036]).
- crumbkit-test: [`check_grammar`] passes a leaf directly under the root,
  a resource name or a command's own `--dry-run` that an opt-out names
  ([ADR-0015]).
- crumbkit-test: [`Budgets`] and [`PathBudgets`] take a reason with each
  budget, a command's budget is a [`Budget`], and a tool can raise a
  budget above crumbkit's default ([ADR-0025]).
- crumbkit: a [`Document`] component may hold dots and dashes, so a
  domain such as `example.dev` names a document's owner ([ADR-0007]).
- crumbkit-test: [`check_document_ids`] takes opt-outs from the document
  name rule, and [`check_schema_goldens`] fails on two ids whose golden
  files share a name ([ADR-0035]).

### Removed

- crumbkit: `Call::word` is gone, because `Call::value` adds a runtime value
  and checks it against the tool's command tree ([`Call`]).

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
- crumbkit: a refusal from [`parse`] leaves out each command or flag that
  clap suggests and the tool hid, and leaves out clap's tip when nothing
  else remains.
- crumbkit: the usage log's `nearest` records the real flag when clap
  suggests a hidden alias, and records nothing when clap suggests a hidden
  flag ([`usage`]).
- crumbkit: the rustdoc builds without warnings when the `usage-log`
  feature is off, because it no longer links to `usage::start`, which only
  that feature defines.
- crumbkit-test: [`check_names`] skips hidden commands, everything below
  them and hidden arguments, because no help page shows them.
- crumbkit: a refusal from [`parse`] prints as JSON only when the walk reads
  `--json` as a flag, so an option's value of `--json` no longer counts
  ([ADR-0021]).

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
[`Prose`]: crates/crumbkit-core/src/prose.rs
[`fetch_each`]: crates/crumbkit/src/fetch.rs
[`report`]: crates/crumbkit-core/src/report.rs
[`parse`]: crates/crumbkit/src/parse.rs
[`Stop`]: crates/crumbkit/src/parse/stop.rs
[`usage::ended`]: crates/crumbkit/src/usage.rs
[`help`]: crates/crumbkit/src/help.rs
[`help::quote`]: crates/crumbkit/src/help.rs
[`help::RuntimeLeaf`]: crates/crumbkit/src/help/runtime.rs
[`commands`]: crates/crumbkit/src/commands.rs
[`schemas`]: crates/crumbkit/src/schemas.rs
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
[ADR-0002]: docs/decisions/api/0002-public-api-and-semver.md
[ADR-0004]: docs/decisions/testing/0004-golden-and-snapshot-tests.md
[ADR-0007]: docs/decisions/api/0007-json-schemas.md
[ADR-0015]: docs/decisions/api/0015-lightweight-adoption.md
[ADR-0016]: docs/decisions/api/output/0016-result-formats.md
[ADR-0021]: docs/decisions/api/help/0021-help-pages.md
[ADR-0023]: docs/decisions/api/help/0023-help-command-and-topics.md
[ADR-0024]: docs/decisions/api/help/0024-leaf-examples-and-columns.md
[ADR-0025]: docs/decisions/api/help/0025-help-checks.md
[ADR-0026]: docs/decisions/api/help/0026-branch-export.md
[ADR-0027]: docs/decisions/api/0027-aliases.md
[ADR-0029]: docs/decisions/api/usage/0029-usage-log.md
[ADR-0030]: docs/decisions/api/usage/0030-trace-context.md
[ADR-0032]: docs/decisions/api/0032-core-crate.md
[cargo-crumbkit]: crates/cargo-crumbkit/README.md
[`example_golden_name`]: crates/crumbkit-test/src/examples.rs
[ADR-0033]: docs/decisions/api/help/0033-weigh.md
[ADR-0034]: docs/decisions/api/usage/0034-usage-report.md
[ADR-0035]: docs/decisions/api/0035-schema-commands.md
[`check_schema_goldens`]: crates/crumbkit-test/src/schemas.rs
[`check_document_ids`]: crates/crumbkit-test/src/documents.rs
[`check_mounts`]: crates/crumbkit-test/src/documents.rs
[`check_path_weights`]: crates/crumbkit-test/src/weights.rs
[`path_totals`]: crates/crumbkit-test/src/weights.rs
[`output_weight`]: crates/crumbkit-test/src/weights.rs
[`Allow`]: crates/crumbkit-test/src/allow.rs
[`check_opt_outs`]: crates/crumbkit-test/src/opt_outs.rs
[`Budget`]: crates/crumbkit-test/src/budget.rs
[`Budgets`]: crates/crumbkit-test/src/sizes.rs
[`PathBudgets`]: crates/crumbkit-test/src/weights.rs
[ADR-0036]: docs/decisions/api/0036-opt-outs.md
