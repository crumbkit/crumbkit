# Usage report

`self usage` reads a tool's [usage log](README.md) and reports the patterns
where callers waste effort. `self figures list` prints the figures of every
command in the log. Both are part of the crumbkit-usage crate. The rustdoc of
[`self_usage`](../../../crates/crumbkit-usage/src/self_usage.rs) and
[`self_figures`](../../../crates/crumbkit-usage/src/self_figures.rs) shows
the wiring and an example of each output, and shelf mounts both commands.

## Mounting the commands

A tool adds a `self` group with a `usage` subcommand that flattens
[`SelfUsage`](../../../crates/crumbkit-usage/src/self_usage.rs). Its
handler passes the result of `crumbkit_usage::findings`, which takes the tool's
`Guide`, to `finish` with the document `SELF_USAGE`.

A tool mounts `self figures list` as a `figures` resource with the `list`
verb under `self`, so it fits the [command grammar](../help/README.md). Its
handler passes the result of `usage::figures` to `finish` with the document
`SELF_FIGURES_LIST`. `cargo crumbkit weigh --usage` runs it, as the
[weight](../help/weight.md) page describes.

Why: ADR-0034, ADR-0033.

## What the report reads

`--since` takes a humantime duration, such as `12h`, `7d` or `2w`, and
defaults to `7d`. The report reads the runs that ended in that duration
before now, and the summary names the UTC day on which the duration starts.
`--log <FILE>` reads that file in place of the tool's own log, such as a
copy from another machine.

The report reads the log once, a line at a time. It holds at most 1,025
bytes of a line, which is the longest record and its newline, and skips a
longer line as damaged. It keeps counts per command and what each session's
findings need of its earlier runs, and never a copy of the file. A line that
is not a whole `crumbkit/usage/v1` record is skipped. On a 10 MB log of
about 29,000 records, the release build of shelf runs `self usage` in about
45 ms, which includes the trim that the run's own log line causes.

A tool whose root command has an empty name has no name to find its log
by, and a missing or unreadable log reads as empty. Either way the report
has no runs and exits 0, because every usage-log failure stays silent.

Why: ADR-0034.

## Failed and refused runs

A run counts as failed when it exited with 1 and no signal ended it. A run
counts as refused when it exited with the code that the guide gives a
refused command line, which is 2 unless the guide lists another, as the
[exit codes](../failures/exit-codes.md) page describes. A run that exited
with any other of the tool's own codes, 3 to 255, counts as neither,
because its meaning belongs to the tool.

Why: ADR-0034, ADR-0002, ADR-0056.

## Findings

The report has eight findings. Each counts runs under one command path.

| Finding | What it counts |
| --- | --- |
| `one id at a time` | A session ran a command that takes ids with one id at least 3 times within 120 s. |
| `identical repeat` | A session ran the same command line again within 60 s. |
| `help loop` | A session read at least 3 help pages before its first successful run, or without one. The pages count under the command of the last page. |
| `usage error` | The parser refused the command line. The row names clap's nearest flag when the record has one. |
| `retry after failure` | A session ran a command again within 60 s of its failure. The row names the failure's code. |
| `short page` | A session followed a cut-short listing within 60 s with the same command and `--all` or `--limit`. The record holds no limit value, so any `--limit` counts, also one that asks for fewer rows than the default page. |
| `heavy output` | The 95th percentile of a command's output is at least 20,000 bytes, which is about 5,000 tokens of a coding agent's context. |
| `slow command` | The 95th percentile of a command's wall time is at least 1,000 ms. |

Why: ADR-0034.

## Checking records

The report checks each record against the tool's command tree before it
counts it, so a hand-edited line cannot put its text in the report.

- The command path must name real subcommands by their names, so an alias
  path fails.
- Each flag and the nearest flag must be a long name of the command's
  arguments, or a short name of an argument without a long one.
- A code and each tolerated code must be 1 to 128 ASCII letters, digits,
  `_`, `-`, `:`, `.` and `/`. A code in a predefined spelling or a custom
  one passes, and a code with whitespace or any other character fails.
- A caller must be `person`, `unknown` or an id that is-ai-agent gives.

A record that fails a check is left out and counted in the summary's
`skipped`.

Why: ADR-0034.

## Output

The output is a [listing](../output/listings/README.md) whose rows are the
findings, the most runs first. The report never prints a hash, a session or
a time finer than a day. Text prints a summary line above the table, such
as `1,284 runs in 23 sessions since 30 Sep: claude-code 1,201, person 83.`
The text summary line ends with the codes that the most runs tolerated, when
there are any.

JSON is the `crumbkit/self-usage/v1` document, checked in as
`crates/crumbkit-usage/schemas/self-usage-v1.json`. Its `summary` holds the runs,
sessions, callers, since day, skipped count and the 10 codes that the most
runs tolerated. With `--json --full`, each row also holds the figures of its
command, such as its runs, failures and 95th percentiles, and the `Next:`
line points there. A command without a finding has no row, so `--full`
gives no figures for it.

Two figures need a definition.

- `help_before` counts the help pages that a session read since its
  previous run that printed no help page. The count is credited to the
  session's next command.
- `exits` counts the runs that exited with each code from 3, with the
  meaning that the guide declares for that command or null. `failed` still
  counts exit 1 only.

Why: ADR-0034, ADR-0016, ADR-0037.

## self figures list

`self figures list` prints the figures of every command in the log, one row
each in path order, with the root under the tool's name. It pages with
`--limit` and `--all`, takes the flags of `self usage`, applies the same
record checks and prints the same summary. Its document is
`crumbkit/self-figures-list/v1`, checked in as
`crates/crumbkit-usage/schemas/self-figures-list-v1.json`.

Why: ADR-0034.

## Tests

shelf's tests write a log of wasteful runs through the binary and compare
the text and `--json --full` output with golden files. One test checks that
the report holds no value that the runs typed and no hash. Another plants
four edited records and checks that the summary counts them as skipped.
shelf opts out of `check_examples` for `self usage` and `self figures list`,
because their output names the current day.

Why: ADR-0034.
