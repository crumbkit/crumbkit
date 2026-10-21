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
  form the failure model ([behaviour/failures]).
- crumbkit: [`Batch`] holds the rows and the failures of a per-id command,
  and [`fetch_each`] runs the per-id lookups on scoped threads.
- crumbkit: the `miette` feature adds `Failure::diagnostic` and converts a
  `miette::Report` into [`Error`] ([`report`]).
- crumbkit: with the `miette` feature, the JSON failure document holds the
  diagnostic's related diagnostics and a source excerpt for each label.
- crumbkit: [`parse`] parses the command line, prints help and refusals,
  and never exits the process.
- crumbkit: [`help`] adds goal headings, help topics, and leaf examples and
  columns to help pages ([behaviour/help], [behaviour/help/examples]).
- crumbkit: [`commands`] gives a tool the `commands list` command
  ([behaviour/help/commands-list]).
- crumbkit: [`help::quote`] quotes one word for a POSIX shell.
- crumbkit: the [schemas] directory holds the JSON Schemas of the
  `<tool>/failure/v1` and `<tool>/commands-list/v1` documents
  ([behaviour/schemas]).
- crumbkit-usage: the new crate appends one line per run to a local usage
  log, which a tool starts with `crumbkit_usage::start()`, and holds the
  JSON Schema of the `crumbkit/usage/v1` document ([crumbkit-usage],
  [`usage`], [behaviour/usage-log]).
- crumbkit-usage: a run joins its caller's W3C trace
  ([behaviour/usage-log/trace-context]).
- crumbkit: [`trace::child_env`] passes a run's W3C trace to a child
  process ([behaviour/usage-log/trace-context]).
- crumbkit: the `tracing` feature runs `parse` and `finish` inside spans
  ([`trace`]).
- crumbkit-usage: `self usage` reports where callers waste effort from the
  usage log ([behaviour/usage-log/report]).
- crumbkit: a [`Listing`] can carry a summary that prints above its table
  ([behaviour/output]).
- crumbkit-test: the checks [`check_codes`], [`check_names`],
  [`check_grammar`], [`check_listings`], [`check_filters`],
  [`check_guide`] and [`check_help_sizes`] fail a test on a tool that
  breaks crumbkit's conventions ([behaviour/help/checks]).
- crumbkit-test: [`check_help_goldens`] and [`check_examples`] compare help
  pages and example output with golden files ([project/testing]).
- crumbkit-test: [`check_guide`] fails on two groups with one path.
- crumbkit-aliases: the new crate expands a person's command aliases and
  lists them ([crumbkit-aliases], [behaviour/parsing/aliases]).
- crumbkit: `Call::render` comes from the [`RenderCall`] trait, and
  `Call` holds its data without reading clap ([behaviour/adoption]).
- cargo-crumbkit: the new crate's `cargo crumbkit weigh --bin <NAME>`
  prints the bytes of each command's help pages and example output, and
  each path's total ([cargo-crumbkit], [behaviour/help/weight]). crumbkit-test's
  [`example_golden_name`] names each example's golden file.
- crumbkit: [`schemas`] gives a tool `schemas list`, which lists the JSON
  documents it prints, and `schemas show <ID>...`, which prints the JSON
  Schema of each ([behaviour/schemas/commands]).
- crumbkit: [`Document`] takes a one-line description, and each document's
  name is the path of the command that prints it, such as
  `crumbkit/commands-list/v1` ([behaviour/schemas]).
- crumbkit-test: [`check_schema_goldens`] pins the JSON Schema of each
  document a tool prints, [`check_document_ids`] checks each document's
  name, and [`check_mounts`] checks the paths of crumbkit's commands
  ([behaviour/schemas/commands]).
- crumbkit-usage: `self figures list` prints the figures of every command
  in the usage log, and `self usage --full` adds `help_before` to each
  command's figures ([`usage`], [behaviour/usage-log/report]).
- crumbkit-usage: `self usage` and `self figures list` take `--log <FILE>` to
  read a copy of a usage log ([behaviour/usage-log/report]).
- crumbkit-test: [`check_path_weights`] budgets the bytes of each path
  through a tool's help, 8,000 by default, and [`path_totals`] and
  [`output_weight`] compute them ([behaviour/help/checks]).
- cargo-crumbkit: `weigh --usage`, `--log` and `--since` add each
  command's runs, help pages read before it and 95th percentile output
  from a usage log ([cargo-crumbkit], [behaviour/help/weight]).
- crumbkit: `Output::is_json` says whether the caller asked for JSON, and
  `Output::progress_allowed` says whether a tool may draw progress
  ([`Output`], [behaviour/output]).
- crumbkit: [`events::ended`] records a tool's own exit number and output
  bytes for a run that never reaches `finish` ([behaviour/usage-log]).
- crumbkit-usage: `crumbkit_usage::start_with(Signals::Tool)` lets a tool
  handle SIGINT and SIGTERM itself while the usage log records the signal
  ([`usage`], [behaviour/usage-log]).
- crumbkit: [`Failure`] has `alternatives`, other repairs that print as
  extra `help:` lines and under `alternatives` in the failure document
  ([behaviour/failures]).
- crumbkit: `Call::value` adds an argument with a concrete value, such as
  `--author 'Le Guin'`, and rendering checks it against the tool's command
  tree ([`Call`], [behaviour/failures]).
- crumbkit: [`Prose`] holds a command's answer as text with a typed value
  and optional failures. Text output prints the text, and `--json` prints
  the value under `data` and the failures under `failures` ([behaviour/output]).
- crumbkit: [`help::RuntimeLeaf`] and `Guide::with_runtime_leaves` give
  examples to commands that a tool adds at run time ([behaviour/help]).
- crumbkit: `Guide::documents` declares [`help::FileFormat`]s, the formats
  of files that users write for a tool, and `schemas list` and
  `schemas show` serve their schemas ([behaviour/schemas/commands]).
- crumbkit: `Document::envelope` chooses a document's envelope: crumbkit's,
  or none, which prints the result alone ([behaviour/output]).

- crumbkit-test: [`check_opt_outs`] pins a tool's opt-outs and budgets,
  with each reason and what it costs an agent, in a text and a JSON golden
  file ([behaviour/adoption/opt-outs]).
- crumbkit: `Guide::exits` declares a tool's own exit codes, each an
  [`help::Exit`] with a meaning and optional commands, and the built-in
  `help exit-codes` topic lists crumbkit's codes and the tool's
  ([behaviour/failures/exit-codes]).
- crumbkit: each `commands list` row has `exits`, the declared codes that
  the command can return, and the text output ends with a note that names
  every code ([behaviour/help/commands-list]).
- crumbkit-test: [`check_exit_codes`] rejects a declared exit code that
  crumbkit owns or that a shell reports, a repeated code, a meaning that is
  not one line, and a command path that names no command ([behaviour/failures/exit-codes]).
- crumbkit-test: [`check_guide`] rejects a topic named `exit-codes`, which
  crumbkit's own topic replaces ([behaviour/help]).
- crumbkit: `Guide::root` names the command that callers run first, such as
  `crumbkit` under `cargo`, which answers `help` with the topics; cargo
  crumbkit sets it, and [`check_guide`] rejects a root that names no
  command ([behaviour/failures/exit-codes]).
- crumbkit: [`events::tolerated`] records the code of a failure that a run
  carried on past ([behaviour/usage-log]).
- crumbkit-usage: the usage record lists up to 8 codes that the run
  tolerated in `tolerated` ([behaviour/usage-log]).
- crumbkit-usage: the `self usage` summary counts the runs that tolerated each
  code, and each command's figures count the runs of each exit code from 3
  with its declared meaning ([behaviour/usage-log/report]).
- crumbkit: [`Effects`] declares a leaf's effects in the terms of MCP's
  tool annotations with `Leaf::effects`, and help and `commands list` show
  them ([behaviour/effects]).
- crumbkit: [`DryRun`] adds `--dry-run`. `parse` records the flag in
  `Output`, `Output::is_dry_run` reports it, the JSON document holds
  `dry_run: true` and the text ends with a dry-run line
  ([behaviour/effects]).
- crumbkit: [`Yes`] adds `--yes`. `Output::confirm` asks at a terminal,
  proceeds with `--yes`, and otherwise fails with the typed command plus
  `--yes` as its recovery ([behaviour/effects]).
- crumbkit: [`Call`] gains `Call::typed`, which repeats the words that the
  caller typed without a lookup in the command tree.
- crumbkit-test: [`check_effects`] requires effects on every visible leaf,
  `--yes` on destructive leaves, and crumbkit's `--dry-run` in place of a
  tool's own ([behaviour/effects]).
- crumbkit: [`Stream`] is the result type of a document whose command
  prints rows as they arrive ([behaviour/output/streams]).
- crumbkit: `stream` opens a [`Streaming`], which prints each row as it
  arrives as a table line or a JSON Lines document, and `Leaf::stream`
  declares it in help ([behaviour/output/streams]).
- crumbkit: `commands list` rows and `schemas list` rows hold `form`,
  which is `document` or `stream` ([behaviour/output/streams]).
- crumbkit-test: [`check_examples`] stops an example after 10 seconds and
  checks that each line of a `--json` example is a document with the
  leaf's id ([behaviour/output/streams]).
- crumbkit-child: the new crate's [`child::hand_over`] hands the terminal
  to a child process and returns its exit code. On Unix the tool ignores
  SIGINT and SIGQUIT from the moment the child starts until the tool ends
  ([crumbkit-child], [behaviour/output/child-processes]).
- crumbkit-child: the crate's codes are spelled `crumbkit::child::*`, and
  its own `codes::ALL` lists them ([crumbkit-child]).
- crumbkit-child: [`child::hand_over_json`] runs a child that prints JSON under
  `--json` and prints its output as the declared document, so `--fields`,
  `--long` and `--full` apply ([behaviour/output/child-processes]).
- crumbkit-child: Ctrl-C while a child holds the terminal ends only the
  child, and crumbkit-usage records the child's exit code in the run's line
  ([behaviour/output/child-processes]).
