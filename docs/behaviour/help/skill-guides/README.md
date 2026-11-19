# Skill guides from usage

This page covers the "Suggest guides from usage" job of cargo crumbkit's
own [skill](../../../../crates/cargo-crumbkit/skill/SKILL.md), which an
agent loads after `cargo crumbkit skills install`. The job reads one
tool's [usage report](../../usage-log/report.md) and suggests guide text
and tool changes for the findings in it.

Why: ADR-0258.

## What the job reads

The job needs cargo crumbkit 0.4.0 or later, which has `tool-skill check`.
It runs four commands of the tool in its checkout: `self usage` with
`--json --full`, `self runs list` with `--all --json` and `--fields`
naming the fields it reads, `self runs export --json`, and
`commands list --json --full`. The fields include `at`, `caller`, `flags`
and `help`, and `lock_ms`, which crumbkit-usage writes from 0.6.0. The
usage commands read the same window, which is 14 days unless the request
names another. When the output budget cuts the runs short, the job
shortens the window and says so in the report's note. A tool without
`self usage`, or a log with no runs in the window, gives the outcome
`skipped`, and the job writes nothing.

The installed binary wrote the log, and `tool-skill check` builds the
checkout, so the report's note says when the two versions differ. The job
builds into a scratch target directory. When `Cargo.lock` holds two
crumbkit versions, the job uses the one that the tool's own package
depends on and names both.

The usage log holds no value that a caller typed, and a refused run holds
no flags. The job never asks for those values, and a judgement that needs
them becomes a proposal for crumbkit.

## How the job judges a finding

The job judges each row of `self usage` whose `runs` field is at least 5,
and lists a row with fewer under `not_judged`. The rows of one command
become one suggestion. The job reads each finding's row in
[GUIDES.md](../../../../crates/cargo-crumbkit/skill/GUIDES.md), which
holds one row for each of the nine findings. A row says what agents were
probably trying to do, a template for the guide text, the tool change that
would remove the cause, the affordances from
[AFFORDANCES.md](../../../../crates/cargo-crumbkit/skill/AFFORDANCES.md)
that the change would use, and when the guide text alone is enough.

The job reads the runs around each finding in `self runs list` to say
what the caller was trying to do. It first takes the runs that a
[program marked](../../usage-log/program/README.md) with
`<TOOL>_PROGRAM`, which `self runs export` shows in the `program` field,
and runs `self usage` again with `--exclude-program` to see what remains.
An unmarked program's `caller` names the agent whose environment it
inherited, so the job marks unmarked runs as a program's when they carry
`--no-input` or a fixed set of flags, come at a steady interval, or fill
sessions that run nothing else. A suggestion made by a program gets a
tool change and no guide text. An identical repeat counts command lines
that match word for word, values included, so a repeat whose output size
changed read state that had changed.

A cost test moves a finding that costs a caller nothing to the report's
`rejected` list with its reason, such as a slow command whose 95th
percentile is under 5 seconds. A rejected finding never makes the outcome
`fail`.

The job fills each template with the example values of the leaf's
examples in `commands list`, or with placeholders in capitals when the
leaf has none. It checks each guide line with
[`tool-skill check`](../skill-check/README.md) before it writes the line.
That check refuses unknown commands and flags, missing flag values and
extra positional arguments, and passes a line that leaves out a required
argument, so the job also compares each line with the arguments that
`commands list` marks required. A line that fails is rewritten or dropped.
Without `commands list`, the job lists its suggestions under `unverified`.

A test in cargo-crumbkit fails when `GUIDES.md` lacks a row for a finding
that `self usage` reports, or names an affordance that `AFFORDANCES.md`
does not list.

## Where the suggestions go

When the caller's instructions say that it keeps tasks in Board for the
tool's repository, the job writes one Board task per suggestion with
`board call tasks.add`, titled with the behaviour the suggestion asks
for. The job does not decide from `board` being on PATH, because
`tasks.add` writes to another repository when no board mounts the tool's.
Otherwise it writes them to the place that the caller's instructions name,
such as an issue tracker.

In every case the job prints a JSON report, or writes it to the file the
request names. The report is the `cargo-crumbkit/guide-report/v1`
document, which
[guide-report-v1.json](../../../../crates/cargo-crumbkit/skill/guide-report-v1.json)
describes. Each suggestion's `written_to` names the task or entry it went
to, so a run that stops after some tasks shows which were written. Its
outcome is `fail` when it holds a suggestion and `pass` when it holds
none.
