# crumbkit affordances

This file lists what a tool author can use from crumbkit. It ships with
cargo crumbkit, and the "Audit a tool" job of its skill names one criterion
for each row or group of rows.

- **Replaces** says what the tool writes without the affordance.
- **Smell** says what shows the affordance is missing.
- **Search** holds patterns for `rg -n -t rust -e`, one per backticked span.
  `<bin>` is the tool's binary name. "Read" means no search finds the absence,
  so the auditor reads the named thing.
- **Since** is the first version with the affordance: `0.1` is 0.1.0 or 0.1.1,
  `0.2.0` is the release of 2026-11-09, and `0.3.0` is the release of
  2026-11-10.
- **Criterion** is the id in the skill's Criteria table.

Compare **Since** with the version in the repository's `Cargo.lock`.

## Names that changed at 0.2.0

A tool on 0.1 uses the old name for the same affordance.

| 0.1 | 0.2.0 |
| --- | --- |
| `crumbkit::Output`, the handle of one run | `crumbkit::Run` |
| `help::Guide` | `crumbkit::Tool`, composed of `help::Help`, `tool::Failures` and `tool::Output` |
| `Guide::output_budget` | `tool::Output::budget` |
| `Guide::documents` | `Help::file_formats` |
| `crumbkit::Duration` | `crumbkit::Elapsed` |
| `check_guide` | `check_help` |
| `Row::CHILD_INDENT`, `Row::SPACED`, `Row::CHILDREN_HELP` | `Row::CHILDREN`, an `Option<Children<Self>>` |
| `Row::child_lines`, `Row::long_child_lines` | `Children::lines`, `Children::long_lines` |
| `parse(command, &guide)` | `parse::<Cli>(&TOOL)` |
| `try_parse`, `try_parse_from` | `try_parse_to` with `Sink::standard` |

The word "guide" in a tool on 0.1 means what 0.2.0 calls the `Tool` record
and its help content: leaves, examples, goal headings and topics.

## Adoption and wiring

| Affordance | Replaces | Smell | Search | Since | Criterion |
| --- | --- | --- | --- | --- | --- |
| `parse`, `parse_from`, `parse_to` | `Cli::parse`, which exits the process | `main` calls clap's parse | `Cli::parse\(\)` `Parser::parse` | 0.1 | `finish-output` |
| `try_parse_to`, `Refused`, `Unparsed`, `Stop` | Matching clap's error by hand | Code reads `ErrorKind` or calls `Error::exit` | `ErrorKind::` `\.exit\(\)` `try_parse\(` | 0.1 | `refusal-seams` |
| `Run::new`, `Run::new_to` | Separate output paths for a clap-only tool | A clap-only tool prints its own JSON | `Run::new` | 0.1 | `finish-output` |
| `finish`, `finish_to` | `println!` of a result | A handler prints and returns unit | `println!` `print!` | 0.1 | `finish-output` |
| `Run::fail` | Printing a failure by hand on a path with no `finish` | `eprintln!("error` beside an exit | `eprintln!\("error` | 0.2.0 | `finish-output` |
| `Tool` record | Scattered help constants | `Tool::new()` passed with nothing declared | `Tool::new\(\)` | 0.1 as `Guide` | `tool-record` |
| `command_path` | Walking the argv by hand | Code indexes `args` to find the command | `args\(\)\.nth` `command_path` | 0.1 | `refusal-seams` |
| `help::quote` | Hand-written shell quoting | A format string wraps a value in quotes for a shell | `shell_escape` `shlex::` | 0.1 | `typed-recovery` |
| Cargo features `derive`, `miette`, `tracing`, `color`, `complete` | Extra glue code | Read the feature list against the code | Read `Cargo.toml` | 0.1 | per feature |

## Failures and recoveries