- crumbkit: `Guide::personal_aliases` makes the root's help name each
  personal alias that expands, and only a run that prints that help reads
  the aliases file ([behaviour/parsing/aliases]).
- crumbkit-aliases: `names` returns the names of the aliases file's entries
  that expand, for `Guide::personal_aliases` ([crumbkit-aliases]).
- crumbkit: `parse` asks the guide's [`Picker`] for a missing required
  argument's values when a person can answer, adds them and prints the run's
  command line on standard error ([behaviour/effects/pickers]).
- crumbkit: `Leaf::choices` names a source of choices for an argument, and
  `pick::from_rows` turns a command's rows into choices ([behaviour/effects/pickers]).
- crumbkit: [`NoInput`] adds the global `--no-input`, which stops pickers
  and makes `Output::confirm` fail without asking ([behaviour/effects/pickers]).
- crumbkit-usage: a run's line records `picked`, the number of arguments
  that a picker filled ([behaviour/usage-log]).
- crumbkit-test: [`check_effects`] requires `--no-input` from `NoInput` on a
  tool that can ask a question, unless the tool opts out of
  `Rule::NoInput` ([behaviour/effects/pickers]).
- crumbkit-test: [`check_guide`] fails on leaf choices that name no
  argument of the leaf.
- crumbkit: [`pick::Menu`] picks values from a numbered list that text
  narrows, using only the standard library ([behaviour/effects/pickers]).
- crumbkit: Windows is a tier-2 platform, which the scheduled lane builds
  and tests ([project/dependencies]).
- crumbkit-usage: the crate builds on Windows, where the log lives
  in `%LOCALAPPDATA%\<tool>`, or under `XDG_STATE_HOME` when it is an
  absolute path ([behaviour/usage-log]).
- crumbkit-child: on Windows, [`child::hand_over`] keeps the tool running on
  Ctrl-C or Ctrl-Break, and a child that Ctrl-C ended returns 130 and marks
  the line interrupted ([behaviour/output/child-processes]).
- crumbkit: help on Windows names the tool without `.exe`.
- cargo-crumbkit: `weigh --usage` finds the usage log on Windows
  ([behaviour/help/weight]).
- crumbkit: [`Failure`] has a `fields` method, and the JSON failure
  document holds the named values under `fields` ([behaviour/schemas]).
- crumbkit: a refused command line or `--fields` name holds the refused
  argument, value or subcommand under `fields` ([behaviour/failures]).
- crumbkit: [`Code::runtime`] builds a code known only at run time, and
  `Spelling::split` reads a code in `Spelling::COLONS` back into its
  segments ([behaviour/failures]).
- crumbkit: `Stop::number` returns a stopped run's exit code as a number
  ([behaviour/parsing]).
- crumbkit: [`try_parse`], `try_parse_from` and `try_parse_to` return a
  refused command line unprinted as a [`Refused`], which holds the typed
  words, the parsed words and the refused word, and prints once
  ([behaviour/parsing]).
- crumbkit-test: `Recoveries::from_codes`, `Twins::from_codes` and
  `Unprovoked::from_code` take codes held by value, such as run-time codes
  ([behaviour/failures]).
- crumbkit-markdown: the new crate's `render` writes a Markdown reference
  of every command of a tool from its `commands list --all --json --full`
  document ([crumbkit-markdown], [behaviour/help/reference]).
- crumbkit: `Effects::line` and `help::STREAM_LINE` are public, so another
  rendering of help prints the effects and stream lines in help's wording
  ([`Effects`], [behaviour/help/reference]).
- crumbkit-test: [`check_command_reference`] fails a test when a tool's
  checked-in Markdown reference differs from the one crumbkit-markdown
  writes from its help, and `SNAPSHOTS=overwrite` rewrites it
  ([behaviour/help/reference]).
- cargo-crumbkit: `cargo crumbkit docs --bin <NAME>` writes the binary's
  Markdown command reference to `docs/commands.md` in its package, or to
  `--out <FILE>` ([cargo-crumbkit], [behaviour/help/reference]).
- crumbkit: [`codes`] makes crumbkit's own failure codes public constants
  and lists them in `codes::ALL`, so a tool's test can name them
  ([behaviour/failures]).
- crumbkit-test: [`check_recoveries`] runs failing command lines, follows
  each recovery they print and pins the chains in a golden report, and
  [`check_twin`] runs a refused command line beside one that succeeds
  ([behaviour/checks/end-to-end], [behaviour/checks/recovery-chains]).
- crumbkit-test: `Rule::Recoveries` and `Rule::SameCode` let a tool opt out
  of the recovery check ([behaviour/adoption/opt-outs]).
- crumbkit-test: [`check_recoveries`] fails on a placeholder that a field
  of the failure fills, and shows the fill in the report, unless an opt-out
  from `Rule::FieldValue` names the failing command ([behaviour/checks/recovery-chains]).
- crumbkit: `Guide::without_terminal_detection(reason)` turns terminal
  detection off, so help has no colour, `Output::progress_allowed` is false,
  and pickers and `Output::confirm` never ask. `get_without_terminal_detection`
  returns the reason ([behaviour/output/terminal]).
- crumbkit-test: [`check_opt_outs`] lists a guide without terminal detection
  as `terminal-detection/off`, with the tool's reason and its costs
  ([behaviour/output/terminal]).
- crumbkit-test: [`check_recoveries`] takes the tool's guide
  ([behaviour/checks/recovery-chains]).
- crumbkit: the `complete` feature adds [`complete`], which answers a
  shell's completion request when `COMPLETE` names the shell, and
  `complete_to`. It follows clap_complete's unstable API, so a minor release
  may change it ([behaviour/parsing/completion]).
- crumbkit: `Effects::replacing`, `Effects::all_or_none` and
  `Effects::per_item` declare a command's write guards, which help prints
  on a `Writes:` line and `commands list` writes under `writes`; a `const`
  that declares a guard on `Effects::read_only` fails to compile
  ([behaviour/effects/write-guards]).
- crumbkit: [`Expect`] adds `--expect <VERSION>`, and a target at another
  version fails with `crumbkit::version::conflict`, whose recovery runs the
  command that reads the target ([behaviour/effects/write-guards]).
- crumbkit-test: [`check_effects`] requires crumbkit's `--expect` on a
  replacing command and `all_or_none` or `per_item` on a write of many
  items, with the opt-outs `Rule::Expect`, `Rule::ExpectFlag` and
  `Rule::Items` ([behaviour/effects/write-guards]).
- crumbkit-markdown: the reference prints a command's `Writes:` line after
  its effects line ([behaviour/effects/write-guards]).

- crumbkit: [`finish`] and [`stream`] print at most an output budget of
  1 MiB on standard output in one run. `Guide::output_budget` sets another
  budget of any size, and `CRUMBKIT_OUTPUT_BUDGET` only lowers it ([behaviour/output/budget]).
- crumbkit: a listing or list past the output budget prints the rows that
  fit and is marked with `cut`. Other results past it fail with
  `crumbkit::output::over_budget` ([behaviour/output/budget]).
- crumbkit: [`Document`] gains `always_print_first`, which prints the
  first answer even past the output budget ([behaviour/output/budget]).
- crumbkit-test: [`check_output_budget`] runs each `--json` example with
  half the bytes it prints and checks that the tool cuts or refuses the
  result ([behaviour/output/budget]).
- crumbkit: a listing with no rows, and a stream that ends with none,
  print a note such as `note: no books match`.
  `Document::when_empty` changes the wording ([behaviour/output/listings]).
- crumbkit: `Page<N, MOST>` refuses a `--limit` above `MOST` with a
  recovery that runs `--all`, and the help and the footer name the bound
  ([behaviour/output/listings]).
- crumbkit-usage: the line of a run whose listing or stream had no rows
  holds `"empty": true` ([behaviour/usage-log]).
- crumbkit: [`Bytes`] is a result that `finish` prints unchanged as
  text, and with `--json` as `text` when it is UTF-8 or `base64` otherwise.
  A `Bytes` row field serializes the same way. `Shape::Bytes` and
  `Rows::bytes` describe it ([behaviour/output], [behaviour/schemas]).
- crumbkit: `--fields`, `--long` and `--full` are refused with exit code 2
  for a [`Bytes`] result, which has no columns ([behaviour/output]).
- crumbkit: a [`Bytes`] result past the output budget is refused and never
  cut, and `--json` measures it with its base64 text ([behaviour/output/budget]).
- crumbkit: the help of a leaf that prints [`Bytes`] says how its output
  prints, and `help::BYTES_LINE` holds that line ([behaviour/output]).
- crumbkit: `Guide::json_layout(JsonLayout::Pretty { reason })` prints every
  whole JSON document indented by two spaces, and a stream keeps one
  document per line ([behaviour/output/json-layout]).
- crumbkit-test: [`check_opt_outs`] lists a guide's JSON layout as
  `json-layout/pretty`, with its reason and cost ([behaviour/output/json-layout]).
- crumbkit: [`Listing::uncounted`] and [`Page::uncounted`] build a listing
  from a source that the tool does not count. While more rows exist, its
  JSON document leaves out `total` and its footer reads `Showing 4 books,
  and more match.` ([behaviour/output/listings]).
- crumbkit: `Listing::singular` names one row, so an uncounted listing
  that shows one row reads `Showing 1 book, and more match.` Without a
  description, it reads `Showing 1 row` ([behaviour/output/listings]).
- crumbkit-test: `Examples::current_dir` sets the directory that each
  example run starts in, for [`check_examples`], [`check_output_budget`],
  [`check_twin`] and [`check_recoveries`] ([behaviour/checks/end-to-end]).
- crumbkit-test: [`check_examples`] gives an example the bytes of its
  `.stdin` file, such as `shelf-books-import.stdin`, on standard input, and
  fails on a `.stdin` file that names no example ([behaviour/checks/end-to-end]).
- crumbkit-test: [`check_examples`] replaces its own XDG paths with
  `[XDG_CONFIG_HOME]` and the like before it compares an output
  ([behaviour/checks/end-to-end]).
