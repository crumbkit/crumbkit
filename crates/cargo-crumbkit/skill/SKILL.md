---
name: cargo-crumbkit
description: Measure, document and check a Rust command-line tool built with crumbkit, and audit a tool for places that should use something crumbkit offers and do not, such as typed recoveries, the `Failure` derive, output shapes, effects, convention checks and process handling, and suggest guide text and tool changes from a tool's usage log. Use for cargo crumbkit's commands, for a crumbkit audit and for guides from usage.
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
  line in every Markdown file under the tool's `skill` folder, or in the
  file or folder that `--skill <PATH>` names, names real commands and flags
  of the tool. It exits with 1 when a line fails, so CI can run it.
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
`places`. Equivalent hints in one file make one finding, whose `places` may
lie in several symbols and whose `symbol` is the one that sorts first.

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
| `typed-recovery` | `Recovery::Run` with `Call`, `Recovery::Retry`, `Recovery::after` | A hint, `no_next_step` reason or failure message names a command of this tool in prose, says to run the same command again, or says to make a repair and then run a command. | `-U` `hint\s*[=(]\s*"[^"]*\b<bin> [a-z-]+` `help\("[^"]*\b<bin> [a-z-]+` `(try\|run .*) again`; Read each `recovery` function |
| `provoked-call` | `check_recoveries` | A `Call` omits a required argument, or a declared code is neither a `Failing` start nor `Unprovoked`. | `Recovery::Run` `Call::new` `Failing::new` |
| `row-command` | `Listing::step`, `Step::call`, `RenderCall::render` | A row field, note, summary or `Prose` text names a command of this tool. | `\.note\(` `Listing::step`; Read row fields that are `String` |
| `program-step` | `Step::run`, `Program` | A listing's note or row names a command of another program. | `\.note\(` |
| `alternatives` | `Failure::alternatives` | One hint offers two different repairs. An "or" between two values of one repair is one repair. | `-U` `"[^"]*(, or \| or )[^"]*"` inside `recovery` and hints |
| `failure-fields` | `Failure::fields` | A failure variant holds an id, path, name, status or number that only its message shows. | Read each `Failure` type's variants |
| `derive-failure` | `#[derive(Failure)]` | An `impl Failure` is a `match` per method, or `CODES` is written by hand. | `impl.*Failure\s+for` `CODES:` |
| `declared-code` | `Code::from_spelling`, `Spelling::COLONS` | An error reaches the caller with no declared code, a code is spelled by hand, or a repairable failure offers no step. | `Error::unclassified` `anyhow!` `Recovery::None` `component\(\)` |
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
place that would be a finding only if crumbkit had a feature it lacks is no
finding. When the same pattern appears in two or more tools or crates, it
is a proposal whose text names the missing feature. A pattern seen once
stays out of the report.

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
- `findings`: one entry per symbol and criterion, or per file for
  equivalent hints, with the criterion id, the severity, the file, the
  symbol that sorts first, every `file:line` under `places`, and
  a text that says what is wrong and the proposed change. A finding about
  something absent cites the line where it belongs.
- `proposals`: each `crumbkit-gap`, a pattern seen in two or more tools or
  crates that no affordance covers, with its text and its places.
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

## Suggest guides from usage

This job reads one tool's usage log and suggests, for each command with
findings in its `self usage`, a line of guide text for the tool's
`SKILL.md` or a help topic, and a change to the tool or to the program
that calls it. It reads one tool per run and applies no suggestion.

### Needs

- cargo crumbkit 0.4.0 or later, which has `tool-skill check`. Earlier
  versions name the command `skill check`.
- The tool's crumbkit-usage version. crumbkit-usage writes `lock_ms` and
  `context` from 0.6.0. With an older version the runs have no lock wait,
  so the job reads only `ms` and the `mostly waiting` rows.
- The tool's crumbkit version in `Cargo.lock`. Two crumbkit versions in one
  `Cargo.lock` mean the tool's codes, events and observers may come from
  either, as the audit's `convention-suite` rule says. Use the version that
  the tool's own package depends on, which
  `cargo tree -p <package> -i crumbkit@<version>` shows, and name both
  versions in the report's `note`.

### Reads

Run these in the tool's checkout, with `<bin>` replaced by the tool's
binary name. The window is 14 days unless the request names another.

```sh
<bin> self usage --since 14d --json --full
<bin> self runs list --since 14d --all --json --full
<bin> commands list --json --full
```

`self runs list` needs `--full` for the `at`, `caller`, `flags`, `help`
and `lock_ms` fields.

- `GUIDES.md` beside this skill: one row per finding, with what agents were
  probably trying to do, a guide text template, the tool change, the
  affordances it would use and when the guide alone is enough. Its
  "Program callers" section covers runs that a program made.
- `AFFORDANCES.md` beside this skill, for the **Since** of each affordance.

The installed binary wrote the log, and `tool-skill check` builds the
checkout. Compare `<bin> --version` with the checkout's version and commit,
and say in the report's `note` when they differ, because a guide line
checked against the checkout can name a command that the installed binary
lacks. Set `CARGO_TARGET_DIR` to a scratch directory for the builds of
`tool-skill check`, so they leave the checkout's target directory alone,
and start them through the job runner that your instructions name, if any.

The job gets the outcome `skipped`, and writes nothing, when the tool has
no `self usage` command or its log holds no runs in the window. When the
tool has no `commands list`, the job judges without it and lists every
suggestion under `unverified`.