| Affordance | Replaces | Smell | Search | Since | Criterion |
| --- | --- | --- | --- | --- | --- |
| `Failure` trait | Error enums with only `Display` | A tool's error reaches the caller with no code | `impl.*Failure\s+for` | 0.1 | `declared-code` |
| `#[derive(Failure)]`, `CODES` | A hand-written `match` per method and a hand-kept code list | `fn code` beside `match self`, a written `CODES` | `fn code\(&self\)` `CODES:` | 0.1 | `derive-failure` |
| `Code::new` | Error-code strings | A string constant such as `"not_found"` is matched | `code:\s*&'static str` | 0.1 | `declared-code` |
| `Code::runtime`, `Spelling::COLONS.split` | Reusing a plugin's error text | Code passes a plugin's code through as text | `split\("::"\)` | 0.1 | `declared-code` |
| `Recovery::Run(Call::new(..))` | A sentence naming a command | A hint names a command of this tool | `Recovery::[Hh]int` `\bhint\s*[=(]` `"[^"]*\b<bin> [a-z-]+` | 0.1 | `typed-recovery` |
| `Call::arg`, `Call::value`, `Call::all` | Formatting values into a command sentence | `format!` builds a recovery's command | `format!\("[^"]*\b<bin> ` | 0.1 | `typed-recovery` |
| `Call::typed` | Re-joining the caller's argv | A recovery repeats `std::env::args` | `env::args` | 0.1 | `typed-recovery` |
| `Call::help` | A sentence saying to read help | A hint says "see --help" | `see .*--help` `run .* -h` | 0.1 | `typed-recovery` |
| `Recovery::Retry` | A sentence saying to try again | A hint says to try or run the command again | `(try\|run .*) again` | 0.1 | `typed-recovery` |
| `Recovery::Hint`, `Recovery::hint` | Nothing: this is the free-text form, for a repair that is no command of the tool | A hint that names a tool command misuses it | as `typed-recovery` | variant 0.1, `hint` 0.2.0 | `typed-recovery` |
| `Recovery::None`, `Recovery::none`, `#[failure(no_next_step)]` | A blank recovery | A repairable failure says there is no next step | `Recovery::None` `no_next_step` | variant 0.1, the others 0.2.0 | `declared-code` |
| `Failure::alternatives`, `#[failure(alternative = ..)]` | One sentence with two repairs | A hint joins two repairs with "or" | `hint.*\bor\b` | 0.1 | `alternatives` |
| `Failure::fields`, `Field`, `Field::shown`, `FieldValue`, `field(..)` | Values only in the message | A variant holds an id or path and has no field | `#\[failure\(` | 0.1 | `failure-fields` |
| `Failure::url`, `#[failure(url = ..)]` | A link inside the message | A failure message holds `https://` | `"[^"]*https://` | 0.1 | `failure-fields` |
| `Error`, `?` across error types | A wrapper enum of every error type | A hand-written `From` chain exists only to reach `finish` | `impl From<.*Error> for` | 0.1 | `declared-code` |
| `Error::unclassified` | Dropping an untyped error | Use is correct only for errors with no class yet | `Error::unclassified` | 0.1 | `declared-code` |
| `miette` feature, `Failure::diagnostic`, source excerpts | Printing a source excerpt by hand | Code draws carets under a line | `miette::` `"\^+"` | 0.1 | `failure-fields` |
| `Exit::new`, `Exit::range`, `Exit::failures`, `Failures::exits` | Numeric exits in handlers | An exit above 2 that no `Exit` declares | `ExitCode::from\(` `process::exit` | 0.1, in `tool` from 0.2.0 | `exit-codes` |
| `Leaf::passes_on_exit` | An undeclared pass-through exit | A leaf returns a child's code with no declaration | `\.code\(\)` | 0.1 | `exit-codes` |
| `Leaf::refusal_alternatives`, `Failures::refusal_alternatives` | Printing a correction after clap's refusal | Code suggests a corrected line by hand | `ErrorKind::` | 0.1 | `refusal-seams` |
| `Leaf::refusal_hint`, `Failures::refusal_hint`, `RefusalContext` | Rewording clap's error | Code rewords clap's text | `ErrorKind::` | 0.1 | `refusal-seams` |
| `RenderCall::render` | Formatting a command into a row or note | A handler builds command text by hand | `RenderCall` `"[^"]*\b<bin> [a-z-]+` | 0.1 | `row-command` |

`Recovery::Run` renders against the clap tree. Rendering checks the path, the
argument ids, hidden items and refused values, and a call that fails prints as
`Recovery::None`. Rendering reads only the arguments that the call names, so a
call that leaves out a required argument renders and clap refuses it when it
runs. `check_recoveries` finds that case when a test provokes the failure.
`Call::typed` and `Call::help` render with no lookup.