- crumbkit-test: `Allow::example` opts one example out of
  [`check_examples`] and [`check_output_budget`] by its words ([behaviour/adoption/opt-outs]).
- crumbkit-test: `Allow::option` opts one option of a command out of a
  filter rule of [`check_filters`], so each option keeps its own reason
  ([behaviour/adoption/opt-outs]).
- crumbkit: re-exports schemars as `crumbkit::schemars`, the version whose
  `Schema` crumbkit's API takes ([behaviour/schemas]).
- crumbkit-test: `Examples::xdg` and `Failing::xdg` name a fixture that
  the checks copy into each run's fresh XDG directories, so a failing
  command can find a plugin or cache there ([behaviour/checks/end-to-end]).
- crumbkit-test: `Examples::fresh_dir` names a fixture that the checks copy
  into a new working directory for each run, or once for each recovery
  chain, and example outputs show the copy's path as `[ROOT]` ([behaviour/checks/end-to-end]).
- crumbkit-test: `Failing::values` gives one recovery chain its own
  placeholder values, which win over every other source ([behaviour/checks/recovery-chains]).
- crumbkit-test: [`check_help_sizes`], [`check_help_goldens`] and
  [`check_path_weights`] take `Pages`, whose `Pages::binary` reads each
  page from the tool's binary, for a tool that builds its help at run
  time. A tool passes the same `Pages` to all three ([behaviour/help/checks]).
- crumbkit: [`ReadFailure`] carries the failure's fields, so a
  [`FailureReader`] gives crumbkit-test the values that fill a recovery's
  placeholders ([behaviour/checks/recovery-chains]).
- crumbkit-test: [`check_twins`] runs every twin of a tool and fails on a
  declared code that no twin refuses and `Twins::unprovoked` does not list
  ([behaviour/checks/end-to-end]).
- crumbkit-test: [`check_recoveries`] fails on a failure with a declared
  code that offers no `run`, `retry` or `hint` step, unless an opt-out from
  `Rule::NextStep` names the failing command ([behaviour/checks/recovery-chains]).
- crumbkit-test: [`FailureReader`] and `Examples::failure_reader` let a
  tool that prints its own failures give the twin, example and recovery
  checks its failures ([behaviour/failures/excerpts-and-readers]).
- crumbkit-test: [`Allow::untwinned`] opts a declared code out of its twin
  with the `Failing` that provokes it, and [`check_twins`] runs that line
  ([behaviour/checks/end-to-end], [behaviour/adoption/opt-outs]).
- crumbkit-test: [`Failing::env`] sets or removes environment variables
  for a failing line and its chain, and [`Twin::variable`] makes a twin
  that differs by one variable ([behaviour/checks/end-to-end], [behaviour/checks/recovery-chains]).
- crumbkit: [`command_path`] returns the path of the command that a tool's
  words name, reading options as [`parse`] does, without building the
  tree ([behaviour/help]).
- crumbkit: `Exit::failures` lists the failure codes that end a run with
  the tool's exit code ([`help::Exit`], [behaviour/failures/exit-codes]).
- crumbkit-test: [`check_exit_codes`] rejects a failure code that two
  exits list, and [`check_twins`] rejects a listed code that no `CODES`
  list holds ([behaviour/failures/exit-codes]).
- crumbkit: `Guide::codes` takes [`help::Rename`] values, each of which
  prints the tool's own code in place of one of crumbkit's failure codes.
  `Guide::printed` and `Guide::failure_exit` return the printed code and
  the exit code of a failure ([behaviour/adoption/declarations]).
- crumbkit-test: `Rule::SharedExit` lets a tool declare the exit code 1 or
  2 with a stated reason, and [`check_exit_codes`] reports a crumbkit code
  that the guide renames twice and two renames to one code of the tool
  ([behaviour/failures/exit-codes], [behaviour/adoption/declarations]).
- crumbkit-test: [`check_twin`] and [`check_recoveries`] fail a run that
  exits otherwise than `Guide::failure_exit` decides for its first failure,
  so an unlisted failure exits 1 and a usage refusal 2 by default
  ([behaviour/failures/exit-codes]).
- crumbkit: `Leaf::passes_on_exit` declares that a command passes on a
  program's exit code. Help and `help exit-codes` say so, and the run
  checks then accept any code but 0 for an unlisted failure ([behaviour/output/child-processes]).
- crumbkit-test: [`RuntimeFailing`] builds failing lines from a code,
  words, paths, input, values and variables that a test gets at run time,
  reuses its settings across lines with `line`, and gives a `Failing`, a
  `Twin` or an `Allow::untwinned` opt-out ([behaviour/adoption]).
- crumbkit: a failure field can hold null, a list or an object, built
  with `FieldValue::list`, `FieldValue::object` or an `Option`, and the
  failure document holds it as a JSON value ([behaviour/schemas]).
- crumbkit: [`Row`] gains `PROBLEM`, which names the field that marks
  a row as reporting a problem ([behaviour/output/listings]).
- crumbkit: a leaf whose row type names a problem field prints a
  `Problems:` line in `-h` and `--help` ([behaviour/output/listings]).
- crumbkit-test: [`check_guide`] fails when a row type's problem field is
  not one of its `COLUMNS`, or when its type has no value that means no
  problem ([behaviour/output/listings]).
- crumbkit: [`Row`] gains `is_problem`, which a row type with a
  computed status overrides in place of a duplicate problem column. Its
  default reads the field that `PROBLEM` names ([behaviour/output/listings]).
- crumbkit: [`help::ViewFlag`] names `--fields`, `--long` and `--full`,
  and `Leaf::offers` reports whether a leaf's document can use one
  ([behaviour/output]).
- crumbkit-test: [`check_guide`] fails on an example that types a view flag
  that its leaf does not offer ([behaviour/output]).
- crumbkit-test: [`check_guide`] fails on a leaf whose effects change
  something and that declares no document, unless it passes on a program's
  exit code, because crumbkit refuses an unusable view flag only after the
  change ([behaviour/output]).
- crumbkit: [`complete_with`] answers a shell's completion request under a
  variable that the tool names, and the registration script and the usage
  log use that name ([behaviour/parsing/completion], [behaviour/adoption/declarations]).
- crumbkit: `Leaf::refusal_alternatives` takes a function that builds
  calls from a refused command line naming the leaf, such as the corrected
  line. The first call whose line parses becomes the recovery, before `-h`
  ([behaviour/adoption/declarations]).
- crumbkit: with `Guide::dashed_forms`, a command named in another style,
  such as `reading_lists`, also answers to its dashed form, such as
  `reading-lists`, in [`parse`], completion and [`command_path`], and
  [`dashed_form`] returns that form ([behaviour/adoption]).
- crumbkit: [`is_dashed`] reports whether a name is lowercase letters and
  digits in words joined by single dashes, the form of a command's name in
  crumbkit's grammar, and crumbkit-test's checks call it.
- crumbkit: `RuntimeLeaf::refusal_alternatives` gives a command that a
  tool adds at run time the alternatives that a refusal of its line offers
  ([behaviour/help]).
- crumbkit: `RuntimeLeaf::document` declares the document that a command
  added at run time prints, so its help lists the columns and `commands
  list` and `schemas list` describe it as a `const` leaf with that document
  ([behaviour/help]).
- crumbkit-test: `RuntimeFailing::at` makes a place that holds a working
  directory and no command line, and `RuntimeFailing::get_dir` returns its
  directory ([behaviour/adoption]).
- crumbkit-test: `RuntimeFailing::variable_twin` makes a variable twin
  from a variable name that the test builds at run time ([behaviour/checks/end-to-end]).
- crumbkit: [`complete_with_words`] passes the words of the shell's command
  line after the program name to the function that builds the command, so
  completers can depend on an earlier flag such as `-C <DIR>` ([behaviour/parsing/completion]).
- crumbkit: `Guide::refusal_alternatives` takes a function that builds
  refusal alternatives for every command whose leaf sets none, and a leaf's
  own function replaces it ([behaviour/adoption/declarations]).
- crumbkit-test: [`check_opt_outs`] lists the guide's refusal alternatives
  once under customisations ([behaviour/adoption/declarations]).
- crumbkit: `Output::room` returns a `Room` whose `has_room` measures a
  batch's rows in the format the run prints and returns true until the
  batch holds a row, so a handler stops fetching keys whose rows would not
  print ([behaviour/output/budget]).
- crumbkit: `Call::all` adds an argument that takes every value of a
  list field of the failure in one run, which the `run` step names in
  `all`. Rendering refuses an argument that takes one value or a placeholder
  beside it ([behaviour/schemas]).
- crumbkit-test: [`check_recoveries`] follows a `run` step with `all` once,
  with every value of the named field, and accepts the same code again when
  fewer values are left ([behaviour/checks/recovery-chains]).
- crumbkit-test: `Pages::clap` reads each help page as clap prints it, so
  the help checks run on a tool that parses with clap alone ([behaviour/help/checks]).
- crumbkit-test: `PathBudgets::help_only` budgets each path by its `-h`
  pages alone, for a tool without example golden files ([behaviour/help/checks]).
- crumbkit-test: crumbkit-test re-exports `Guide`, so a tool without a
  crumbkit dependency can run `check_names` and `check_grammar` ([behaviour/adoption]).
- crumbkit-derive: the new crate's `#[derive(Failure)]` writes a
  [`Failure`] impl and a `CODES` list from `#[failure(...)]` attributes.
  The `derive` feature of crumbkit re-exports it ([crumbkit-derive],
  [behaviour/failures/derive]).
- crumbkit: [`Error::unclassified`] holds an error that the tool did
  not classify, such as one from an error library, with the code
  `crumbkit::report::unchecked` and no next step ([behaviour/failures]).
- crumbkit: [`codes`] gains `REPORT_UNCHECKED`, the code of an unclassified
  error and of a converted `miette::Report`, and `codes::ALL` lists it.

- crumbkit: a batch failure holds the `key` it failed on and that key's
  `index` in JSON, and prints a `key:` line in text. The failure document's
  schema holds both as optional fields ([behaviour/output]).
