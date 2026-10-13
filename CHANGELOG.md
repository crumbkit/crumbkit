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
- crumbkit: `Guide::documents` declares [`help::FileFormat`]s, the formats
  of files that users write for a tool, and `schemas list` and
  `schemas show` serve their schemas ([ADR-0035]).
- crumbkit: `Document::without_envelope` marks a document that keeps an
  existing shape. With `--json`, [`finish`] prints its result alone and its
  partial failures as text on standard error ([ADR-0016]).

- crumbkit-test: [`check_opt_outs`] pins a tool's opt-outs and budgets,
  with each reason and what it costs an agent, in a text and a JSON golden
  file ([ADR-0036]).
- crumbkit: `Guide::exits` declares a tool's own exit codes, each an
  [`help::Exit`] with a meaning and optional commands, and the built-in
  `help exit-codes` topic lists crumbkit's codes and the tool's
  ([ADR-0037]).
- crumbkit: each `commands list` row has `exits`, the declared codes that
  the command can return, and the text output ends with a note that names
  every code ([ADR-0026]).
- crumbkit-test: [`check_exit_codes`] rejects a declared exit code that
  crumbkit owns or that a shell reports, a repeated code, a meaning that is
  not one line, and a command path that names no command ([ADR-0037]).
- crumbkit-test: [`check_guide`] rejects a topic named `exit-codes`, which
  crumbkit's own topic replaces ([ADR-0023]).
- crumbkit: `Guide::root` names the command that callers run first, such as
  `crumbkit` under `cargo`, which answers `help` with the topics; cargo
  crumbkit sets it, and [`check_guide`] rejects a root that names no
  command ([ADR-0037]).
- crumbkit: [`usage::tolerated`] records the code of a failure that a run
  carried on past, and the usage record lists up to 8 such codes in
  `tolerated` ([ADR-0029]).
- crumbkit: the `self usage` summary counts the runs that tolerated each
  code, and each command's figures count the runs of each exit code from 3
  with its declared meaning ([ADR-0034]).
- crumbkit: [`Effects`] declares a leaf's effects in the terms of MCP's
  tool annotations with `Leaf::effects`, and help and `commands list` show
  them ([ADR-0039]).
- crumbkit: [`DryRun`] adds `--dry-run`. After `Output::dry_run`, the JSON
  document holds `dry_run: true` and the text ends with a dry-run line
  ([ADR-0039]).
- crumbkit: [`Yes`] adds `--yes`. `Output::confirm` asks at a terminal,
  proceeds with `--yes`, and otherwise fails with the typed command plus
  `--yes` as its recovery ([ADR-0039]).
- crumbkit: [`Call`] gains `Call::typed`, which repeats the words that the
  caller typed without a lookup in the command tree.
- crumbkit-test: [`check_effects`] requires effects on every visible leaf,
  `--yes` on destructive leaves, and crumbkit's `--dry-run` in place of a
  tool's own ([ADR-0039]).
- crumbkit-core: [`Stream`] is the result type of a document whose command
  prints rows as they arrive ([ADR-0041]).
- crumbkit: `stream` opens a [`Streaming`], which prints each row as it
  arrives as a table line or a JSON Lines document, and `Leaf::stream`
  declares it in help ([ADR-0041]).
- crumbkit: `commands list` rows and `schemas list` rows hold `form`,
  which is `document` or `stream` ([ADR-0041]).
- crumbkit-test: [`check_examples`] stops an example after 10 seconds and
  checks that each line of a `--json` example is a document with the
  leaf's id ([ADR-0041]).
- crumbkit: the `hand-over` feature adds [`child::hand_over`], which hands
  the terminal to a child process and returns its exit code. On Unix the
  tool ignores SIGINT and SIGQUIT from the moment the child starts until
  the tool ends ([ADR-0042]).
- crumbkit: [`child::hand_over_json`] runs a child that prints JSON under
  `--json` and prints its output as the declared document, so `--fields`,
  `--long` and `--full` apply ([ADR-0042]).
- crumbkit: with the `usage-log` feature, Ctrl-C while a child holds the
  terminal ends only the child, and the run's line records the child's exit
  code ([ADR-0042]).
- crumbkit: `Guide::personal_aliases` makes the root's help name each
  personal alias that expands, and only a run that prints that help reads
  the aliases file ([ADR-0027]).
- crumbkit-aliases: `names` returns the names of the aliases file's entries
  that expand, for `Guide::personal_aliases` ([crumbkit-aliases]).
- crumbkit: `parse` asks the guide's [`Picker`] for a missing required
  argument's values when a person can answer, adds them and prints the run's
  command line on standard error ([ADR-0043]).
- crumbkit: `Leaf::choices` names a source of choices for an argument, and
  `pick::from_rows` turns a command's rows into choices ([ADR-0043]).
- crumbkit: [`NoInput`] adds the global `--no-input`, which stops pickers
  and makes `Output::confirm` fail without asking ([ADR-0043]).