## Output

| Affordance | Replaces | Smell | Search | Since | Criterion |
| --- | --- | --- | --- | --- | --- |
| `Row`, `COLUMNS`, `LONG_COLUMNS` | Hand-padded tables | Tab-joined or padded fields | `join\("\\t"\)` `format!\("\{:[<>]` | 0.1 | `row-shape` |
| `Document::new`, `Document::when_empty` | Ad hoc JSON and empty messages | `serde_json::to_string` beside `println!`, `"no .* found"` | `serde_json::to_string` | 0.1 | `finish-output` |
| `Envelope::Without` | A second output path for a legacy JSON shape | A leaf serialises a legacy shape by hand | `Envelope::Without` | 0.1 | `finish-output` |
| `ViewFlags` | Own `--json`, `--fields`, `--long` | A command declares these flags | `json:\s*bool` `fields:\s*` `long:\s*bool` | 0.1 | `view-flags` |
| `Run::is_json` | A second parsed copy of `--json` | Code reads argv for `--json` | `"--json"` | 0.1 | `view-flags` |
| `JsonLayout` | Pretty-printing by hand | `to_string_pretty` in a handler | `to_string_pretty` | 0.1 | `view-flags` |
| `Page<N, MOST>`, `first_of`, `listing`, `uncounted` | Own `--limit` and `--all` | A limit field and a `take` | `limit:\s*(Option<)?usize` `\.take\(` | 0.1 | `page-listing` |
| `Listing::new`, `from_all`, `uncounted`, `capped` | Hand-written "showing N of M" | A footer string in a handler | `Showing ` | 0.1 | `page-listing` |
| `Listing::described`, `singular`, `summary`, `note`, `long_note` | Lines printed around a table | `eprintln!` next to rows | `eprintln!` | 0.1 | `page-listing` |
| `Listing::step`, `Step::call` | A sentence naming the next command | A note or row names a tool command | `\.note\(` `reason:` | 0.1, JSON `next` 0.1.1 | `row-command` |
| `Step::run`, `Program` | A sentence naming another program's command | A note or row says to run another program | `\.note\(` `reason:` | 0.1 | `program-step` |
| `Row::PROBLEM`, `Problem` | Setting exit 1 by hand when a row is bad | A handler counts bad rows and exits | `ExitCode::FAILURE` | 0.1, `Problem` 0.5 | `problem-rows` |
| `Document::only_problems` | A `--problems` filter | A flag that hides healthy rows | `only_problems` `problems:\s*bool` | 0.1 | `problem-rows` |
| `Row::in_default_text` | Filtering rows before `finish`, which also drops them from JSON | A handler filters rows only for text | `is_json\(\)` | 0.1 | `row-shape` |
| `Row::UNCUT_COLUMNS` | Widening a table so an id survives | Read: rows with a copyable id | Read `Row` impls | 0.1 | `row-shape` |
| `Row::cell_style`, `Style` | Colour crates and escape codes | ANSI codes in a cell | `\\x1b` `\\u\{1b\}` `owo_colors` `colored::` | 0.1 | `row-shape` |
| `Row::CHILDREN`, `Children`, `Children::lines`, `Children::long_lines` | Indented lines built by hand | A cell holds `\n` or leading spaces | `format!\(" +` | 0.1, `Children` 0.2.0 | `row-shape` |
| `Row::LINE`, `Line` | A custom line in `println!` | One format per row, no table | `println!` | 0.1, `Line` 0.5 | `row-shape` |
| `Row::WHOLE` | A second serialiser for JSON | A tool writes full JSON by hand | `Row::WHOLE` `const WHOLE` | 0.1 | `row-shape` |
| `Batch`, `Batch::note` | A loop that stops at the first failed id | `show` takes one id, or uses `?` in a per-id loop | `ids?:\s*Vec<` | 0.1 | `batch-lookup` |
| `fetch_each` | A hand-written thread pool of lookups | `thread::scope` around per-id calls | `thread::scope` | 0.1 | `batch-lookup` |
| `Run::room`, `Room::has_room` | Fetching rows that will be cut | Read: a batch with a slow fetch | `Run::room` `\.room\(` | 0.1 | `batch-lookup` |
| `Prose`, `Prose::note` | `println!` of a sentence | A handler prints a sentence result | `println!` | 0.1 | `bytes-prose` |
| `Bytes` | Printing file contents raw | `io::copy` or `write_all` on stdout | `write_all\(` `io::copy` | 0.1 | `bytes-prose` |
| `Headed`, `Document::headed` | `==> name <==` headers by hand | A format string with `==>` | `==>` | 0.1 | `bytes-prose` |
| `Stream`, `stream`, `stream_to`, `Streaming`, `Leaf::stream` | A print-and-flush loop | `flush` in a loop, a `--watch` flag | `\.flush\(\)` `watch:\s*bool` `follow:\s*bool` | 0.1 | `stream-output` |
| `Age`, `Age::compact` | "3h ago" strings | A row field holds a formatted time | `_ago` `chrono_humanize` `timeago` | 0.1 | `value-types` |
| `Elapsed` | Hand-formatted durations | A row field holds `"{}s"` text | `format_duration` `as_secs_f` | 0.2.0 as `Elapsed`, 0.1 as `Duration` | `value-types` |
| `Shown`, `shown!` | Separate text and JSON fields for one value | A row has `size` and `size_text` | `_text:\s*String` `_display:\s*String` | 0.1 | `value-types` |
| `tool::Output::budget`, `<TOOL>_OUTPUT_BUDGET` | Truncating by byte count | A handler cuts output at a constant | `truncate\(` `MAX_[A-Z_]*BYTES` | 0.1 as `Guide::output_budget` | `output-budget` |
| `Run::budget`, `Streaming::budget` | Fetching more rows than print | Read: a handler with an unbounded fetch | `\.budget\(\)` | 0.1 | `output-budget` |
| `Document::always_print_first` | Raising the budget for one big answer | A raised budget whose reason names one file | `\.budget\(` | 0.1 | `output-budget` |
| `Run::progress_allowed` | Own check before a spinner | A progress bar behind `is_terminal` | `is_terminal` `indicatif` | 0.1 | `terminal` |
| `Sink`, `Sink::fitted`, `Sink::styled`, `Sink::with_input` | Capturing stdout in tests through a subprocess | A test launches the binary only to read a table | `Sink::` | 0.1 | `terminal` |
| `tool::Output::without_terminal_detection` | Own environment switch for plain output | A variable that turns off colour or prompts | `NO_COLOR` `isatty` | 0.1 | `terminal` |
| `Leaf::prints_no_result` | An opt-out for a server command | `help/document` opt-out on a server | `prints_no_result` | 0.1 | `tool-record` |