The usage log holds no value that a caller typed, and a refused run holds
no flags. The job never asks for those values or looks for them
elsewhere. When a judgement needs them, it says so in a proposal.

### Method

1. Take each row of `self usage` whose `runs` field is at least 5. The
   `runs` field counts the runs that show the finding, and
   `figures.runs` counts every run of the command. A row with fewer goes
   under `not_judged` with the reason "fewer than 5 runs".
2. Group the rows by command. All the rows of one command become one
   suggestion with every finding under `findings`, because they usually
   share one cause and one task is enough.
3. Decide who made each command's runs, from `self runs list`. A run came
   from a program when its flags hold `--no-input` or a fixed set that the
   tool's docs do not show, when the runs come at a steady interval in
   `at`, or when the session runs the same few commands and nothing else.
   A run's `caller` field comes from the environment, so a program
   started inside an agent session records the agent's id. Set the
   suggestion's `made_by` to `program` when most of the runs came from a
   program, and to `agent` otherwise.
4. Read the runs before and after each finding in each session by `step`.
   The command the agent ran next, its `flags`, `code`, `ms`, `out_bytes`
   and, from crumbkit-usage 0.6.0, `lock_ms` show what it was trying to
   do. Write that in `intent`. An identical repeat whose runs differ in
   `out_bytes` was probably two calls with different values, so leave
   those repeats out of the count and say so in `intent`.
5. Apply the cost test. Move a finding that costs a caller nothing to
   `rejected`, under its command and with the reason. A command left
   with no finding gets no suggestion. A finding costs nothing when:
   - it is `slow command` and `p95_ms` is under 5,000 ms;
   - it is `identical repeat` and its runs fall under 5 once step 4
     leaves out the repeats with different output;
   - it is `heavy output` or `identical repeat` and every run came from a
     program that reads the output once and discards it.
6. For a suggestion made by an `agent`, write the guide text from each
   finding's row in `GUIDES.md`. Fill each command line with the example
   values of the leaf's `examples` in `commands list`, so the line is one
   an agent can run. When the leaf has no example, write each value as a
   placeholder in capitals, such as `NAME`. A line about one command goes
   in the tool's `SKILL.md`, and a concept that spans commands goes in a
   help topic. A suggestion made by a `program` has no guide text, and
   its tool change follows the "Program callers" section of `GUIDES.md`.
7. Check each guide line. Write the lines to a scratch Markdown file and
   run `cargo crumbkit tool-skill check --bin <NAME> --skill <FILE>` in the
   tool's workspace. The check refuses an unknown command, an unknown
   flag, a flag without its value, a value outside the argument's possible
   values and an extra positional argument. It passes a line that leaves
   out a required argument, and it checks no rule across arguments, such
   as two conflicting flags. So also compare each line with the arguments
   that `commands list` marks `required`, and add any that are missing.
   Rewrite a failing line, or drop it.
8. Write the tool change and its affordances from the rows. An affordance
   whose **Since** is newer than the tool's crumbkit version needs an
   upgrade first, and the tool change says so.
9. Set `guide_enough` from the rows' last column, or `false` for a
   suggestion made by a `program`.

A cause that the log cannot show, such as the values behind a repeat, or a
tool change that no affordance covers, is a proposal for crumbkit. It goes
in `proposals` with the findings that showed it, as the audit's
`crumbkit-gap` does.

### Where suggestions go

1. When `board` is on PATH, run
   `board call tasks.add --title <TITLE> --text <TEXT>` from the tool's
   checkout once per suggestion, so Board routes each task by the
   checkout's git remote. The title names the behaviour the suggestion asks
   for, such as "shelf's skill tells agents to pass every id to one books
   show call". The text holds the command, the findings with their runs,
   the window, `made_by`, the intent, the guide text, the tool change and
   the affordances. When Board finds no board that mounts the tool's
   repository, the caller does not use Board for this tool, so go to the
   next place.
2. When your instructions name another place, such as an issue tracker,
   write one entry per suggestion there with the same fields.
3. Otherwise the suggestions are only in the report.

Each task is one write. When Board refuses a task, for example with
`tasks::repo::ambiguous`, pass `--repo` as the refusal names and add the
rest. A rejected command gets no task.

### Report

Write the report as one JSON document that `guide-report-v1.json` beside
this skill describes, with the id `cargo-crumbkit/guide-report/v1`. It
holds:

- `tool`, `repository`, `window`, `since`, `runs` and `sessions`: what was
  read, and the summary of `self usage`.
- `outcome`: `skipped` when the log could not be read or holds no runs,
  `fail` when the report holds a suggestion, and `pass` otherwise. A
  rejected finding never makes the outcome `fail`.
- `suggestions`: one entry per command, the most runs first, with its
  command, its findings with their runs and codes, `made_by`, the intent,
  the guide text and its target, the tool change and `change_to`, which
  says whether it belongs to the tool or the calling program, the
  affordances,
  `guide_enough` and `written_to`, the task or entry it was written to,
  or `null`.
- `rejected`: each command with findings that failed the cost test, with
  those findings and the reason.
- `not_judged`, `proposals` and `unverified`, as the Method describes.
- `note`: a one-line verdict, which also names a version mismatch between
  the binary and the checkout, and two crumbkit versions in `Cargo.lock`.

Print the report, or write it to the file the request names, in every
case, so it lists which suggestions were written and where.