- crumbkit: the over-budget failure that ends a batch holds `left_out_from`,
  the index of the first key left out ([behaviour/output/budget]).
- crumbkit: [`events::observe`] installs one run observer for the process,
  which `parse`, `finish`, streams and child processes tell what the run
  did, and the usage log is such an observer ([`events`], [behaviour/run-observer]).
- crumbkit: `FieldValue::from_json` reads a failure field back from a
  `serde_json::Value`, and returns `None` for a number that `FieldValue`
  cannot hold, such as a fraction ([behaviour/adoption]).
- crumbkit: `Leaf::codes` renames crumbkit's codes for the runs of one
  command, which read the leaf's renames before the guide's. A leaf cannot
  rename `crumbkit::cli::usage` ([behaviour/adoption/declarations]).
- crumbkit-test: [`check_exit_codes`] reports a leaf that renames one code
  twice, a tool code that renames of two crumbkit codes share across the
  guide and its leaves, and two exits that a leaf's renames make list one
  code ([behaviour/adoption/declarations]).
- crumbkit-test: [`check_opt_outs`] lists each leaf's code renames as
  customisations of the leaf's command ([behaviour/adoption/declarations]).
- crumbkit: `Headed` and `Document::headed` mark a batch whose rows
  hold bytes, and `finish` prints each row's bytes as `head` prints files,
  under a `==> KEY <==` header when there are several keys
  ([behaviour/output]).
- crumbkit: each answer of a headed batch's text prints whole within the
  output budget, and one failure names the answers past the budget
  ([behaviour/output/budget]).
- crumbkit: the JSON of a headed batch holds every field of each row unless
  `--fields` names some. `--fields` needs `--json`, and a headed leaf offers
  neither `--long` nor `--full` ([behaviour/output]).
- crumbkit: the help of a headed leaf ends with an `Output:` line and lists
  its JSON fields, including `text` and `base64`, under a heading naming
  `--fields`. `commands list` gives it the form `headed`, and
  crumbkit-markdown's reference prints both ([behaviour/help]).
- crumbkit: `commands list` gives a leaf that prints [`Bytes`] the form
  `bytes`, and crumbkit-markdown's reference prints its `Output:` line as
  help does ([behaviour/help]).
- crumbkit-test: [`check_guide`] fails a headed leaf whose row does not
  flatten its `Bytes` field ([behaviour/help/examples]).
- crumbkit-test: [`check_output_budget`] also runs the text examples of a
  headed leaf with half the bytes they print ([behaviour/output/budget]).
- crumbkit-test: [`check_guide`] fails a leaf whose batch is not headed and
  whose row type has no `Row::COLUMNS` ([behaviour/help/examples]).
- crumbkit: `Output::new` and the builders `json`, `fields`, `long` and
  `full` build the output flags for a tool that defines them in its own
  parser, and `finish` checks them as parsed flags ([behaviour/output]).
- crumbkit: a row type that sets [`Row`]`::WHOLE` prints every field
  of each row with `--json`, without `--full`. `--full` is refused for it,
  and `--fields` prints `null` for a field that a row leaves out
  ([behaviour/output]).
- crumbkit-test: [`check_guide`] fails a row type that sets `Row::WHOLE`,
  names no columns and has fields, and [`check_document_ids`] requires an
  opt-out from `documents/rows-whole` for a leaf whose row type sets
  `Row::WHOLE` ([behaviour/adoption/opt-outs]).
- crumbkit: a [`Row`] that declares `LINE` and `text_line` prints each row
  as its own line in text, with no header. `--long` and `--fields` print the
  table, and help prints a `Text:` line ([behaviour/output]).
- crumbkit: `commands list` gives each command a `line` field, and
  crumbkit-markdown's reference prints the `Text:` line as help does
  ([behaviour/help/commands-list]).
- crumbkit: a row type that sets `Row::WHOLE` and declares `Row::LINE` may
  name no columns. Its text prints each row's own line, and `--fields` and
  `--long` are refused for it ([behaviour/output]).
- crumbkit: `finish` and `stream` fail to compile for a whole row type
  without columns and without a `LINE`, and [`check_guide`] fails a batch
  of it ([behaviour/output]).
- crumbkit-test: [`check_guide`] fails a `Row::LINE` that is not one
  non-empty line or that belongs to a headed leaf, and [`check_opt_outs`]
  lists a `text-line` customisation ([behaviour/help/examples]).

- crumbkit: [`Prose`] and [`Batch`] take a `note`, which text prints on
  standard error where a listing's note prints, before any failures, and
  which the JSON document leaves out ([behaviour/output/listings]).
- crumbkit: `Sink::styled` makes a [`Sink`] ask for styles, and with the
  new `color` feature `parse_to` writes help to it with clap's styles.
  `parse` styles its sink when standard output is a terminal
  ([behaviour/parsing], [behaviour/output/terminal]).
- crumbkit: [`help::Group`]`::compact` makes a group's `-h` list each goal's
  subcommand names on one line, such as `Read:  list, show`, while `--help`
  keeps the full list ([behaviour/help]).
- crumbkit: `Exit::range` declares a range of exit codes with one meaning
  for a command that passes on another program's code, and `help
  exit-codes`, the `commands list` note and the reference print it as
  `124-125` ([`help::Exit`], [behaviour/failures/exit-codes]).
- crumbkit: each exit of a `commands list` row has `last`, the last code of
  a range, which equals `exit` for a single code
  ([behaviour/help/commands-list]).
- crumbkit-test: [`check_exit_codes`] fails a range that starts below 3,
  ends before it starts, lists failure codes or holds a code that another
  declaration holds ([behaviour/failures/exit-codes]).
- crumbkit-test: an opt-out from `Rule::GuideExamples` lets
  [`check_guide`] pass a command without an example, and every command
  below it, for a stated reason ([behaviour/help/examples],
  [behaviour/adoption/opt-outs]).
- crumbkit-test: an opt-out from `Rule::GuideDocument` lets
  [`check_guide`] pass a leaf that changes something and declares no
  document, and every leaf below it, for a stated reason
  ([behaviour/help/examples], [behaviour/adoption/opt-outs]).
- crumbkit: `Refused::error` returns clap's error for a refused command
  line, so a tool reads its kind and context, such as the argument that
  conflicted or was repeated ([behaviour/parsing]).
- crumbkit: `Stop::printed` returns a [`Printed`] that says whether help,
  the version, a topic, a refusal or a picker's failure printed before
  `parse` ended the run ([behaviour/parsing]).
- crumbkit: `Refused::path`, `Refused::flags` and `Refused::value` return
  the path to the deepest command, the ids of the flags typed and the last
  value of an option, such as a global `--format` ([behaviour/parsing]).
- crumbkit: an [`Age`] row field prints a compact age such as `3h` in the
  text table and a stream's text lines, and an RFC 3339 time in JSON
  ([behaviour/output]).
- crumbkit: `Stop::path`, `Stop::flags` and `Stop::value` return the path,
  the flags typed and an option's value of a run that `parse` ended, such
  as after a help page, as `Refused` does ([behaviour/parsing]).
- crumbkit: `Row::CHILDREN` and `Row::child_lines` print a row's children
  as indented lines under it in text, while compact JSON keeps the field,
  and help, `commands list` and crumbkit-test describe and check it
  ([behaviour/output]).
- crumbkit: `Leaf::refusal_hint` and `Guide::refusal_hint` let a refused
  command line recover with the tool's own sentence, before `-h` and the
  tool's calls, and `RefusalContext::command` returns the leaf's command
  ([behaviour/failures]).
- crumbkit: `Refused::as_json` fixes whether a refusal that the tool prints
  is the JSON failure document on standard output or text on standard
  error, whatever the words said ([behaviour/parsing]).
- crumbkit: a refusal and its group page carry clap's styles on standard
  error at a terminal or with `CLICOLOR_FORCE`, never under `NO_COLOR`, and
  `Sink::styled_errors` asks for them on a tool's own sink
  ([behaviour/parsing]).
- crumbkit: `Listing::capped` marks a listing whose tool read only part of
  its source, so its footer gives the rows read as a lower bound and its
  JSON document leaves out `total` ([behaviour/output/listings]).
- crumbkit: `Sink::fitted` gives a tool's own sink a width, and
  `finish_to` fits a text table to it as `finish` fits one to the terminal.
  `Sink::fitted_width` reads it ([behaviour/output/terminal]).

### Changed

- crumbkit: text output escapes each Unicode format character, such as a
  bidirectional override or a zero-width space, as it escapes control
  characters, in table cells, rows' text lines, failures, headed keys and
  picker labels ([behaviour/output]).
- crumbkit: a document's JSON Schema describes what the tool prints. A field
  skipped when empty is optional, a field always printed is required even
  with a serde default, and a row requires only its columns
  ([behaviour/schemas]).
- crumbkit: the schema of a document that prints only the row's columns
  admits null for a column that the row type skips when it is empty, and
  that column stays optional ([behaviour/schemas]).
- crumbkit: [`Output::new`] returns `Result<Output, Stop>` and refuses
  `--full` without `--json` and `--fields` with `--long` or `--full`
  before the handler runs; [`finish`] and [`stream`] no longer refuse
  them ([behaviour/output]).
- crumbkit: [`Output::new`] and `Output::new_to` take the command's
  [`DryRun`] after the view flags, so an `Output` knows its dry run from
  the start ([behaviour/effects]).
- crumbkit: the `derive` feature re-exports `#[derive(Failure)]` from
  crumbkit-derive itself, and the `miette` feature holds `Failure::diagnostic`
  and the conversion of a `miette::Report`, which the removed crumbkit-core
  held behind features of its own ([behaviour/failures/derive]).
- crumbkit: a tool sets its code spelling once with `Guide::spelling`,
  which defaults to `Spelling::COLONS`. [`parse`], [`finish`], [`stream`],
  `hand_over` and crumbkit-test's checks no longer take a `Spelling`, and
  [`check_codes`] takes the guide ([behaviour/failures]).