- crumbkit: with the `usage-log` feature, a run's line records `picked`, the
  number of arguments that a picker filled ([ADR-0029]).
- crumbkit-test: [`check_effects`] requires `--no-input` from `NoInput` on a
  tool that can ask a question, unless the tool opts out of
  `Rule::NoInput` ([ADR-0043]).
- crumbkit-test: [`check_guide`] fails on leaf choices that name no
  argument of the leaf.
- crumbkit-picker: the new crate's `Menu` picks values from a numbered list
  that text narrows, using only the standard library ([crumbkit-picker]).
- crumbkit: Windows is a tier-2 platform, which the scheduled lane builds
  and tests ([ADR-0003]).
- crumbkit: the `usage-log` feature builds on Windows, where the log lives
  in `%LOCALAPPDATA%\<tool>`, or under `XDG_STATE_HOME` when it is an
  absolute path ([ADR-0029]).
- crumbkit: on Windows, [`child::hand_over`] keeps the tool running on
  Ctrl-C or Ctrl-Break, and a child that Ctrl-C ended returns 130 and marks
  the line interrupted ([ADR-0042]).
- crumbkit: help on Windows names the tool without `.exe`.
- cargo-crumbkit: `weigh --usage` finds the usage log on Windows
  ([ADR-0033]).
- crumbkit: [`Failure`] has a `fields` method, and the JSON failure
  document holds the named values under `fields` ([ADR-0007]).
- crumbkit: a refused command line or `--fields` name holds the refused
  argument, value or subcommand under `fields` ([ADR-0001]).
- crumbkit: [`Code::runtime`] builds a code known only at run time, and
  `Spelling::split` reads a spelled code back into its segments
  ([ADR-0001]).
- crumbkit-markdown: the new crate's `render` writes a Markdown reference
  of every command of a tool from its `commands list --all --json --full`
  document ([crumbkit-markdown], [ADR-0044]).
- crumbkit: `Effects::line` and `help::STREAM_LINE` are public, so another
  rendering of help prints the effects and stream lines in help's wording
  ([`Effects`], [ADR-0044]).
- crumbkit-test: [`check_command_reference`] fails a test when a tool's
  checked-in Markdown reference differs from the one crumbkit-markdown
  writes from its help, and `SNAPSHOTS=overwrite` rewrites it
  ([ADR-0044]).
- cargo-crumbkit: `cargo crumbkit docs --bin <NAME>` writes the binary's
  Markdown command reference to `docs/commands.md` in its package, or to
  `--out <FILE>` ([cargo-crumbkit], [ADR-0044]).
- crumbkit: [`FailureForm`] and `Guide::failure_form` let a tool print
  every failure in its own JSON document and text, from a
  [`PreparedFailure`] with the code spelled and the recoveries rendered
  ([ADR-0045]).
- crumbkit: `schemas list` serves a failure form's document in place of
  `crumbkit/failure/v1`, as an `unwrapped` document ([ADR-0045]).
- crumbkit-test: [`check_document_ids`] fails on a guide with a failure
  form unless an opt-out from `Rule::FailureForm` names the root
  ([ADR-0045]).
- crumbkit: [`codes`] makes crumbkit's own failure codes public constants
  and lists them in `codes::ALL`, so a tool's test can name them
  ([ADR-0001]).
- crumbkit-test: [`check_recoveries`] runs failing command lines, follows
  each recovery they print and pins the chains in a golden report, and
  [`check_twin`] runs a refused command line beside one that succeeds
  ([ADR-0005], [ADR-0053]).
- crumbkit-test: `Rule::Recoveries` and `Rule::SameCode` let a tool opt out
  of the recovery check ([ADR-0036]).
- crumbkit-test: [`check_recoveries`] fails on a placeholder that a field
  of the failure fills, and shows the fill in the report, unless an opt-out
  from `Rule::FieldValue` names the failing command ([ADR-0053]).
- crumbkit: `Guide::without_terminal_detection(reason)` turns terminal
  detection off, so help has no colour, `Output::progress_allowed` is false,
  and pickers and `Output::confirm` never ask. `get_without_terminal_detection`
  returns the reason ([ADR-0049]).
- crumbkit-test: [`check_opt_outs`] lists a guide without terminal detection
  as `terminal-detection/off`, with the tool's reason and its costs
  ([ADR-0049]).
- crumbkit-test: [`check_recoveries`] takes the tool's guide and fails
  before it runs anything on a guide with a failure form, unless an opt-out
  from `Rule::Recoveries` names the root ([ADR-0045]).
- crumbkit: the `complete` feature adds [`complete`], which answers a
  shell's completion request when `COMPLETE` names the shell, and
  `complete_to`. It follows clap_complete's unstable API, so a minor release
  may change it ([ADR-0048]).
- crumbkit: `Effects::replacing`, `Effects::all_or_none` and
  `Effects::per_item` declare a command's write guards, which help prints
  on a `Writes:` line and `commands list` writes under `writes`
  ([ADR-0047]).
