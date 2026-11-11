---
name: cargo-crumbkit
description: Measure, document and check a Rust command-line tool built with crumbkit, and audit a tool for places that should use something crumbkit offers and do not, such as typed recoveries, the `Failure` derive, output shapes, effects, convention checks and process handling. Use for cargo crumbkit's commands and for a crumbkit audit.
---

# cargo crumbkit

`cargo crumbkit` is a cargo subcommand for tools built with crumbkit. Each
command that takes `--bin <NAME>` builds that binary of the current
workspace and runs it, as an agent would.

## Commands

- `cargo crumbkit weigh --bin <NAME>` measures the bytes that an agent reads
  on each path through the tool's help, from the root help page to each
  command's examples. Add `--usage` to add the runs, help pages and output
  of each command from the tool's own usage log. Use it to find the help
  that costs most to read.
- `cargo crumbkit docs --bin <NAME>` writes a Markdown reference of every
  command of the tool to `docs/commands.md` in the binary's package, or to
  the file that `--out <FILE>` names. Use it after changing the tool's
  commands, and let the tool's convention test check that the file is
  current.
- `cargo crumbkit tool-skill check --bin <NAME>` checks that every command
  line in the tool's `skill/SKILL.md`, or in the file that `--skill <FILE>`
  names, names real commands and flags of the tool. It exits with 1 when a
  line fails, so CI can run it.
- `cargo crumbkit tool-skill draft --bin <NAME>` writes the generated
  sections of the tool's `SKILL.md` from its commands, and with
  `--usage <DIR>` from its exported usage. `--check` writes nothing and
  exits with 1 when a draft would change a section.
- `cargo crumbkit skills install` installs this skill into each agent skills
  directory that exists. `cargo crumbkit skills check` reports a copy that
  is missing or differs from this cargo crumbkit, and
  `cargo crumbkit skills print` prints this file.
- `cargo crumbkit commands list` lists every command with its arguments,
  examples and columns. Add `--json --full` for the whole tree.

`cargo crumbkit help` lists the commands with their examples, and
`cargo crumbkit help exit-codes` says what each exit code means.

## Audit a tool

This job audits a Rust command-line tool for places that should use
something crumbkit offers and do not. It writes a JSON report.

### Reads

- `AFFORDANCES.md` beside this skill: every affordance with what it
  replaces, its smell, its search and the version that added it.
- Each package's `Cargo.toml`, with the crumbkit features it turns on. The
  `Failure` derive needs the `derive` feature.
- The crumbkit version in `Cargo.lock`. A git source shows `0.0.0`, so read
  the commit it pins: run
  `git -C <crumbkit checkout> merge-base --is-ancestor <rev> v0.2.0`, or read
  the checkout under `~/.cargo/git/checkouts` and test for names such as
  `Tool` (0.2.0) against `Guide` (0.1).
- The clap tree. Its binary names fill `<bin>` in the searches.
- Each `Tool` record, `Failure` impl and its `recovery` function, `Row` type,
  handler and `main`.
- The convention tests.
- Comments that name a missing crumbkit feature. They are evidence for
  `crumbkit-gap`.

A repository with no clap command tree gets the outcome `skipped`.

### Scope

A full audit judges the whole repository. A catch-up audit judges only the
files that changed since a starting commit, which
`git diff --name-only <since>..HEAD` lists, and the report names that commit
under `since`.

The criteria apply to the package that holds the clap tree, and to the
library crates it calls for `process-run`, `signals` and `crumbkit-gap`.
Search with `-g '!tests/**' -g '!**/tests.rs' -g '!xtask/**' -g '!build.rs'`,
and reject hits inside a `#[cfg(test)]` module. Tests are read only for
`convention-suite`, `twins-examples`, `terminal` and `tool-record`.

A server or daemon keeps its own graceful stop flag, HTTP request handling
and long-running supervision. `signals`, `dry-run`, `expect-guard` and
`agent-commands` apply to its commands, not to those.

### Method