- crumbkit: a batch whose document does not set `always_print_first`
  refuses a first answer that passes the output budget alone with its own
  failure, and names every later answer in the one over-budget failure, in
  text and JSON ([behaviour/output/budget]).
- crumbkit: a tool's output names the tool where it named crumbkit. A tool
  named `shelf` prints `shelf/failure/v1` and `shelf::cli::usage`, and reads
  `SHELF_OUTPUT_BUDGET` in place of `CRUMBKIT_OUTPUT_BUDGET`
  ([behaviour/failures]).
- crumbkit: `ToolName` spells crumbkit's codes, document ids and
  variables with a tool's name, and `Spelling::for_tool` prints codes with
  it. A hyphen stays in an id and becomes `_` in a code. A variable
  starts with `_` before a leading digit.
- crumbkit-test: the checks expect the tool's names in place of
  `crumbkit/...` ids and `crumbkit::` codes. The recovery check follows
  crumbkit's and its add-on crates' codes in the tool's component, and
  the checks fail only on crumbkit's own warnings.
- crumbkit-usage: the usage log records `<tool>/usage/v1`, self-usage serves
  `<tool>/self-usage/v1`, and the report reads records under any component.
  The checked-in record schema accepts any tool's id.
- crumbkit-child: the hand-over failure codes print with the tool's
  component, such as `shelf::child::failed`.
- crumbkit-aliases: the aliases failure code prints with the tool's
  component, such as `shelf::aliases::unreadable`, and the new `codes`
  module lists it.
- crumbkit-markdown: the reference reads a `commands-list/v1` document under
  any component, and its header no longer names crumbkit.
- cargo-crumbkit: `weigh --usage` and `docs` read the `self-figures-list/v1`
  and `commands-list/v1` documents that a tool prints under its own name.
- crumbkit: [`parse`] returns the tool's type and an [`Output`] that holds
  the command tree, the typed words and the leaf's code renames. A tool
  flattens [`ViewFlags`] in place of `Output` ([behaviour/parsing]).
- crumbkit: [`Page`] refuses 0, a word or a negative `--limit` with a
  sentence that states the range, such as `--limit takes a whole number
  from 1 to 100` ([behaviour/output/listings]).
- crumbkit: a tool that parses with clap alone builds its [`Output`] with
  `Output::new(view, dry_run, command, guide, args)`, which finds the leaf
  that the argv names, so the run prints that leaf's code renames
  ([behaviour/adoption/declarations]).
- crumbkit: `Output::print_failure`, `Output::failure_exit`,
  `Output::listed_exit`, `Output::refuse_view` and
  `Output::refuse_fields_for_text` are public, so a crate that prints a
  response without [`finish`] prints failures, chooses exit codes and
  refuses view flags as `finish` does ([`Output`]).
- crumbkit: [`Sink`]`::standard` is public, so such a crate prints to the
  standard streams without opening them itself.
- crumbkit: [`events`]`::handed_over`, `taken_back`, `interrupted` and
  `handles_interrupt` are public and need no feature, so a crate that hands
  the terminal to a child reports it to the observer
  ([behaviour/run-observer]).
- crumbkit-test: [`check_twins`] needs no twin and no declaration for a
  code in the `crumbkit` component that `crumbkit::codes::ALL` does not
  list, such as crumbkit-child's `crumbkit::child::not_found`, and
  [`check_recoveries`] follows the recovery of such a code
  ([behaviour/checks/end-to-end], [behaviour/checks/recovery-chains]).
- crumbkit: `Exit::covers` is public, so crumbkit-usage can give each exit
  code in its reports its meaning ([behaviour/failures/exit-codes]).
- crumbkit: [`complete_to`] takes the function that builds the command from
  the typed words in place of a built command ([behaviour/parsing/completion]).
- crumbkit-test: [`check_opt_outs`] lists code renames,
  exits of crumbkit's codes, passes-on-exit leaves and refusal alternatives
  under a customisations section without reasons, as a `customisations`
  JSON array, and leaves the section out when empty ([behaviour/adoption/declarations]).
- crumbkit: [`finish`], batches and [`Streaming`] exit with the code that
  the guide declares for the run's first failure, and the usage log and
  the `crumbkit.finish` span record that number ([behaviour/failures/exit-codes]).
- crumbkit-test: [`check_twin`] takes the tool's guide as its first
  argument ([behaviour/checks/end-to-end]).
- crumbkit-test: [`check_recoveries`] fills a placeholder from the `fields`
  of the failure that offers the step before it reads `Recoveries::values`
  ([behaviour/checks/recovery-chains]).
- crumbkit-test: [`check_recoveries`] fills each placeholder inside a word,
  such as `<plugin>.<capability>`, and keeps the text between them
  ([behaviour/checks/recovery-chains]).
- crumbkit: the help text of `--json` reads `Print the result as a JSON
  document` and no longer names a layout.
- crumbkit-test: when the guide sets `JsonLayout::Pretty`, [`check_examples`]
  requires a `--json` example of one result to print one indented document
  ([behaviour/output/json-layout]).
- crumbkit-test: [`check_output_budget`], [`check_twin`],
  [`check_recoveries`] and [`check_examples`] read an indented JSON document
  as well as a document on one line.
- crumbkit-usage: a run whose `COMPLETE` variable asks for shell
  completion writes no line ([behaviour/parsing/completion]).
- crumbkit: a build with debug assertions writes a `warning:` line on
  standard error for each recovery call of a failure that does not render,
  with the reason ([behaviour/failures]).
- crumbkit: a guided list ends each description with the command's visible
  aliases, as clap's `Commands:` list does ([behaviour/help]).
- crumbkit-test: [`check_names`] takes the tool's guide and fails on a
  guide that names personal aliases unless an opt-out of
  `Rule::PersonalAliases` covers the root ([behaviour/parsing/aliases]).
- crumbkit: a guided command's help prints the tool's own `before_help`
  and `before_long_help` above the goal list ([behaviour/help]).
- crumbkit-test: [`Examples`] borrows its paths for a lifetime, so a test
  can build one from a path it computes at runtime.
- crumbkit: each argument row of `commands list` has a `one_of` field that
  names the arguments that can be given in its place: the other members of
  its required groups and, for a required argument, the arguments it
  conflicts with ([`commands`]).
- crumbkit-test: [`check_listings`] accepts a `show` command whose ids sit
  in a required argument group.
- cargo-crumbkit: `weigh` prints `long_help` only with `--long`
  ([behaviour/help/weight]).
- crumbkit: `Output::command`, `commands::list`, `schemas::list` and
  `schemas::show` take `&mut Output`, and `Output::set_command` no longer
  builds the command tree, so a run builds only the commands it reads
  ([behaviour/adoption]).
- crumbkit: [`finish`], `finish_to`, [`stream`] and `stream_to` take the
  [`Output`] by value, so a second `finish` with one `Output` does not
  compile, and a `Streaming` owns its view and command tree
  ([behaviour/output]).
- crumbkit: `Output::budget` returns the whole output budget, because an
  `Output` prints one result ([behaviour/output/budget]).
- crumbkit-child: [`child::hand_over`] takes the `Output` by value, as
  `finish` does ([behaviour/output/child-processes]).
- crumbkit-aliases: `list` takes `&mut Output`, because it builds the
  tool's command tree when it runs.
- crumbkit: [`parse`] returns a [`Stop`] after help, the version or a
  refusal, which holds the exit code and an `Output` ([behaviour/adoption]).
- crumbkit: `<tool> help` prints what `<tool> --help` prints, whose
  `Topics:` line names each topic ([behaviour/help]).
- crumbkit: run observers hear of a refusal when it prints, so a refusal
  that the tool answers itself reports no `refused` event
  ([behaviour/run-observer]).
- crumbkit-test: [`check_grammar`], [`check_names`], [`check_listings`],
  [`check_filters`] and [`Examples`] take opt-outs, each an [`Allow`] that
  names one rule, a command and a reason, in place of exempt command paths
  ([behaviour/adoption/opt-outs]).
- crumbkit-test: [`check_grammar`] passes a leaf directly under the root,
  a resource name or a command's own `--dry-run` that an opt-out names
  ([behaviour/adoption]).
- crumbkit-test: [`Budgets`] and [`PathBudgets`] take a reason with each
  budget, a command's budget is a [`Budget`], and a tool can raise a
  budget above crumbkit's default ([behaviour/help/checks]).
- crumbkit: a [`Document`] component may hold dots and dashes, so a
  domain such as `example.dev` names a document's owner ([behaviour/schemas]).
- crumbkit-test: [`check_document_ids`] takes opt-outs from the document
  name rule, and [`check_schema_goldens`] fails on two ids whose golden
  files share a name ([behaviour/schemas/commands]).
- crumbkit: each row of `schemas list` has a `kind` field and column, which
  is `output`, `unwrapped` or `file` ([behaviour/schemas/commands]).
- crumbkit-test: [`check_document_ids`] fails on a document without
  crumbkit's envelope unless an opt-out of `Rule::Envelope` names its
  command ([behaviour/adoption/opt-outs]).
- crumbkit-test: [`check_schema_goldens`] pins the schemas of file formats
  and fails on a file format with the id of another document ([behaviour/schemas/commands]).
- crumbkit-usage: `findings` and `figures` take the tool's
  `Guide`, which gives the meaning of each exit code ([behaviour/usage-log/report]).
- crumbkit: [`parse`] returns a `Parsed` in place of the `Command`. It
  holds the command and the typed words, and `Output::set_command` takes
  either ([behaviour/adoption]).
- crumbkit: [`Sink`] has a third type parameter for standard input, and
  `Sink::with_input` builds one for tests.
- crumbkit-test: [`check_grammar`] no longer checks `--dry-run`, which
  [`check_effects`] checks.
- crumbkit: `Output::confirm` asks only when no coding agent is detected,
  and otherwise fails as it does without a terminal ([behaviour/effects/pickers]).
- crumbkit: crumbkit enables clap's `error-context` feature, which adds
  about 33 KB to a tool's release binary ([project/dependencies]).