- crumbkit: [`Expect`] adds `--expect <VERSION>`, and a target at another
  version fails with `crumbkit::version::conflict`, whose recovery runs the
  command that reads the target ([ADR-0047]).
- crumbkit-test: [`check_effects`] requires crumbkit's `--expect` on a
  replacing command and `all_or_none` or `per_item` on a write of many
  items, with the opt-outs `Rule::Expect`, `Rule::ExpectFlag` and
  `Rule::Items` ([ADR-0047]).
- crumbkit-markdown: the reference prints a command's `Writes:` line after
  its effects line ([ADR-0047]).

- crumbkit: [`finish`] and [`stream`] print at most an output budget of
  1 MiB on standard output in one run. `Guide::output_budget` sets another
  budget, and `CRUMBKIT_OUTPUT_BUDGET` only lowers it ([ADR-0046]).
- crumbkit: a listing or list past the output budget prints the rows that
  fit and is marked with `cut`. Other results past it fail with
  `crumbkit::output::over_budget` ([ADR-0046]).
- crumbkit-core: [`Document`] gains `always_print_first`, which prints the
  first answer even past the output budget ([ADR-0046]).
- crumbkit-test: [`check_output_budget`] runs each `--json` example with
  half the bytes it prints and checks that the tool cuts or refuses the
  result ([ADR-0046]).
- crumbkit: a listing with no rows, and a stream that ends with none,
  print a note such as `note: no books match`.
  `Document::when_empty` changes the wording ([ADR-0050]).
- crumbkit: `Page<N, MOST>` refuses a `--limit` above `MOST` with a
  recovery that runs `--all`, and the help and the footer name the bound
  ([ADR-0050]).
- crumbkit: with the `usage-log` feature, the line of a run whose listing
  or stream had no rows holds `"empty": true` ([ADR-0029]).
- crumbkit-core: [`Bytes`] is a result that `finish` prints unchanged as
  text, and with `--json` as `text` when it is UTF-8 or `base64` otherwise.
  A `Bytes` row field serializes the same way. `Shape::Bytes` and
  `Rows::bytes` describe it ([ADR-0016], [ADR-0007]).
- crumbkit: `--fields`, `--long` and `--full` are refused with exit code 2
  for a [`Bytes`] result, which has no columns ([ADR-0016]).
- crumbkit: a [`Bytes`] result past the output budget is refused and never
  cut, and `--json` measures it with its base64 text ([ADR-0046]).
- crumbkit: the help of a leaf that prints [`Bytes`] says how its output
  prints, and `help::BYTES_LINE` holds that line ([ADR-0016]).
- crumbkit: [`FailureForm`] gains the provided method `read`, which reads a
  failed run's code and steps into a [`ReadFailure`] and returns `None` by
  default ([ADR-0045]).
- crumbkit: [`FailureForm`] gains the provided methods `entry` and
  `entry_schema`, which write each entry of a batch's or a prose result's
  `failures` and its schema in `schemas show`. The defaults keep crumbkit's
  entry ([ADR-0045]).
- crumbkit-test: the twin, example and recovery checks read each entry of a
  batch's `failures` through the guide's `FailureForm::read`, and read an
  entry that the form cannot read in crumbkit's shape ([ADR-0045]).
- crumbkit: `Guide::json_layout(JsonLayout::Pretty, reason)` prints every
  whole JSON document indented by two spaces, and a stream keeps one
  document per line ([ADR-0052]).
- crumbkit-test: [`check_opt_outs`] lists a guide's JSON layout as
  `json-layout/pretty`, with its reason and cost ([ADR-0052]).
- crumbkit: [`Listing::uncounted`] and [`Page::uncounted`] build a listing
  from a source that the tool does not count. While more rows exist, its
  JSON document leaves out `total` and its footer reads `Showing 4 books,
  and more match.` ([ADR-0050]).
- crumbkit: `Page::fetch_limit` gives the number of rows to fetch for
  [`Page::uncounted`], one past the limit.
- crumbkit-test: `Examples::current_dir` sets the directory that each
  example run starts in, for [`check_examples`], [`check_output_budget`],
  [`check_twin`] and [`check_recoveries`] ([ADR-0005]).
- crumbkit-test: [`check_examples`] gives an example the bytes of its
  `.stdin` file, such as `shelf-books-import.stdin`, on standard input, and
  fails on a `.stdin` file that names no example ([ADR-0005]).
- crumbkit-test: `Examples::redact` replaces each named value with its
  placeholder before [`check_examples`] compares an output, and the check
  replaces its own XDG paths with `[XDG_CONFIG_HOME]` and the like
  ([ADR-0005]).
- crumbkit-test: `Allow::example` opts one example out of
  [`check_examples`] and [`check_output_budget`] by its words ([ADR-0036]).
- crumbkit: re-exports schemars as `crumbkit::schemars`, so a
  `FailureForm` needs no schemars dependency of its own ([ADR-0045]).