Run each backticked pattern in the Search column as one `rg -n -t rust -e`
pattern, and replace `<bin>` first. `\|` in a table cell stands for `|`. A
pattern marked `-U` runs multiline. A hit
is a candidate. Read each candidate and judge it against the Finding column.
Where the Search column says "Read", read the named thing whole, because no
search finds an absence. The step is complete when every pattern has run,
every hit is judged and every "Read" is done.

A hint is judged by its text. Read every `Failure::recovery` function and
every `#[failure(hint = ..)]` whole, since a match arm's string carries no
`hint` token. `<bin>` followed by a word is a candidate only inside a hint,
recovery, `help(..)` or row; elsewhere it is usually the tool's name as the
subject of a sentence.

Compare each affordance's **Since** with the pinned version. An affordance
newer than that version is no finding, and the report lists it under
`newer_affordances` as available after an upgrade. Two crumbkit versions in
one `Cargo.lock` are a finding on `convention-suite`, because their codes,
events and observers differ.

Count one finding per symbol and criterion, with every place in its
`places`. Equivalent hints in one file make one finding.

### Severity

The default severity is `low`. It is `medium` for `typed-recovery`,
`provoked-call`, `row-command`, `effects-declared`, `confirm-yes` and
`expect-guard`, because a caller acts on what they print or declare.

It is `high` where a hand-written mechanism runs beside crumbkit's and can
disagree with it:

- a child-process runner that `crumbkit_process::run` covers, which leaves
  descendants running or reads output with no limit or deadline;
- signal handlers in two owners, such as one beside `crumbkit_usage::start`;
- a failure, exit-code or output stack of the tool's own beside `Failure`,
  `Exit` and `finish`;
- a usage log of the tool's own.

### Criteria