## Effects and prompts

| Affordance | Replaces | Smell | Search | Since | Criterion |
| --- | --- | --- | --- | --- | --- |
| `Effects::read_only`, `additive`, `destructive`, `idempotent`, `open_world`, `Leaf::effects` | A description of what a command changes | A leaf has no `.effects(` | `Leaf::new` `\.effects\(` | 0.1 | `effects-declared` |
| `Effects::all_or_none`, `per_item` | A sentence about partial writes | A multi-item write declares neither | `\.effects\(` | 0.1 | `effects-declared` |
| `DryRun`, `Run::is_dry_run` | Own `--dry-run` | A bool field `dry_run` | `dry_run:\s*bool` | 0.1 | `dry-run` |
| `Yes`, `Run::confirm` | Own `--yes` and a stdin prompt | A bool field `yes`, `read_line` on stdin | `yes:\s*bool` `read_line` `Confirm::new` | 0.1 | `confirm-yes` |
| `NoInput`, `Run::no_input` | Own `--no-input` | A bool field `no_input` | `no_input:\s*bool` | 0.1, `Run::no_input` 0.2.0 | `confirm-yes` |
| `Expect`, `Expect::check`, `Expect::conflict`, `Effects::replacing` | A hand-written compare-and-swap | A write compares a stored version | `expect:\s*Option` `if_match` `expected_version` | 0.1 | `expect-guard` |
| `Tool::picker`, `pick::Menu`, `Picker`, `Leaf::choices`, `Choices`, `pick::from_rows` | Prompting for a missing argument | A prompt library or a loop over choices | `dialoguer` `inquire::` `Select::new` | 0.1 | `picker` |