- crumbkit-test: `Examples::xdg` and `Failing::xdg` name a fixture that
  the checks copy into each run's fresh XDG directories, so a failing
  command can find a plugin or cache there ([ADR-0005]).
- crumbkit-test: `Examples::fresh_dir` names a fixture that the checks copy
  into a new working directory for each run, or once for each recovery
  chain, and example outputs show the copy's path as `[ROOT]` ([ADR-0005]).
- crumbkit-test: `Failing::values` gives one recovery chain its own
  placeholder values, which win over every other source ([ADR-0053]).
- crumbkit-test: [`check_help_sizes`], [`check_help_goldens`] and
  [`check_path_weights`] take `Pages`, whose `Pages::binary` reads each
  page from the tool's binary, for a tool that builds its help at run
  time. A tool passes the same `Pages` to all three ([ADR-0025]).
- crumbkit: [`ReadFailure`] carries the failure's fields, so a form's
  `read` gives crumbkit-test the values that fill a recovery's
  placeholders ([ADR-0053]).
- crumbkit-test: [`check_twins`] runs every twin of a tool and fails on a
  declared code that no twin refuses and `Twins::unprovoked` does not list
  ([ADR-0005]).
- crumbkit-test: [`check_recoveries`] fails on a failure with a declared
  code that offers no `run`, `retry` or `hint` step, unless an opt-out from
  `Rule::NextStep` names the failing command ([ADR-0053]).
- crumbkit-test: [`FailureReader`] and `Examples::failure_reader` let a
  tool that prints its own failures without a failure form give the twin,
  example and recovery checks its failures ([ADR-0045]).
- crumbkit-test: [`Allow::untwinned`] opts a declared code out of its twin
  with the `Failing` that provokes it, and [`check_twins`] runs that line
  ([ADR-0005], [ADR-0036]).
- crumbkit-test: [`Failing::env`] sets or removes environment variables
  for a failing line and its chain, and [`Twin::variable`] makes a twin
  that differs by one variable ([ADR-0005], [ADR-0053]).
- crumbkit: [`command_path`] returns the path of the command that a tool's
  words name, reading options as [`parse`] does, without building the
  tree ([ADR-0021]).
- crumbkit: `Exit::failures` lists the failure codes that end a run with
  the tool's exit code ([`help::Exit`], [ADR-0037]).
- crumbkit-test: [`check_exit_codes`] rejects a failure code that two
  exits list, and [`check_twins`] rejects a listed code that no `CODES`
  list holds ([ADR-0037]).
- crumbkit: `Guide::codes` takes [`help::Rename`] values, each of which
  prints the tool's own code in place of one of crumbkit's failure codes.
  `Guide::printed` and `Guide::failure_exit` return the printed code and
  the exit code of a failure ([ADR-0056]).
- crumbkit-test: `Rule::SharedExit` lets a tool declare the exit code 1 or
  2 with a stated reason, and [`check_exit_codes`] reports a crumbkit code
  that the guide renames twice and two renames to one code of the tool
  ([ADR-0037], [ADR-0056]).
- crumbkit-test: [`check_twin`] and [`check_recoveries`] fail a run that
  exits otherwise than `Guide::failure_exit` decides for its first failure,
  so an unlisted failure exits 1 and a usage refusal 2 by default
  ([ADR-0037]).
- crumbkit: `Leaf::passes_on_exit` declares that a command passes on a
  program's exit code. Help and `help exit-codes` say so, and the run
  checks then accept any code but 0 for an unlisted failure ([ADR-0042]).
- crumbkit-test: [`RuntimeFailing`] builds failing lines from a code,
  words, paths, input, values and variables that a test gets at run time,
  reuses its settings across lines with `line`, and gives a `Failing`, a
  `Twin` or an `Allow::untwinned` opt-out ([ADR-0002]).
- crumbkit: a failure field can hold null, a list or an object, built
  with `FieldValue::list`, `FieldValue::object` or an `Option`, and the
  failure document holds it as a JSON value ([ADR-0007]).
- crumbkit-core: [`Row`] gains `PROBLEM`, which names the field that marks
  a row as reporting a problem ([ADR-0050]).
- crumbkit: a leaf whose row type names a problem field prints a
  `Problems:` line in `-h` and `--help` ([ADR-0050]).
- crumbkit-test: [`check_guide`] fails when a row type's problem field is
  not one of its `COLUMNS`, or when its type has no value that means no
  problem ([ADR-0050]).
- crumbkit: [`help::ViewFlag`] names `--fields`, `--long` and `--full`,
  and `Leaf::offers` reports whether a leaf's document can use one
  ([ADR-0016]).
- crumbkit-test: [`check_guide`] fails on an example that types a view flag
  that its leaf does not offer ([ADR-0016]).
- crumbkit-test: [`check_guide`] fails on a leaf whose effects change
  something and that declares no document, unless it passes on a program's
  exit code, because crumbkit refuses an unusable view flag only after the
  change ([ADR-0016]).