| Id | Affordance | A finding is a place where this holds | Search |
| --- | --- | --- | --- |
| `typed-recovery` | `Recovery::Run` with `Call`, `Recovery::Retry` | A hint, `no_next_step` reason or failure message names a command of this tool in prose, or says to run the same command again. | `-U` `hint\s*[=(]\s*"[^"]*\b<bin> [a-z-]+` `help\("[^"]*\b<bin> [a-z-]+` `(try\|run .*) again`; Read each `recovery` function |
| `provoked-call` | `check_recoveries` | A `Call` omits a required argument, or a declared code is neither a `Failing` start nor `Unprovoked`. | `Recovery::Run` `Call::new` `Failing::new` |
| `row-command` | `Listing::step`, `Step::call`, `RenderCall::render` | A row field, note, summary or `Prose` text names a command of this tool. | `\.note\(` `Listing::step`; Read row fields that are `String` |
| `program-step` | `Step::run`, `Program` | A listing's note or row names a command of another program. | `\.note\(` |
| `alternatives` | `Failure::alternatives` | One hint offers two different repairs. An "or" between two values of one repair is one repair. | `-U` `"[^"]*(, or \| or )[^"]*"` inside `recovery` and hints |
| `failure-fields` | `Failure::fields` | A failure variant holds an id, path, name, status or number that only its message shows. | Read each `Failure` type's variants |
| `derive-failure` | `#[derive(Failure)]` | An `impl Failure` is a `match` per method, or `CODES` is written by hand. | `impl.*Failure\s+for` `CODES:` |
| `declared-code` | `Code::new`, `Spelling::COLONS` | An error reaches the caller with no declared code, a code is spelled by hand, or a repairable failure offers no step. | `Error::unclassified` `anyhow!` `Recovery::None` `component\(\)` |
| `exit-codes` | `Exit` | A handler exits above 2 undeclared, or calls `process::exit`. | `ExitCode::from\(` `process::exit` `-> u8` `EXIT_` `\b130\b` |
| `refusal-seams` | `refusal_hint`, `refusal_alternatives`, `try_parse_to` | Code rewords or matches clap's refusal. | `clap::error::ErrorKind` `ErrorKind::(Unknown\|Invalid\|Missing)` `\.exit\(\)` |
| `finish-output` | `parse`, `finish`, `Run::fail` | A handler prints its result itself. | `println!` `print!` `Cli::parse\(\)` |
| `view-flags` | `ViewFlags` | A command declares its own view flag. | `json:\s*bool` `long:\s*bool` `full:\s*bool` |
| `page-listing` | `Page`, `Listing` | A command pages or writes a footer itself. | `limit:\s*(Option<)?usize` `Showing ` |
| `batch-lookup` | `Batch`, `fetch_each` | A many-id command stops at its first failure or pools threads itself. | `ids?:\s*Vec<` `thread::scope` |
| `value-types` | `Age`, `Elapsed`, `Shown` | A row field holds preformatted time, duration or units. | `format_duration` `_ago` `_text:\s*String` `to_rfc3339` |
| `row-shape` | `Row` constants and methods | A handler pads, indents, colours or filters rows for text. | `format!\("\{:[<>]` `\\x1b` `owo_colors` |
| `problem-rows` | `Row::PROBLEM` | A handler sets exit 1 because of a row. | `ExitCode::FAILURE` |
| `stream-output` | `stream` | A loop prints and flushes rows. | `\.flush\(\)` `watch:\s*bool` |
| `bytes-prose` | `Bytes`, `Prose`, `Headed` | File bytes or a sentence print outside `finish`. | `write_all\(` `==>` `eprintln!` |
| `output-budget` | `tool::Output::budget` | A handler truncates output by size. | `truncate\(` `MAX_[A-Z_]*BYTES` |
| `terminal` | `Run::progress_allowed`, `Sink` | Printing branches on a terminal check, or a test launches the binary only to read its output. | `is_terminal` `NO_COLOR` `CARGO_BIN_EXE` |
| `effects-declared` | `Leaf::effects` | A visible leaf declares no effects. | Read: count `Leaf::new` against the clap leaves |
| `dry-run` | `DryRun` | A command has its own `--dry-run`. | `dry_run:\s*bool` |
| `confirm-yes` | `Yes`, `Run::confirm`, `NoInput` | A command has its own `--yes` or prompt, or decides who can answer from a terminal check alone. | `yes:\s*bool` `read_line` `stdin\(\)\.is_terminal` |
| `expect-guard` | `Expect` | A write compares versions by hand. | `expect:\s*Option` `if_match` |
| `picker` | `Tool::picker` | Code prompts for a missing argument. A picker that is the command's product is rejected. | `dialoguer` `inquire::` |
| `tool-record` | `Tool`, `Leaf`, `Group`, `Topic` | Examples or concepts sit in clap's help text or a long doc comment, or a `Tool` declares no leaves. | `after_help` `after_long_help` `Tool::new\(\)` |
| `agent-commands` | `commands list`, `schemas`, `file_formats` | A tool on crumbkit mounts neither, keeps its own description of the clap tree, or reads a config file with no served schema. | Read the clap tree for `CommandsList` and `SchemasList` |
| `aliases-completion` | `crumbkit_aliases`, `complete` | Help shows a second name, aliases are expanded by hand, or completion is generated by hand. | `visible_alias` `clap_complete` `\[alias\]` |
| `command-reference` | `cargo crumbkit docs` | A command reference is kept or rendered by hand. | `clap_markdown` `command.reference`; Read `docs/` |
| `convention-suite` | `Suite`, `conventions!`, `check_codes` | A package with a `Tool` has no suite, calls checks one by one, or skips one with a reason that only defers work. | `conventions!` `crumbkit_test::check_` `Suite::skip` |
| `twins-examples` | `check_twins`, `check_examples` | A refusal test has no accepted twin, or help is snapshotted by hand. | `check_twins?\(` `Some\(2\)` `insta::` |
| `process-run` | `crumbkit_process::run`, `Cancel` | A child's output is read with no byte limit or deadline, or a bounded run is written by hand. | `Command::new` `wait_timeout` `try_wait` `AtomicBool` |
| `hand-over` | `crumbkit_child::hand_over` | A child on the terminal is the whole response. | `Command::new[^;]*\.status\(\)` `Stdio::inherit` |
| `signals` | `forward`, `start_under` | Code owns a signal handler, two owners handle one signal, or code adds 128 to a signal. | `ctrlc` `signal_hook` `128 \+` `\.signal\(\)` |
| `usage-log` | `crumbkit_usage` | A tool writes its own run history, or keeps the log with no `self usage`. | `crumbkit_usage::start` `SelfUsage` `history` |
| `crumbkit-gap` | None | A hand-written mechanism repeats across two or more tools or crates, and no row of `AFFORDANCES.md` covers it. | "Known gaps" in `AFFORDANCES.md`; comments naming a missing feature |