- cargo-crumbkit: the failures of building and running a tool's binary
  have `crumbkit::tool::*` codes, such as `crumbkit::tool::build_failed`,
  which `weigh` and `docs` share ([behaviour/help/weight]).
- crumbkit-aliases: `aliases list` lists the entries with a problem first,
  so a page cut short by `--limit` still shows them ([crumbkit-aliases]).
- crumbkit: the confirmation codes are `crumbkit::confirmation::declined`
  and `crumbkit::confirmation::needs_yes`, whose area is a noun ([behaviour/failures]).
- cargo-crumbkit: the failures of `weigh` have `crumbkit::weight::*` codes,
  such as `crumbkit::weight::help_failed` ([behaviour/help/weight]).

- crumbkit-test: [`check_opt_outs`] takes the tool's guide and lists an
  output budget that the guide sets as `output-budget/run` ([behaviour/output/budget]).
- crumbkit: the JSON Schema of each listing and list document holds the
  optional `cut` object, and the documents keep their `v1` ids
  ([behaviour/output/budget]).
- crumbkit: in text, a listing's footer, empty note and note, the `Next:`
  line, the dry-run line, the cut line and a stream's notes print on
  standard error after standard output is flushed ([behaviour/output/listings]).
- crumbkit-test: [`check_examples`] compares an example's standard error
  with its `.stderr` golden file, and fails on standard error only for a
  `warning: crumbkit ` line ([behaviour/checks/end-to-end]).
- cargo-crumbkit: `weigh` adds the size of an example's `.stderr` golden
  file to the command's output weight ([behaviour/help/weight]).
- crumbkit: every listing's JSON document holds `truncated`, which is true
  when the document leaves out rows because of `--limit` or the output
  budget, and false when it holds every row ([behaviour/schemas]).
- crumbkit-test: [`check_opt_outs`] gives the cost of a
  `documents/envelope` opt-out as a document without `truncated` in place
  of `total`.
- crumbkit: every listing's JSON Schema marks `total` as optional, and the
  documents keep their `v1` ids, because no schema has been released
  ([behaviour/schemas]).
- crumbkit-markdown: the reference refuses a `commands list` document that
  leaves out `total` or holds `"truncated": true`, because it would leave
  commands out.
- crumbkit: with the `miette` feature, a text failure prints one source
  line before and after each label's lines, where the source has them
  ([behaviour/output]).
- crumbkit: with the `miette` feature, a text failure prints every line of
  a span over several lines, with carets on its first and last line, and
  counts the lines left out of a span over more than 7 lines ([behaviour/output]).
- crumbkit: with the `miette` feature, labels on the same or nearby lines
  print their `at:` lines together and share their source lines, and each
  underline ends with its label's text ([behaviour/output]).
- crumbkit: with the `miette` feature, the carets under a label measure
  terminal columns with unicode-width, so they line up after wide
  characters such as CJK text ([behaviour/output]).
- crumbkit-test: [`check_recoveries`] fills a lowercase placeholder such
  as `<path>` from the value of the same name ([behaviour/checks/recovery-chains]).
- crumbkit-test: [`check_recoveries`] fills placeholders and reports
  `recoveries/field-value` from the fields of the failure that a reader
  returns ([behaviour/checks/recovery-chains]).
- crumbkit-test: [`check_recoveries`] follows every failure that a run
  prints, such as each entry of a batch, and a start passes when any of
  its failures carries its code ([behaviour/checks/recovery-chains]).
- crumbkit-test: [`check_recoveries`] compares a step's code with the code
  of every earlier failure of its chain, so a chain that returns to an
  earlier code through another code fails the check ([behaviour/checks/recovery-chains]).
- crumbkit-test: [`check_recoveries`] reads a failed run's last `code:`
  line when the run prints no failure document ([behaviour/failures/excerpts-and-readers]).
- crumbkit-test: [`check_twins`] adds `--json` to both lines of each twin
  when neither line holds it, as [`check_recoveries`] does, so a tool's
  reader sees the failure an agent reads ([behaviour/checks/end-to-end]).
- crumbkit-test: [`check_twin`] refuses a twin whose admitted line adds a
  help or version word, or runs another command than the refused line
  ([behaviour/checks/end-to-end]).
- crumbkit-test: a check's run whose tool exits while a process it started
  keeps standard output or standard error open ends with a problem at the
  run's timeout, and the check no longer waits for that process ([behaviour/checks/end-to-end]).
- crumbkit-test: [`check_twin`] and [`check_twins`] take the tool's command
  tree first and read each line's command from it, so a global option
  before the path and an alias name the same command ([behaviour/checks/end-to-end]).
- crumbkit-test: [`check_twin`] refuses a twin in which neither line runs a
  command, so a twin of the misspelt `books lst` admits `books list` and
  not `books` ([behaviour/checks/end-to-end]).
- crumbkit: an `Exit` may list one of crumbkit's failure codes or the
  tool's rename of it, so a refused command line can exit with the tool's
  code ([behaviour/adoption/declarations]).
- crumbkit: `help exit-codes` and the `commands list` note leave out
  crumbkit's 2 when a refusal exits with another code ([behaviour/failures/exit-codes]).
- crumbkit-usage: `self usage` and `self figures list` count a run as refused
  when it exited with the code that the guide gives a refused command line
  ([behaviour/usage-log/report]).
- crumbkit-test: [`check_exit_codes`] accepts an exit that lists one of
  crumbkit's codes, and refuses the exit code 1 or 2 without an opt-out of
  `Rule::SharedExit`. [`check_twin`], [`check_recoveries`] and
  [`check_output_budget`] expect the codes and exits that the guide
  declares ([behaviour/adoption/declarations]).
- crumbkit: [`finish`] and [`stream`] exit with 1 when a row reports a
  problem and the run has no failure, and the text ends with a note that
  counts those rows ([behaviour/output/listings]).
- crumbkit: `help exit-codes` says that 1 also means a row of the result
  reported a problem ([behaviour/output/listings]).
- crumbkit-aliases: `aliases list` exits with 1 while an entry of the
  aliases file has a problem ([behaviour/parsing/aliases]).
- crumbkit: a leaf's help leaves out each view flag that its document
  cannot use, and [`parse`] refuses such a flag as a refused command line
  before the handler runs ([behaviour/output]).
- crumbkit: [`finish`] and [`stream`] refuse `--long` and `--full` for
  prose, `--long` for rows without long columns, and `--full` for a
  document without the envelope ([behaviour/output]).
- crumbkit: `complete_to` takes the name of the completion variable, which
  the registration script sets ([behaviour/parsing/completion]).
- crumbkit: `Batch::new` takes the command, the argument and the keys
  that the caller gave, and the handler pushes one result for each key, so
  each failed key has its own failure ([behaviour/output]).
- crumbkit: a batch past the output budget ends with a failure whose
  `left_out` lists the keys it left out in input order, and whose recovery
  runs the command once with those keys. `unlisted` counts the rest
  ([behaviour/output/budget]).
- crumbkit: a batch whose handler stopped before its last key ends with the
  over-budget failure that lists the keys it never fetched, even when its
  rows fit ([behaviour/output/budget]).
- crumbkit: [`complete`], [`complete_with`], [`complete_with_words`],
  `complete_to` and [`command_path`] take the tool's guide, so they read its
  `Guide::dashed_forms` declaration as [`parse`] does ([behaviour/adoption]).
- crumbkit-test: [`check_grammar`] takes the tool's guide, and with
  `Guide::dashed_forms` it accepts a resource or verb named in another style
  that answers to its dashed form without an opt-out of
  `Rule::ResourceName` ([behaviour/adoption]).
- crumbkit-test: [`check_opt_outs`] lists the `Guide::dashed_forms`
  declaration and each command that answers to its dashed form as
  customisations ([behaviour/adoption/declarations]).
- crumbkit: [`ReadFailure`] holds each field as a `FieldValue`, so a
  [`FailureReader`] gives back a null, list or object field as the failure
  held it ([behaviour/failures/excerpts-and-readers]).
- crumbkit: [`help::RuntimeLeaf`] no longer implements `PartialEq` and
  `Eq`, because it holds the function of its refusal alternatives.
- crumbkit-test: `RuntimeFailing::at` returns a `RuntimePlace`, which has
  no code or words and no method that turns it into a `Failing`, so a place
  used as a command line is a compile error ([behaviour/adoption]).
- crumbkit-test: the failure reader leaves out a field of crumbkit's
  failure document that holds a fraction, which it read as text before.
- crumbkit: `Guide::printed` and `Guide::failure_exit` take the command
  path of the run, because a leaf's renames apply to its own command
  ([behaviour/adoption/declarations]).
- crumbkit: the schema of [`Bytes`] is one object with `text` and
  `base64` and a `oneOf` that requires one of them, so a flattened `Bytes`
  field lists both among the row's properties ([behaviour/schemas]).
- crumbkit-test: [`check_output_budget`] accepts a run that prints past
  the budget only when its document always prints its first answer, and
  accepts exit code 0 past the budget only for one such answer
  ([behaviour/output/budget]).
- crumbkit: [`finish`] fails to compile for a row, a `Vec` or a `Listing`,
  and [`stream`] for a row type, whose [`Row`]`::COLUMNS` is empty, because
  such rows printed none of their fields ([behaviour/output]).
- crumbkit: [`finish_to`] returns the exit code as a `u8`, so a tool can
  record the code before it exits ([behaviour/output]).
- crumbkit: the one over-budget failure of a batch names every key left out
  when the first row and a failure that names no key already pass the budget
  ([behaviour/output/budget]).
- crumbkit: [`events::observe`] installs up to four observers, and each
  report reaches every installed observer in install order. A tool's own
  observer and the usage log both hear each run ([behaviour/run-observer]).
- crumbkit-usage: `start` after a tool installed its own observer records
  the run, and a second `start` installs the log's observer no second time
  ([behaviour/usage-log]).