- crumbkit: [`complete_with`] answers a shell's completion request under a
  variable that the tool names, and the registration script and the usage
  log use that name ([ADR-0048], [ADR-0056]).

### Changed

- crumbkit: [`finish`], batches and [`Streaming`] exit with the code that
  the guide declares for the run's first failure, and the usage log and
  the `crumbkit.finish` span record that number ([ADR-0037]).
- crumbkit-test: [`check_twin`] takes the tool's guide as its first
  argument and reads a refused run through the guide's failure form
  ([ADR-0045]).
- crumbkit-test: [`check_recoveries`] fills a placeholder from the `fields`
  of the failure that offers the step before it reads `Recoveries::values`
  ([ADR-0053]).
- crumbkit-test: [`check_recoveries`] fills each placeholder inside a word,
  such as `<plugin>.<capability>`, and keeps the text between them
  ([ADR-0053]).
- crumbkit: the help text of `--json` reads `Print the result as a JSON
  document` and no longer names a layout.
- crumbkit-test: when the guide sets `JsonLayout::Pretty`, [`check_examples`]
  requires a `--json` example of one result to print one indented document
  ([ADR-0052]).
- crumbkit-test: [`check_output_budget`], [`check_twin`],
  [`check_recoveries`] and [`check_examples`] read an indented JSON document
  as well as a document on one line.
- crumbkit: with the `usage-log` feature, a run whose `COMPLETE` variable
  asks for shell completion writes no line ([ADR-0048]).
- crumbkit: a build with debug assertions writes a `warning:` line on
  standard error for each recovery call of a failure that does not render,
  with the reason ([ADR-0001]).
- crumbkit: a guided list ends each description with the command's visible
  aliases, as clap's `Commands:` list does ([ADR-0021]).
- crumbkit-test: [`check_names`] takes the tool's guide and fails on a
  guide that names personal aliases unless an opt-out of
  `Rule::PersonalAliases` covers the root ([ADR-0027]).
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
- crumbkit: each row of `schemas list` has a `kind` field and column, which
  is `output`, `unwrapped` or `file` ([ADR-0035]).
- crumbkit-test: [`check_document_ids`] fails on a document without
  crumbkit's envelope unless an opt-out of `Rule::Envelope` names its
  command ([ADR-0036]).
- crumbkit-test: [`check_schema_goldens`] pins the schemas of file formats
  and fails on a file format with the id of another document ([ADR-0035]).
- crumbkit: `usage::findings` and `usage::figures` take the tool's
  `Guide`, which gives the meaning of each exit code ([ADR-0034]).
- crumbkit: [`parse`] returns a [`Parsed`] in place of the `Command`. It
  holds the command and the typed words, and `Output::set_command` takes
  either ([ADR-0002]).
- crumbkit: [`Sink`] has a third type parameter for standard input, and
  `Sink::with_input` builds one for tests.
- crumbkit-test: [`check_grammar`] no longer checks `--dry-run`, which
  [`check_effects`] checks.
- crumbkit: `Output::confirm` asks only when no coding agent is detected,
  and otherwise fails as it does without a terminal ([ADR-0043]).
- crumbkit: crumbkit enables clap's `error-context` feature, which adds
  about 33 KB to a tool's release binary ([ADR-0003]).
- cargo-crumbkit: the failures of building and running a tool's binary
  have `crumbkit::tool::*` codes, such as `crumbkit::tool::build_failed`,
  which `weigh` and `docs` share ([ADR-0033]).
- crumbkit-aliases: `aliases list` lists the entries with a problem first,
  so a page cut short by `--limit` still shows them ([crumbkit-aliases]).
- crumbkit: the confirmation codes are `crumbkit::confirmation::declined`
  and `crumbkit::confirmation::needs_yes`, whose area is a noun ([ADR-0001]).
- cargo-crumbkit: the failures of `weigh` have `crumbkit::weight::*` codes,
  such as `crumbkit::weight::help_failed` ([ADR-0033]).

- crumbkit-test: [`check_opt_outs`] takes the tool's guide and lists an
  output budget that the guide sets as `output-budget/run` ([ADR-0046]).
- crumbkit: the JSON Schema of each listing and list document holds the
  optional `cut` object, and the documents keep their `v1` ids
  ([ADR-0046]).
- crumbkit: in text, a listing's footer, empty note and note, the `Next:`
  line, the dry-run line, the cut line and a stream's notes print on
  standard error after standard output is flushed ([ADR-0050]).
- crumbkit-test: [`check_examples`] compares an example's standard error
  with its `.stderr` golden file, and fails on standard error only for a
  `warning: crumbkit ` line ([ADR-0005]).
- crumbkit-test: [`check_twin`], [`check_recoveries`] and [`check_examples`]
  read a failed run through `FailureForm::read`, and [`check_recoveries`]
  runs a guide with a failure form instead of failing before any run
  ([ADR-0045]).
- cargo-crumbkit: `weigh` adds the size of an example's `.stderr` golden
  file to the command's output weight ([ADR-0033]).