## Help and reference

| Affordance | Replaces | Smell | Search | Since | Criterion |
| --- | --- | --- | --- | --- | --- |
| `Leaf::new`, `Leaf::examples`, `Example`, `Example::needs` | Examples in `after_help` text | An `after_help` holding command lines | `after_help` `after_long_help` | 0.1 | `tool-record` |
| Doc comments on row fields as column descriptions | A column table in help text | `after_help` listing columns | `after_long_help` | 0.1 | `tool-record` |
| `Group`, `Goal`, `Group::compact` | Hand-arranged help headings | `help_heading` or `help_template` on a group | `help_template` `next_help_heading` | 0.1 | `tool-record` |
| `Topic`, `help exit-codes` | Long concept text on a command | `after_help` that explains a concept, a `docs` command | `after_help` `include_str!` | 0.1 | `tool-record` |
| `Help::root_verbs` | An undeclared leaf at the root | `Rule::RootLeaf` opt-outs for several verbs | `Rule::RootLeaf` | 0.1 | `tool-record` |
| `Help::root` | Help that names the wrong root under another program | A tool run as a subcommand of another | `Help::root` | 0.1 | `tool-record` |
| `RuntimeLeaf`, `Tool::with_runtime_leaves` | Plugin commands with no examples | Commands added with `Command::subcommand` at run time | `\.subcommand\(` | 0.1 | `tool-record` |
| `help::extend` | Goal headings by hand in a clap-only tool | A clap-only tool with hand-built headings | `help::extend` | 0.1 | `tool-record` |
| `commands::CommandsList`, `commands list` | A hand-written command dump | A `--list-commands` flag or nothing | `CommandsList` | 0.1 | `agent-commands` |
| `schemas::SchemasList`, `SchemasShow`, `schemas list`, `schemas show` | Schema files that agents cannot reach | Checked-in schemas and no command | `SchemasList` `schema_for!` | 0.1 | `agent-commands` |
| `Help::file_formats`, `FileFormat` | Config format described in prose | A TOML config with no served schema | `file_formats` | 0.2.0, 0.1 as `Guide::documents` | `agent-commands` |
| `crumbkit_aliases::expand`, `file`, `list`, `names`, `AliasesList` | Own alias file | Code reads a user's alias map before parsing | `aliases\.toml` `crumbkit_aliases::` | 0.1 | `aliases-completion` |
| Hidden clap aliases, under `check_names` | `visible_alias` | Help shows two names | `visible_alias` | 0.1 | `aliases-completion` |
| `complete` feature, `complete`, `complete_with`, `complete_with_words` | `clap_complete::generate` | A static completion subcommand | `clap_complete` | 0.1 | `aliases-completion` |
| `cargo crumbkit docs`, `check_command_reference`, `crumbkit_markdown::render` | A hand-kept command reference | A CLI document edited by hand, `clap_markdown` | `clap_markdown` `check_command_reference` | 0.1 | `command-reference` |
| `cargo crumbkit weigh`, `--usage` | Guessing what help costs | Read: no recorded weight for a tool with big help | Read CI config | 0.1 | `convention-suite` |

## Convention tests