Rendering a `Call` checks its path, argument ids, hidden items and values. It
reads only the arguments the call names, so `provoked-call` reads each
command's required arguments. `Call::typed` renders with no lookup. Only
`check_recoveries` catches a missing argument, and only when a test provokes
that failure. Until a tool uses `Recovery::Run`, `provoked-call` is not
applicable.

A `crumbkit-gap` is a proposal for crumbkit itself. It goes in the report's
`proposals`, never in `findings`, and it never makes the outcome `fail`. A
pattern seen once stays out. A finding that needs a feature crumbkit lacks
says so in its text and goes in `proposals`, not in `findings`.

### False positives

- A part that a tool has not adopted is a finding only where the smell is
  present, because crumbkit is adopted one part at a time. `convention-suite`
  and `effects-declared` are the exceptions in a package that has a `Tool`.
- A hint that names no command of this tool is rejected, such as setting a
  variable, editing a file or running another program. Naming another
  program's command is a known gap.
- A departure with an `Allow` or `Suite::skip` reason, or a comment at the
  spot that states why, is rejected. A reason that only defers the work is a
  finding.
- `.output()` on a child whose output is fixed and small is rejected when a
  comment at the spot says so.
- A deliberate departure that protects a secret, such as withholding typed
  values from a refusal, is rejected when a comment or document states it.
- A name's semantic precision alone is rejected. A name that is easy to reach
  for stays even when it is less exact, and help text carries the exact
  meaning. The exception is a name that hides a consequence the reader would
  not guess, such as `--force` on a command that deletes something only an
  untrusted file named.

### Report

Write the report as one JSON document that `audit-report-v1.json` beside
this skill describes, with the id `cargo-crumbkit/audit-report/v1`. It
holds:

- `repository`, `commit` and `crumbkit_version`: what was audited, at which
  commit, against which crumbkit version or pinned commit.
- `since`: the starting commit of a catch-up audit, or `null` for a full
  audit.
- `outcome`: `skipped` when no criterion could be judged, `fail` when the
  report holds a finding, and `pass` otherwise.
- `files_read`: the number of files read.
- `findings`: one entry per symbol and criterion, with the criterion id,
  the severity, the file, the symbol, every `file:line` under `places`, and
  a text that says what is wrong and the proposed change. A finding about
  something absent cites the line where it belongs.
- `proposals`: each `crumbkit-gap`, with its text and the places in each
  tool or crate.
- `newer_affordances`: each affordance newer than the pinned version, with
  its **Since**.
- `not_judged`: each criterion that could not be judged, with the reason,
  such as Reads that are absent.
- `unverified`: each finding with no `file:line` to cite, with its
  criterion and text. It counts in no outcome.
- `note`: a one-line verdict.

Print the report, or write it to the file the request names. When your
instructions name another place for findings, such as an issue tracker or
a task list, write the findings and proposals there with these fields.

### History

An adopter's hint named `<bin> books remove --yes`, which clap refused
because `remove` needs a book id. A hint is free text, so no check parsed
it. crumbkit's ADR-0001 and ADR-0053 give the reasons for recoveries and
their chains.

A trial on ten repositories rated nothing `high`, while an independent reader
rated seven hand-written runners, signal owners, failure stacks and usage logs
`high`. The Severity section now names those.