- crumbkit: every listing's JSON document holds `truncated`, which is true
  when the document leaves out rows because of `--limit` or the output
  budget, and false when it holds every row ([ADR-0007]).
- crumbkit-test: [`check_opt_outs`] gives the cost of a
  `documents/envelope` opt-out as a document without `truncated` in place
  of `total`.
- crumbkit: every listing's JSON Schema marks `total` as optional, and the
  documents keep their `v1` ids, because no schema has been released
  ([ADR-0007]).
- crumbkit-markdown: the reference refuses a `commands list` document that
  leaves out `total` or holds `"truncated": true`, because it would leave
  commands out.
- crumbkit: with the `miette` feature, a text failure prints one source
  line before and after each label's lines, where the source has them
  ([ADR-0016]).
- crumbkit: with the `miette` feature, a text failure prints every line of
  a span over several lines, with carets on its first and last line, and
  counts the lines left out of a span over more than 7 lines ([ADR-0016]).
- crumbkit: with the `miette` feature, labels on the same or nearby lines
  print their `at:` lines together and share their source lines, and each
  underline ends with its label's text ([ADR-0016]).
- crumbkit: with the `miette` feature, the carets under a label measure
  terminal columns with unicode-width, so they line up after wide
  characters such as CJK text ([ADR-0016]).
- crumbkit-test: [`check_recoveries`] fills a lowercase placeholder such
  as `<path>` from the value of the same name ([ADR-0053]).
- crumbkit-test: [`check_recoveries`] fills placeholders and reports
  `recoveries/field-value` from the fields of the failure that a form or a
  reader returns ([ADR-0053]).
- crumbkit-test: [`check_recoveries`] follows every failure that a run
  prints, such as each entry of a batch, and a start passes when any of
  its failures carries its code ([ADR-0053]).
- crumbkit-test: [`check_recoveries`] compares a step's code with the code
  of every earlier failure of its chain, so a chain that returns to an
  earlier code through another code fails the check ([ADR-0053]).
- crumbkit-test: [`check_recoveries`] reads a failed run's last `code:`
  line when the run prints no failure document ([ADR-0045]).
- crumbkit-test: [`check_twin`], [`check_examples`] and
  [`check_recoveries`] fail when the guide sets a failure form and
  `Examples` sets a failure reader ([ADR-0045]).
- crumbkit-test: [`check_twins`] adds `--json` to both lines of each twin
  when neither line holds it, as [`check_recoveries`] does, so a tool's
  reader sees the failure an agent reads ([ADR-0005]).
- crumbkit-test: [`check_twin`] refuses a twin whose admitted line adds a
  help or version word, or runs another command than the refused line
  ([ADR-0005]).
- crumbkit-test: a check's run whose tool exits while a process it started
  keeps standard output or standard error open ends with a problem at the
  run's timeout, and the check no longer waits for that process ([ADR-0005]).
- crumbkit-test: [`check_twin`] and [`check_twins`] take the tool's command
  tree first and read each line's command from it, so a global option
  before the path and an alias name the same command ([ADR-0005]).
- crumbkit-test: [`check_twin`] refuses a twin in which neither line runs a
  command, so a twin of the misspelt `books lst` admits `books list` and
  not `books` ([ADR-0005]).
- crumbkit: an `Exit` may list one of crumbkit's failure codes or the
  tool's rename of it, so a refused command line can exit with the tool's
  code ([ADR-0056]).
- crumbkit: `help exit-codes` and the `commands list` note leave out
  crumbkit's 2 when a refusal exits with another code ([ADR-0037]).
- crumbkit: `self usage` and `self figures list` count a run as refused
  when it exited with the code that the guide gives a refused command line
  ([ADR-0034]).
- crumbkit-test: [`check_exit_codes`] accepts an exit that lists one of
  crumbkit's codes, and refuses the exit code 1 or 2 without an opt-out of
  `Rule::SharedExit`. [`check_twin`], [`check_recoveries`] and
  [`check_output_budget`] expect the codes and exits that the guide
  declares ([ADR-0056]).
- crumbkit: [`finish`] and [`stream`] exit with 1 when a row reports a
  problem and the run has no failure, and the text ends with a note that
  counts those rows ([ADR-0050]).
- crumbkit: `help exit-codes` says that 1 also means a row of the result
  reported a problem ([ADR-0050]).
- crumbkit-aliases: `aliases list` exits with 1 while an entry of the
  aliases file has a problem ([ADR-0027]).
- crumbkit: a leaf's help leaves out each view flag that its document
  cannot use, and [`parse`] refuses such a flag as a refused command line
  before the handler runs ([ADR-0016]).
- crumbkit: [`finish`] and [`stream`] refuse `--long` and `--full` for
  prose, `--long` for rows without long columns, and `--full` for a
  document without the envelope ([ADR-0016]).
- crumbkit: `complete_to` takes the name of the completion variable, which
  the registration script sets ([ADR-0048]).

### Removed