| Affordance | Replaces | Smell | Search | Since | Criterion |
| --- | --- | --- | --- | --- | --- |
| `Suite`, `conventions!`, `Check`, `Suite::skip` | A hand-kept list of `check_*` tests | `check_` calls and no `conventions!` | `conventions!` `crumbkit_test::check_` | 0.2.0 | `convention-suite` |
| `check_names` | Review of second names | No test calls it | `check_names` | 0.1 | `convention-suite` |
| `check_grammar` | Review of command shape | No test calls it | `check_grammar` | 0.1 | `convention-suite` |
| `check_effects` | Review of `--yes` and `--dry-run` | No test calls it | `check_effects` | 0.1 | `convention-suite` |
| `check_listings` | Review of `list` and `show` | No test calls it | `check_listings` | 0.1 | `convention-suite` |
| `check_filters` | Review of multi-value options | No test calls it | `check_filters` | 0.1 | `convention-suite` |
| `check_help` | Review of the `Tool` record | No test calls it | `check_help` `check_guide` | 0.1 as `check_guide` | `convention-suite` |
| `check_exit_codes` | Review of exit numbers | No test calls it | `check_exit_codes` | 0.1 | `convention-suite` |
| `check_help_sizes`, `Budgets`, `Budget` | No limit on help size | No test calls it | `check_help_sizes` | 0.1 | `convention-suite` |
| `check_path_weights`, `PathBudgets` | No limit on a help path | No test calls it | `check_path_weights` | 0.1 | `convention-suite` |
| `check_help_goldens`, `Pages` | Help snapshots by hand | `insta` over help | `check_help_goldens` `insta::` | 0.1 | `twins-examples` |
| `check_schema_goldens` | Schemas checked in by hand | No test calls it | `check_schema_goldens` | 0.1 | `convention-suite` |
| `check_document_ids`, `check_mounts` | Review of document names, and a test that runs each query of `crumbkit::queries` against the binary | No test calls it, or the root arguments lack `NoInput` or `ViewFlags` | `check_document_ids` `check_mounts` `NoInput` | 0.1, the query rule after 0.4.0 | `convention-suite` |
| `check_opt_outs`, `Allow`, `Rule` | Lint suppressions with no reason | No test calls it | `check_opt_outs` | 0.1 | `convention-suite` |
| `check_examples`, `Examples`, `example_golden_name`, `FailureReader` | `assert_cmd` tests of each example | Examples that no test runs | `check_examples` `assert_cmd` | 0.1 | `twins-examples` |
| `check_output_budget` | No test of the cut result | No test calls it | `check_output_budget` | 0.1 | `convention-suite` |
| `check_recoveries`, `Recoveries`, `Failing`, `Unprovoked`, `RuntimeFailing`, `RuntimePlace` | Tests of one recovery at a time | A declared code with no start | `check_recoveries` `Failing::new` `Unprovoked::` | 0.1 | `provoked-call` |
| `check_codes` | A test that scans source for codes | A code test written by hand, or `check_codes` skipped | `check_codes` | 0.1 | `convention-suite` |
| `check_twin`, `check_twins`, `Twin`, `Twins` | Tests that assert only a failure | A refusal test with no accepted line | `check_twins?\(` | 0.1 | `twins-examples` |

## Usage log and tracing

| Affordance | Replaces | Smell | Search | Since | Criterion |
| --- | --- | --- | --- | --- | --- |
| `crumbkit_usage::start`, `Log`, `log_path` | Hand-written telemetry | A tool writes its own run history | `crumbkit_usage::start` `history\.jsonl` | 0.1 | `usage-log` |
| `start_under` | Two owners of each signal | `start` beside a signal handler that ends the tool | `start_under` | 0.2.0 | `signals` |
| `Extension`, `extend`, `Facts`, `Key`, `Label` | Own timing or request counters | Counters written to a side file | `crumbkit_usage::extend` | 0.1 | `usage-log` |
| `SelfUsage`, `findings`, `SelfFigures`, `figures` | A hand-written usage report | The log is kept and no `self usage` is mounted | `SelfUsage` `SelfFigures` | 0.1 | `usage-log` |
| `events::observe`, `Observer` | Parsing the tool's own output for metrics | A second code path around each exit | `events::observe` | 0.1 | `usage-log` |
| `events::tolerated` | Losing a recovered failure | A retry or wait that swallows a code | `events::tolerated` | 0.1 | `usage-log` |
| `events::ended` | Unreported exit paths | A `try_parse_to` tool prints its own refusal | `events::ended` | 0.1 | `usage-log` |
| `trace::child_env`, `TRACEPARENT` | Children outside the run's trace | The log is kept and children get no trace | `child_env` `TRACEPARENT` | 0.1 | `usage-log` |
| `tracing` feature | Own spans around `parse` and `finish` | `#[instrument]` on `main` | `tracing::` | 0.1 | `usage-log` |

## Processes and signals

