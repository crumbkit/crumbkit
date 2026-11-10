# Usage export

`self runs export` prints the records of a tool's [usage log](../README.md)
after a position, so a job that the person runs can gather them from many
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
applies none of the report's record checks, because it prints what the log
stores, but it skips a line that is not a whole `<tool>/usage/v1` record.
It prints only lines that end with a newline and takes no lock, so it never
prints the half line of a run that is still writing and never makes a run
wait.

Each line has three fields.

- `position` is `RUN@AT`, the run's span id and its end time to the
  millisecond, such as `00f067aa0ba902b7@2026-10-11T11:56:40.120Z`.
- `gap` is true on the first line when records may be missing before it.
- `record` is the stored record with every field the log holds, including
  the keyed session and command-line hashes. Its keys are ordered by name.

`--after <RUN@AT>` prints the records after the run at that position, so a
caller that keeps the last position it read exports only what is new. When
that run is no longer in the log, because a trim removed it, the export
prints every record that ended at or after the position's time and sets
`gap` on the first line. Such an export may repeat a run that ended in the
same millisecond as the position, and a reader drops it by its span id.
`--log <FILE>` reads another log, as it does for the report.

The text prints `<POSITION>  <COMMAND>  <EXIT>` for each record, with
`after a gap` at the end of the first line when `gap` is true. The JSON is
one `<tool>/self-runs-export/v1` document per line, checked in as
`crates/crumbkit-usage/schemas/self-runs-export-v1.json`.

The export prints within the tool's output budget, 1 MiB unless the tool
record sets another. When the next line would pass it, the stream prints
the over-budget failure and exits with 1, and the caller exports again
after the last position it read.

The tool sends nothing anywhere. The person who runs the export decides
where its output goes.

Why: ADR-0191, ADR-0041.

## Tests

Tests of `self runs export` read a fixed log after a position and after a
trimmed run. Another checks that an export of shelf's own log prints its
stored records, and that the next export after the last position prints
only the newer line.

Why: ADR-0191.