- crumbkit: `help::split` is gone. [`check_recoveries`] reads a recovery's
  words with its own private reading, which also reads double-quoted words
  and refuses a `$` or a backtick between double quotes ([ADR-0053]).
- crumbkit-test: `Examples::failure_form` is gone, because [`check_twin`]
  reads the failure form from the tool's guide ([ADR-0045]).
- crumbkit-core: `Document::over_budget` and `Document::over_budget_code`
  are gone, because a rename of `crumbkit::output::over_budget` in
  `Guide::codes` sets the code for every document ([ADR-0046]).
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
- crumbkit-test: [`check_examples`], [`check_twin`] and [`check_recoveries`]
  report each run that their timeout stopped as stopped, where a stop up to
  1 ms early showed as a failed run that ended with SIGKILL ([ADR-0005]).
- crumbkit-test: [`check_output_budget`], [`check_command_reference`] and
  the help checks that run the binary say when their timeout stopped a run,
  and a run without input reads an empty, closed standard input.
- crumbkit-test: the checks that run the binary write standard input on
  their own thread, so a run that never reads an
  input larger than a pipe holds is stopped at its timeout instead of
  blocking the check ([ADR-0005]).
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
- crumbkit: a tool that starts the usage log with `Signals::Tool` keeps
  handling Ctrl-C itself after [`child::hand_over`] has run a child
  ([ADR-0042]).
- crumbkit: on Unix, a Ctrl-C sent while [`child::hand_over`] runs a child
  no longer ends a tool with more than one thread, such as one with the
  usage log, when the signal reaches the tool after the child has exited
  ([ADR-0042]).
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
- crumbkit-test: [`check_recoveries`] adds `--json` before a `--` word, so
  the tool reads it as a flag and not as a positional value ([ADR-0053]).
- crumbkit-test: a check kills a run whose exit it cannot learn before it
  reports the error, so the run does not outlive the check ([ADR-0005]).

<!-- Each label below points at the source file that holds the item's
rustdoc, or at its decision record. -->

