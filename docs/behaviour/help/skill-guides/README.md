# Skill guides from usage

This page covers the "Suggest guides from usage" job of cargo crumbkit's
own [skill](../../../../crates/cargo-crumbkit/skill/SKILL.md), which an
agent loads after `cargo crumbkit skills install`. The job reads one
tool's [usage report](../../usage-log/report.md) and suggests guide text
and tool changes for the findings in it.

Why: ADR-0258.

## What the job reads

The job runs three commands of the tool in its checkout: `self usage` with
`--json --full`, `self runs list` with `--all --json`, and
`commands list --json --full`. Both usage commands read the same window,
which is 14 days unless the request names another. A tool without
`self usage`, or a log with no runs in the window, gives the outcome
`skipped`, and the job writes nothing.

The job reads no value that a caller typed, because the usage log holds
none.

## How the job judges a finding

The job judges each finding of `self usage` with at least 5 runs, and
lists a finding with fewer under `not_judged`. It reads the finding's row
in [GUIDES.md](../../../../crates/cargo-crumbkit/skill/GUIDES.md), which
holds one row for each of the nine findings. A row says what agents were
probably trying to do, a template for the guide text, the tool change that
would remove the cause, the affordances from
[AFFORDANCES.md](../../../../crates/cargo-crumbkit/skill/AFFORDANCES.md)
that the change would use, and when the guide text alone is enough.

The job reads the runs around each finding in `self runs list`, such as
the command that the session ran next, to say what the agent was trying to
do. It fills the template with the tool's real commands, flags and
figures, and checks each guide line with
[`tool-skill check`](../skill-check/README.md) before it writes the line.
A line that fails is rewritten or dropped. Without `commands list`, the
job lists its suggestions under `unverified`.

A test in cargo-crumbkit fails when `GUIDES.md` lacks a row for a finding
that `self usage` reports, or names an affordance that `AFFORDANCES.md`
does not list.

## Where the suggestions go

When `board` is on PATH and a board mounts the tool's repository, the job
writes one Board task per suggestion with `board call tasks.add`, titled
with the behaviour the suggestion asks for. Otherwise it writes them to
the place that the caller's instructions name, such as an issue tracker.

In every case the job prints a JSON report, or writes it to the file the
request names. The report is the `cargo-crumbkit/guide-report/v1`
document, which
[guide-report-v1.json](../../../../crates/cargo-crumbkit/skill/guide-report-v1.json)
describes. Each suggestion's `written_to` names the task or entry it went
to, so a run that stops after some tasks shows which were written. Its
outcome is `fail` when it holds a suggestion and `pass` when it holds none.
