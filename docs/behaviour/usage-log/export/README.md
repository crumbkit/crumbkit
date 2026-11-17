# Usage export

`self runs export` prints the records of a tool's [usage log](../README.md)
after a run, so a job that the person runs can gather them from many
machines. It is part of the crumbkit-usage crate, and the rustdoc of
[`self_runs_export`](../../../../crates/crumbkit-usage/src/self_runs_export.rs)
shows the wiring, and shelf mounts it.

A tool mounts it as the `export` verb of the `runs` resource under `self`,
beside `self runs list` from the [usage report](../report.md). Its handler
returns the exit code of `crumbkit_usage::export`, which takes the `Run`
and prints the stream `SELF_RUNS_EXPORT`. The tool passes `export` to
crumbkit-test's grammar check as one of its verbs.

`self runs export` prints one line for each record in the log, oldest
first, as a [stream](../../output/streams.md). It reads the whole log and
skips a line that is not a whole `<tool>/usage/v1` record, and a line that
the [report's check](../report.md) refuses, because anyone with the user's
rights can edit the log and the text line prints the command. A record
that the check refuses is also not found by `--after`. When an upgrade
removes a command or flag that the last record read names, the next export
treats that run as trimmed and prints every record again, with `gap` set.
It prints only lines that end with a newline and takes no lock, so it never
prints the half line of a run that is still writing and never makes a run
wait.

Each line has two fields.

- `gap` is true on the first line when records may be missing before it.
- `record` is the stored record with every field the log holds, including
  the run's span id under `run` and the keyed session and command-line
  hashes. Its keys are ordered by name.

`--after <RUN>` takes a span id, 16 lowercase hex digits, and prints the
records after the run with that id. A caller that keeps the `run` of the
last record it read exports only what is new. A span id is random, so a
log holds no two runs with the same one in practice.

When the run that `--after` names is no longer in the log, because a trim
removed it, the export prints every record in the log and sets `gap` on the
first line. A trim keeps the newest lines, which were all appended after
the removed run, so the export repeats no record. The records appended
between the removed run and the oldest kept line are lost, and `gap` tells
the reader so. The export does not fail. `--log <FILE>` reads another log,
as it does for the report.

The text prints `<RUN>  <COMMAND>  <EXIT>` for each record, with
`after a gap` at the end of the first line when `gap` is true. The JSON is
one `<tool>/self-runs-export/v1` document per line, checked in as
`crates/crumbkit-usage/schemas/self-runs-export-v1.json`.

The export prints within the tool's output budget, 1 MiB unless the tool
record sets another. When the next line would pass it, the stream prints
the over-budget failure and exits with 1, and the caller exports again
after the last run it read.

The tool sends nothing anywhere. The person who runs the export decides
where its output goes.

Why: ADR-0191, ADR-0041.

## Tests

Tests of `self runs export` read a fixed log after a run and after a
trimmed run. Another checks that an export of shelf's own log prints its
stored records, and that the next export after the last run prints only the
newer line. Others check that an export skips a record whose command is no
command of the tool, in text and in JSON, and that `--after` naming such a
record prints every known record after a gap. Another lowers the output
budget, checks that the export stops with the over-budget failure, and
resumes after the last run it printed.

Why: ADR-0191.