[`Row`]: crates/crumbkit-core/src/row.rs
[`Document`]: crates/crumbkit-core/src/document.rs
[`Output`]: crates/crumbkit/src/output.rs
[`finish`]: crates/crumbkit/src/output.rs
[`stream`]: crates/crumbkit/src/output/stream.rs
[`Stream`]: crates/crumbkit-core/src/stream.rs
[`Streaming`]: crates/crumbkit/src/output/stream.rs
[`Page`]: crates/crumbkit/src/page.rs
[`Listing::uncounted`]: crates/crumbkit-core/src/listing.rs
[`Page::uncounted`]: crates/crumbkit/src/page.rs
[`Listing`]: crates/crumbkit-core/src/listing.rs
[`Call`]: crates/crumbkit-core/src/call.rs
[`Failure`]: crates/crumbkit-core/src/failure.rs
[`Recovery`]: crates/crumbkit-core/src/failure.rs
[`Code`]: crates/crumbkit-core/src/code.rs
[`Spelling`]: crates/crumbkit-core/src/code.rs
[`Code::runtime`]: crates/crumbkit-core/src/code/runtime.rs
[`Error`]: crates/crumbkit-core/src/failure.rs
[`Batch`]: crates/crumbkit-core/src/batch.rs
[`Prose`]: crates/crumbkit-core/src/prose.rs
[`Bytes`]: crates/crumbkit-core/src/bytes.rs
[`fetch_each`]: crates/crumbkit/src/fetch.rs
[`report`]: crates/crumbkit-core/src/report.rs
[`parse`]: crates/crumbkit/src/parse.rs
[`command_path`]: crates/crumbkit/src/parse/walk.rs
[`RuntimeFailing`]: crates/crumbkit-test/src/recoveries/runtime.rs
[`Stop`]: crates/crumbkit/src/parse/stop.rs
[`usage::ended`]: crates/crumbkit/src/usage.rs
[`usage::tolerated`]: crates/crumbkit/src/usage.rs
[`help`]: crates/crumbkit/src/help.rs
[`help::quote`]: crates/crumbkit/src/help.rs
[`help::ViewFlag`]: crates/crumbkit/src/help/view.rs
[`help::RuntimeLeaf`]: crates/crumbkit/src/help/runtime.rs
[`help::FileFormat`]: crates/crumbkit/src/help/format.rs
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
[`check_exit_codes`]: crates/crumbkit-test/src/exits.rs
[`help::Exit`]: crates/crumbkit/src/help/exit.rs
[`help::Rename`]: crates/crumbkit/src/help/rename.rs
[`check_help_sizes`]: crates/crumbkit-test/src/sizes.rs
[`check_help_goldens`]: crates/crumbkit-test/src/goldens.rs
[`check_examples`]: crates/crumbkit-test/src/examples.rs
[`Examples`]: crates/crumbkit-test/src/examples/settings.rs
[`FailureReader`]: crates/crumbkit-test/src/failures.rs
[crumbkit-aliases]: crates/crumbkit-aliases/README.md
[crumbkit-core]: crates/crumbkit-core/README.md
[`RenderCall`]: crates/crumbkit/src/call.rs
[ADR-0001]: docs/decisions/api/failures/0001-error-model.md
[ADR-0002]: docs/decisions/api/0002-public-api-and-semver.md
[ADR-0003]: docs/decisions/project/0003-dependency-policy.md
[ADR-0004]: docs/decisions/testing/0004-golden-and-snapshot-tests.md
[ADR-0005]: docs/decisions/testing/0005-cli-end-to-end-tests.md
[ADR-0007]: docs/decisions/api/schemas/0007-json-schemas.md
[ADR-0015]: docs/decisions/api/adoption/0015-lightweight-adoption.md
[ADR-0016]: docs/decisions/api/output/results/0016-result-formats.md
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
[ADR-0035]: docs/decisions/api/schemas/0035-schema-commands.md
[`check_schema_goldens`]: crates/crumbkit-test/src/schemas.rs
[`check_document_ids`]: crates/crumbkit-test/src/documents.rs
[`check_mounts`]: crates/crumbkit-test/src/documents.rs
[`check_path_weights`]: crates/crumbkit-test/src/weights.rs
[`path_totals`]: crates/crumbkit-test/src/weights.rs
[`output_weight`]: crates/crumbkit-test/src/weights.rs
[`Allow`]: crates/crumbkit-test/src/allow.rs
[`Allow::untwinned`]: crates/crumbkit-test/src/allow.rs
[`Failing::env`]: crates/crumbkit-test/src/recoveries.rs
[`Twin::variable`]: crates/crumbkit-test/src/twins/coverage.rs
[`check_opt_outs`]: crates/crumbkit-test/src/opt_outs.rs
[`check_output_budget`]: crates/crumbkit-test/src/output_budget.rs
[`Budget`]: crates/crumbkit-test/src/budget.rs
[`Budgets`]: crates/crumbkit-test/src/sizes.rs
[`PathBudgets`]: crates/crumbkit-test/src/weights.rs
[ADR-0036]: docs/decisions/api/adoption/0036-opt-outs.md
[ADR-0037]: docs/decisions/api/help/0037-exit-codes.md
[`Effects`]: crates/crumbkit/src/help/effects.rs
[`DryRun`]: crates/crumbkit/src/output/dry_run.rs
[`Yes`]: crates/crumbkit/src/output/confirm.rs
[`Parsed`]: crates/crumbkit/src/parse/parsed.rs
[`Sink`]: crates/crumbkit/src/sink.rs
[`check_effects`]: crates/crumbkit-test/src/effects.rs
[ADR-0039]: docs/decisions/api/effects/0039-effects-dry-runs-and-confirmation.md
[ADR-0041]: docs/decisions/api/output/0041-streams.md
[`child::hand_over`]: crates/crumbkit/src/child.rs
[`child::hand_over_json`]: crates/crumbkit/src/child.rs
[ADR-0042]: docs/decisions/api/output/0042-child-processes.md
[ADR-0043]: docs/decisions/api/effects/0043-pickers-and-interactive-input.md
[`Picker`]: crates/crumbkit/src/pick.rs
[`NoInput`]: crates/crumbkit/src/pick/no_input.rs
[crumbkit-picker]: crates/crumbkit-picker/README.md
[crumbkit-markdown]: crates/crumbkit-markdown/README.md
[ADR-0044]: docs/decisions/api/help/0044-command-reference.md
[`check_command_reference`]: crates/crumbkit-test/src/reference.rs
[`FailureForm`]: crates/crumbkit/src/form.rs
[`ReadFailure`]: crates/crumbkit/src/form.rs
[`PreparedFailure`]: crates/crumbkit/src/output/failure/prepared.rs
[ADR-0045]: docs/decisions/api/failures/0045-failure-forms.md
[ADR-0046]: docs/decisions/api/output/0046-output-budget.md
[ADR-0049]: docs/decisions/api/output/0049-terminal-detection.md
[ADR-0052]: docs/decisions/api/output/0052-json-layout.md
[ADR-0053]: docs/decisions/testing/0053-recovery-chains.md
[ADR-0056]: docs/decisions/api/adoption/0056-declarations.md
[ADR-0050]: docs/decisions/api/output/listings/README.md
[`codes`]: crates/crumbkit/src/codes.rs
[`check_recoveries`]: crates/crumbkit-test/src/recoveries.rs
[`check_twin`]: crates/crumbkit-test/src/twins.rs
[`check_twins`]: crates/crumbkit-test/src/twins/coverage.rs
[`complete`]: crates/crumbkit/src/complete.rs
[`complete_with`]: crates/crumbkit/src/complete.rs
[ADR-0048]: docs/decisions/api/0048-completion.md
[`Expect`]: crates/crumbkit/src/expect.rs
[ADR-0047]: docs/decisions/api/effects/0047-write-guards.md
