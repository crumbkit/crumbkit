# ADR-0191. `self runs export` prints the usage log after a position

- Date: 2026-11-10
- Status: accepted. It amends ADR-0029, which rejected a log that ships off the machine.

## Context

A tool's usage log stays on the machine that wrote it (ADR-0029), and the
reports print no hash (ADR-0029, ADR-0034, ADR-0190). A skill written for a
tool's agents needs to know which commands callers run most and in which
order, across every machine that runs the tool. That needs the log's records,
with the session hash that groups one session's runs, gathered somewhere
else by a job that the person runs.

The log has no stable place to resume from. Past 10 MB, the next append
keeps only the newest lines within 5 MB and renames a new file over the
log, so byte offsets and line numbers change.

## Decision

- crumbkit-usage adds `self runs export`, with the public `SelfRunsExport`,
  `export`, `RunLine` and `SELF_RUNS_EXPORT`, which a tool mounts as an
  `export` verb under `self runs`. It prints a stream (ADR-0041) whose
  document is `<tool>/self-runs-export/v1`, one line per record, oldest
  first.
- Each line holds the stored `<tool>/usage/v1` record under `record`, with
  every field the log stores, including the keyed session and command-line
  hashes. Chain mining needs the session hash to group a session's runs. The
  tool still sends nothing anywhere: the person who runs the export decides
  where its output goes. This amends ADR-0029's rejection of a log that
  ships off the machine for this command only. The reports keep printing no
  hash.
- A position is `RUN@AT`, a run's random span id and its end time to the
  millisecond, and each line gives its own under `position`. `--after`
  takes one and prints the records that follow that run. Without `--after`,
  the export prints the whole log.
- When the run named by `--after` is no longer in the log, the export prints
  every record that ended at or after its time and sets `gap` on the first
  line, so a script sees that records may be missing. It never fails for
  this.
- The export takes no lock. It prints only lines that end with a newline,
  so it never prints the half line of a run that is still writing. Holding
  the lock while it streams to a slow reader would make concurrent runs
  drop their lines after 100 ms.

## Consequences

An export of a large log stops at the tool's output budget, 1 MiB by
default, with the over-budget failure, and the caller exports again after
the last position it read. An export after a trimmed run may repeat a run
that ended in the same millisecond as the position, which a reader drops by
its span id. The JSON orders each record's keys by name, as crumbkit's JSON
does, so a record's text can differ from the stored line while its fields
and values are the same. A tool passes `export` to crumbkit-test's grammar
check as one of its verbs, as it passes `usage`.

Considered: a byte offset or line number as the position, which a trim
invalidates; the span id alone, which cannot fall back to a time; a second
flag `--after-time`, which splits one position over two flags; the lock
held while reading, which drops concurrent runs' lines; printing each stored
line without crumbkit's envelope, which leaves the gap and the position
nowhere to go; leaving the session hash out, which stops chain mining.