| Affordance | Replaces | Smell | Search | Since | Criterion |
| --- | --- | --- | --- | --- | --- |
| `crumbkit_child::hand_over` | Inheriting the terminal by hand | `.status()` with inherited streams as the whole response | `\.status\(\)` `\.exec\(\)` `Stdio::inherit` | 0.1 | `hand-over` |
| `crumbkit_process::run`, `RunOptions`, `ChildFinished` | `.output()` with no limit or deadline | Child output read whole | `\.output\(\)` `wait_with_output` | 0.2.0 | `process-run` |
| `Capture::Whole`, `Tail`, `Discard`, `OverLimit` | Reading into an unbounded buffer | `read_to_end` on a child pipe | `read_to_end` | 0.2.0 | `process-run` |
| `Capture::Lines`, `line_queue`, `Lines` | A reader thread and a channel | `BufReader::new(child.stdout` | `BufReader::new\(.*stdout` | 0.2.0 | `process-run` |
| `RunOptions::deadline`, `Cancel`, `Outcome`, `StopReason` | `try_wait` polling and a timer | A loop around `try_wait` | `try_wait` | 0.2.0 | `process-run` |
| `Escalation`, `Signal` | `kill` then sleep then `kill -9` | A hand-written stop sequence | `\.kill\(\)` `SIGKILL` | 0.2.0 | `process-run` |
| `Scope`, process group or job object | `setsid` or `process_group` by hand | `process_group(0)`, `pre_exec` | `process_group\(` `pre_exec` | 0.2.0 | `process-run` |
| `Status::shell_code`, `ChildFinished::status` | `128 + signal` by hand | Arithmetic on a signal number | `128 \+` `\+ 128` | 0.2.0 | `signals` |
| `forward` feature, `forward::install`, `forward::clean_up`, `forward::handed_over` | Own handler that signals children | `signal_hook` or `ctrlc` around children | `ctrlc` `signal_hook` `SIGINT` | 0.2.0 | `signals` |
| `RunOptions::around_spawn`, `Spawn`, `Spawned` | A lock held around `spawn` by hand | A lock guards a spawn | `\.spawn\(\)` | 0.3.0 | `process-run` |
| `RunOptions::stdout_copy`, `stderr_copy` | A tee thread | Output shown and captured by hand | `\.spawn\(\)` | 0.3.0 | `process-run` |
| `Capture::Inherit` | Leaving the crate for one interactive step | `Stdio::inherit` in a child that returns | `Stdio::inherit` | 0.3.0 | `process-run` |
| `RunOptions::report_survivors`, `ChildFinished::survivors` | Scanning for left-over processes | `pgrep`, `ps` after a run | `pgrep` `"ps"` | 0.3.0 | `process-run` |
| `Input::File` | A pipe fed by hand | `Stdio::from` a file | `Stdio::from\(` | 0.3.0 | `process-run` |
| `Input::Bytes`, cleared on drop by `Zeroizing` | A secret left in a `Vec` | A secret written to a child's stdin | `write_all\(` `Zeroizing` | `Input::Bytes` 0.2.0, cleared on drop 0.3.0 | `process-run` |
| `LongLine`, `Capture::Lines` `end` | Splitting on NUL by hand | `split\(b'\0'` on child output | `split\(.*\\0` | 0.3.0 | `process-run` |

## Known gaps

These are places where crumbkit offers nothing. They are candidates for
`crumbkit-gap`, and each needs a repeated pattern before the audit files it.

- A row has no typed command. `Row` and its fields take text, so a command in
  a row's reason is a string. `Listing::step` works per listing, and
  `RenderCall::render` returns a string that the handler must place itself.
- Rendering a `Call` does not require the required arguments of its command.
- No check parses a `Step::call` line. `check_recoveries`
  parses only recoveries. `check_examples` pins the printed line in a golden
  file.
- `check_recoveries` records a `Recovery::Hint` and moves on, so nothing parses
  a command inside a hint and nothing flags a hint that names one.
- A failure has no typed recovery that names another program. `Step::run` and
  `Program` exist only on listings, and `Recovery` holds `Run`, `Retry`, `Hint`
  and `None`.

## Items left out

An author does not choose these directly: `Rows`, `Shape`, `AsFailure`,
`CallError`, `CallTarget`, `CallArg`, `RecoveryStep`, `ToolName`, `Printed`,
`Silent`, `ViewFlag`, `Items`, `CommandRow`, `SchemaRow`, `SchemaShowRow`,
`Served`, `schemas::documents`, `schemas::document_schema`, the
`crumbkit::codes` constants, and the `#[doc(hidden)]` items `output_weight`,
`path_totals` and `UnreadableGolden`.