- crumbkit-test: [`check_names`] and [`check_grammar`] take [`Pages`] in
  place of a guide. A tool that parses with clap alone passes
  `Pages::clap()`, and a guided tool passes `Pages::rendered` or
  `Pages::binary` with its guide ([behaviour/checks]).
- crumbkit-test: [`check_grammar`] skips hidden commands, as help does, so
  a hidden leaf such as `__query` needs no verb. A grammar opt-out naming a
  hidden command now fails as stale; remove it. A hidden `help` still fails
  ([behaviour/checks]).
- crumbkit-test: the first line of the opt-out report counts budgets apart
  from opt-outs, such as `shelf has 3 opt-outs from the conventions and sets
  1 budget.` ([behaviour/adoption/opt-outs]).
- crumbkit-test: [`check_exit_codes`] passes a single exit code from 129
  to 159, which a run that a signal ended returns, without an opt-out. An
  opt-out of `exit-codes/shell-code` for such a code now fails as stale;
  remove it ([behaviour/failures/exit-codes]).
- crumbkit-test: [`check_guide`] takes the tool's opt-outs as a third
  argument. A tool without opt-outs passes `&[]`
  ([behaviour/help/examples]).
- crumbkit-test: [`check_guide`]'s document rule applies only to a leaf
  whose command takes a global `--json`, `--fields`, `--long` or `--full`.
  A `guide/document` opt-out for any other leaf now fails as stale; remove
  it ([behaviour/output]).
- crumbkit: the refusal of a command line that stops at a group names the
  group's visible subcommands, so `shelf books` prints `needs a subcommand:
  list, show, summarize, remove` ([behaviour/failures]).
- crumbkit: the refusal of a command line that leaves out a required
  argument with possible values lists them after the argument, as in
  `<SHELL> [possible values: bash, fish, zsh]`, as an invalid value does
  ([behaviour/failures]).
- crumbkit: in text, a command line that stops at a group prints the
  group's `-h` page, and a refusal from clap prints clap's usage as
  `usage:` lines under `error:`. The failure document is unchanged
  ([behaviour/failures]).
- crumbkit: the text table that `finish` prints leaves out a column whose
  field is null or missing in every shown row. A column that `--fields`
  names still prints, and `--json` and a stream's text are unchanged
  ([behaviour/output]).
- crumbkit: a text table at a terminal narrows its widest columns to fit
  the terminal's width and ends each cut cell with `…`. `Row::UNCUT_COLUMNS`
  names the columns that it never cuts ([behaviour/output/terminal]).
- crumbkit: a refusal on standard error stays plain at a terminal whose
  `TERM` is `dumb` and under `CLICOLOR=0`, as help does, unless
  `CLICOLOR_FORCE` forces colour ([behaviour/parsing]).

### Removed

- crumbkit-core: the crate is gone. crumbkit holds every type it held, each
  at the path at which crumbkit re-exported it, so a tool built on crumbkit
  changes nothing ([behaviour/adoption]).
- crumbkit: `Output::set_command`, `Parsed` and `Parsed::guided` are gone,
  and [`Output`] no longer derives `clap::Args` or `Default`, because every
  `Output` that a tool gets already holds the command ([behaviour/parsing]).
- crumbkit: the codes `crumbkit::commands::unavailable` and
  `crumbkit::schemas::unavailable` are gone, because every `Output` holds a
  command.
- crumbkit-aliases: the code `crumbkit::aliases::unavailable` is gone,
  because every `Output` holds a command ([crumbkit-aliases]).
- crumbkit: `events::observer` is gone, because a process can hold several
  observers ([behaviour/run-observer]).
- crumbkit-test: `Rule::DocumentName` is gone. A document named other than
  its command's path is a declaration, which [`check_document_ids`]
  accepts and [`check_opt_outs`] lists as a `document-name` customisation
  ([behaviour/adoption/declarations]).
- crumbkit: `help::split` is gone. [`check_recoveries`] reads a recovery's
  words with its own private reading, which also reads double-quoted words
  and refuses a `$` or a backtick between double quotes ([behaviour/checks/recovery-chains]).
- crumbkit: `Document::over_budget` and `Document::over_budget_code`
  are gone, because a rename of `crumbkit::output::over_budget` in
  `Guide::codes` sets the code for every document ([behaviour/output/budget]).
- crumbkit: `Call::word` is gone, because `Call::value` adds a runtime value
  and checks it against the tool's command tree ([`Call`]).
- crumbkit-child: `child::hand_over_json`, `Json` and the codes
  `crumbkit::child::no_json`, `crumbkit::child::bad_json` and
  `crumbkit::child::failed` are gone. [`child::hand_over`] refuses `--json`
  with `<tool>::cli::usage`, and a tool reads a child's JSON itself
  ([behaviour/output/child-processes]).
- crumbkit-usage: `start_with`, `Signals` and `section` are gone, with the
  record's `ext` field. `start` writes the line when SIGINT or SIGTERM ends
  the run, and a tool that handles either signal itself runs without the
  log ([`usage`], [behaviour/usage-log]).
- crumbkit-picker: the crate is gone, and its `Menu` is [`pick::Menu`] in
  crumbkit, so a tool with a picker depends on crumbkit alone
  ([behaviour/effects/pickers]).
- crumbkit: the topic list that `<tool> help` printed is gone, with its
  `Next: <tool> help <TOPIC>` line. crumbkit prints a topic's summary
  nowhere ([behaviour/help]).

### Fixed

- crumbkit: the compact schema of an enum row gives a variant without a
  column or the children field a property that admits null, so the
  compact document matches even when the variant refuses unknown fields
  ([behaviour/schemas]).
- crumbkit: a refusal's `-h` recovery names the deepest typed command that
  has a help flag, and offers no step when no command on the path has one.
  Before, it could name a `-h` call that clap refuses ([behaviour/failures]).
- crumbkit: a refusal prints as a JSON document, and `Output::is_json` of
  its [`Stop`] is true, only when a command on the typed path declares
  `--json`. An undeclared `--json` counted before ([behaviour/parsing]).
- crumbkit: a `help` command line that ends with flags, such as
  `<tool> help --json`, prints the help of the command it names, as
  `<tool> --help --json` does. A root or group without the flag refused
  the line before ([behaviour/help]).
- crumbkit: `<tool> help <TOPIC> --name value` prints the topic, because
  crumbkit reads the words after `help` with the root's options, so an
  option's separate value names no topic. The line was refused before
  ([behaviour/help]).
- crumbkit: a [`Listing`] from `Listing::uncounted` or [`Page::uncounted`]
  never states a total in JSON or text, even when it holds every row or the
  output budget cut it ([behaviour/output/listings]).
- crumbkit: a batch's one over-budget failure names the first later answer
  that does not fit, every answer after it and every unfetched key, in text
  and JSON alike. A budget of 1 reads `1 byte` ([behaviour/output/budget]).
- crumbkit: when the handler returns a failure, [`finish`] refuses only
  conflicting view flags and otherwise prints the failure. [`parse`] refuses
  an unknown `--fields` name before the handler runs on a leaf with a
  document ([behaviour/output]).
- crumbkit: the over-budget recovery of a batch, and each per-key recovery,
  reruns the caller's words with the keys left out, keeping `--json`,
  `--fields`, filters and global options ([behaviour/output/budget]).
- crumbkit: a [`Listing`] that `Listing::from_all` or [`Page`]'s
  `first_of` cut to its limit counts the rows that report a problem among
  every row, so the exit and the note include the cut rows
  ([behaviour/output/listings]).
- crumbkit: the `Output` inside a [`Stop`] uses the guide's output budget
  in place of crumbkit's default.
- crumbkit: a debug build of a tool whose `main` never calls
  `Output::set_command` writes a warning from `finish` and `stream`, and
  crumbkit-test's `check_examples` and `check_recoveries` fail on it
  ([behaviour/adoption]).
- crumbkit: `--full` without `--json`, and `--fields` with `--long` or
  `--full`, are refused in crumbkit's words with a hint that names the flag
  to add or drop, in place of clap's error ([behaviour/output]).
- crumbkit: a command without a doc comment that flattens `Output`,
  `Page`, `NoInput`, `DryRun`, `Yes` or `Expect`, or takes one of
  crumbkit's argument types, shows no description instead of crumbkit's
  rustdoc ([behaviour/adoption]).
- crumbkit-test: [`check_twins`] requires no twin for a declared code that
  the guide's `Rename` prints in place of one of crumbkit's codes, as its
  docs said ([behaviour/checks/end-to-end]).
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
  one problem ([behaviour/help/checks]).
- crumbkit: the `-h` of a headed leaf whose row type names no column
  ends with the `Next:` line that points at the JSON fields that
  `--help` lists ([behaviour/help/examples]).
- crumbkit-test: [`check_examples`], [`check_twin`] and [`check_recoveries`]
  report each run that their timeout stopped as stopped, where a stop up to
  1 ms early showed as a failed run that ended with SIGKILL ([behaviour/checks/end-to-end]).
- crumbkit-test: [`check_output_budget`], [`check_command_reference`] and
  the help checks that run the binary say when their timeout stopped a run,
  and a run without input reads an empty, closed standard input.
- crumbkit-test: the checks that run the binary write standard input on
  their own thread, so a run that never reads an
  input larger than a pipe holds is stopped at its timeout instead of
  blocking the check ([behaviour/checks/end-to-end]).
- crumbkit-usage: a trim of the usage log keeps the newest whole lines within a
  size budget, drops a torn line, and replaces the log in one step under a
  separate lock file ([`usage`], [behaviour/usage-log]).
- crumbkit-usage: a usage-log key file goes into place only when complete, and a
  key of the wrong length is replaced ([behaviour/usage-log]).
- crumbkit-usage: a signal or the end of `main` during the usage log's write
  waits a bounded time for the line to finish ([behaviour/usage-log]).
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
- crumbkit-child: a tool that starts the usage log with `Signals::Tool` keeps
  handling Ctrl-C itself after [`child::hand_over`] has run a child
  ([behaviour/output/child-processes]).
- crumbkit-child: on Unix, a Ctrl-C sent while [`child::hand_over`] runs a child
  no longer ends a tool with more than one thread, such as one with the
  usage log, when the signal reaches the tool after the child has exited
  ([behaviour/output/child-processes]).
- crumbkit: a refusal from [`parse`] names the real flag or subcommand
  where clap suggests a hidden alias, and names it once.
- crumbkit: a refusal from [`parse`] leaves out each command or flag that
  clap suggests and the tool hid, and leaves out clap's tip when nothing
  else remains.
- crumbkit-usage: the usage log's `nearest` records the real flag when clap
  suggests a hidden alias, and records nothing when clap suggests a hidden
  flag ([`usage`]).
- crumbkit-test: [`check_names`] skips hidden commands, everything below
  them and hidden arguments, because no help page shows them.
- crumbkit: a refusal from [`parse`] prints as JSON only when the walk reads
  `--json` as a flag, so an option's value of `--json` no longer counts
  ([behaviour/help]).
- crumbkit-test: [`check_recoveries`] adds `--json` before a `--` word, so
  the tool reads it as a flag and not as a positional value ([behaviour/checks/recovery-chains]).
- crumbkit-test: a check kills a run whose exit it cannot learn before it
  reports the error, so the run does not outlive the check ([behaviour/checks/end-to-end]).
- crumbkit-test: [`check_examples`] and the other checks that run a tool's
  binary create each run's XDG directories below the canonical temporary
  directory, so their paths hold no symbolic link
  ([behaviour/checks/end-to-end]).
- crumbkit: a refusal from [`parse`] separates several missing required
  arguments with commas, as in `--edit <APP>, <NAME>`, so they no longer
  read as one option that takes two values ([behaviour/failures]).
- crumbkit: `Refused::flags`, `Stop::flags` and their `value` getters read
  the words that the caller typed, so `<tool> help nosuch` reports no
  flags. The word `help` reported a `help` flag before
  ([behaviour/parsing]).

<!-- Each label below points at the source file that holds the item's
rustdoc, or at the page in docs/ that describes the behaviour. -->

[`Row`]: crates/crumbkit/src/row.rs
[`Document`]: crates/crumbkit/src/document.rs
[`Output`]: crates/crumbkit/src/output.rs
[`Output::new`]: crates/crumbkit/src/output/build.rs
[`finish`]: crates/crumbkit/src/output.rs
[`finish_to`]: crates/crumbkit/src/output.rs
[`stream`]: crates/crumbkit/src/output/stream.rs
[`Stream`]: crates/crumbkit/src/stream.rs
[`Streaming`]: crates/crumbkit/src/output/stream.rs
[`Page`]: crates/crumbkit/src/page.rs
[`Listing::uncounted`]: crates/crumbkit/src/listing.rs
[`Page::uncounted`]: crates/crumbkit/src/page.rs
[`Listing`]: crates/crumbkit/src/listing.rs
[`Call`]: crates/crumbkit/src/call.rs
[`Failure`]: crates/crumbkit/src/failure.rs
[`Recovery`]: crates/crumbkit/src/failure.rs
[`Code`]: crates/crumbkit/src/code.rs
[`Spelling`]: crates/crumbkit/src/code.rs
[`Code::runtime`]: crates/crumbkit/src/code/runtime.rs
[`Error`]: crates/crumbkit/src/failure.rs
[`Batch`]: crates/crumbkit/src/batch.rs
[`Prose`]: crates/crumbkit/src/prose.rs
[`Bytes`]: crates/crumbkit/src/bytes.rs
[`Age`]: crates/crumbkit/src/age.rs
[`fetch_each`]: crates/crumbkit/src/fetch.rs
[`report`]: crates/crumbkit/src/report.rs
[`parse`]: crates/crumbkit/src/parse.rs
[`command_path`]: crates/crumbkit/src/parse/walk.rs
[`dashed_form`]: crates/crumbkit/src/dashed.rs
[`is_dashed`]: crates/crumbkit/src/dashed.rs
[`RuntimeFailing`]: crates/crumbkit-test/src/recoveries/runtime.rs
[`Stop`]: crates/crumbkit/src/parse/stop.rs
[`try_parse`]: crates/crumbkit/src/parse.rs
[`Refused`]: crates/crumbkit/src/parse/refused.rs
[`Printed`]: crates/crumbkit/src/parse/stop.rs
[`events::ended`]: crates/crumbkit/src/events.rs
[`events::tolerated`]: crates/crumbkit/src/events.rs
[`events`]: crates/crumbkit/src/events.rs
[`events::observe`]: crates/crumbkit/src/events.rs
[`help`]: crates/crumbkit/src/help.rs
[`help::quote`]: crates/crumbkit/src/help.rs
[`help::ViewFlag`]: crates/crumbkit/src/help/view.rs
[`help::RuntimeLeaf`]: crates/crumbkit/src/help/runtime.rs
[`help::Group`]: crates/crumbkit/src/help/group.rs
[`help::FileFormat`]: crates/crumbkit/src/help/format.rs
[`commands`]: crates/crumbkit/src/commands.rs
[`schemas`]: crates/crumbkit/src/schemas.rs
[`usage`]: crates/crumbkit-usage/src/lib.rs
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
[crumbkit-usage]: crates/crumbkit-usage/README.md
[crumbkit-child]: crates/crumbkit-child/README.md
[crumbkit-derive]: crates/crumbkit-derive/README.md
[`Error::unclassified`]: crates/crumbkit/src/unclassified.rs
[`RenderCall`]: crates/crumbkit/src/call.rs
[cargo-crumbkit]: crates/cargo-crumbkit/README.md
[`example_golden_name`]: crates/crumbkit-test/src/examples.rs
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
[`Pages`]: crates/crumbkit-test/src/pages.rs
[`PathBudgets`]: crates/crumbkit-test/src/weights.rs
[`Effects`]: crates/crumbkit/src/help/effects.rs
[`DryRun`]: crates/crumbkit/src/output/dry_run.rs
[`Yes`]: crates/crumbkit/src/output/confirm.rs
[`ViewFlags`]: crates/crumbkit/src/output/build.rs
[`Sink`]: crates/crumbkit/src/sink.rs
[`check_effects`]: crates/crumbkit-test/src/effects.rs
[`child::hand_over`]: crates/crumbkit-child/src/hand_over.rs
[`child::hand_over_json`]: crates/crumbkit-child/src/hand_over.rs
[`Picker`]: crates/crumbkit/src/pick.rs
[`NoInput`]: crates/crumbkit/src/pick/no_input.rs
[`pick::Menu`]: crates/crumbkit/src/pick/menu.rs
[crumbkit-markdown]: crates/crumbkit-markdown/README.md
[`check_command_reference`]: crates/crumbkit-test/src/reference.rs
[`ReadFailure`]: crates/crumbkit/src/form.rs
[`codes`]: crates/crumbkit/src/codes.rs
[`check_recoveries`]: crates/crumbkit-test/src/recoveries.rs
[`check_twin`]: crates/crumbkit-test/src/twins.rs
[`check_twins`]: crates/crumbkit-test/src/twins/coverage.rs
[`complete`]: crates/crumbkit/src/complete.rs
[`complete_with`]: crates/crumbkit/src/complete.rs
[`complete_with_words`]: crates/crumbkit/src/complete.rs
[`complete_to`]: crates/crumbkit/src/complete.rs
[`Expect`]: crates/crumbkit/src/expect.rs
[behaviour/adoption]: docs/behaviour/adoption/README.md
[behaviour/adoption/declarations]: docs/behaviour/adoption/declarations.md
[behaviour/adoption/opt-outs]: docs/behaviour/adoption/opt-outs.md
[behaviour/checks]: docs/behaviour/checks/README.md
[behaviour/checks/end-to-end]: docs/behaviour/checks/end-to-end.md
[behaviour/checks/recovery-chains]: docs/behaviour/checks/recovery-chains.md
[behaviour/effects]: docs/behaviour/effects/README.md
[behaviour/effects/pickers]: docs/behaviour/effects/pickers.md
[behaviour/effects/write-guards]: docs/behaviour/effects/write-guards.md
[behaviour/failures]: docs/behaviour/failures/README.md
[behaviour/failures/derive]: docs/behaviour/failures/derive.md
[behaviour/failures/exit-codes]: docs/behaviour/failures/exit-codes.md
[behaviour/failures/excerpts-and-readers]: docs/behaviour/failures/excerpts-and-readers.md
[behaviour/help]: docs/behaviour/help/README.md
[behaviour/help/checks]: docs/behaviour/help/checks.md
[behaviour/help/commands-list]: docs/behaviour/help/commands-list.md
[behaviour/help/examples]: docs/behaviour/help/examples.md
[behaviour/help/reference]: docs/behaviour/help/reference.md
[behaviour/help/weight]: docs/behaviour/help/weight.md
[behaviour/output]: docs/behaviour/output/README.md
[behaviour/output/budget]: docs/behaviour/output/budget/README.md
[behaviour/output/child-processes]: docs/behaviour/output/child-processes.md
[behaviour/output/json-layout]: docs/behaviour/output/json-layout.md
[behaviour/output/listings]: docs/behaviour/output/listings/README.md
[behaviour/output/streams]: docs/behaviour/output/streams.md
[behaviour/output/terminal]: docs/behaviour/output/terminal.md
[behaviour/parsing]: docs/behaviour/parsing/README.md
[behaviour/parsing/aliases]: docs/behaviour/parsing/aliases.md
[behaviour/parsing/completion]: docs/behaviour/parsing/completion.md
[behaviour/run-observer]: docs/behaviour/run-observer/README.md
[behaviour/schemas]: docs/behaviour/schemas/README.md
[behaviour/schemas/commands]: docs/behaviour/schemas/commands.md
[behaviour/usage-log]: docs/behaviour/usage-log/README.md
[behaviour/usage-log/report]: docs/behaviour/usage-log/report.md
[behaviour/usage-log/trace-context]: docs/behaviour/usage-log/trace-context.md
[project/dependencies]: docs/project/dependencies.md
[project/testing]: docs/project/testing.md
