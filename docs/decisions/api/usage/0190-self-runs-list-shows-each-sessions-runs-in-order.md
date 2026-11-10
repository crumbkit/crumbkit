# ADR-0190. `self runs list` shows each session's runs in order

- Date: 2026-11-10
- Status: accepted. The Context credits ADR-0034 with the rule that the report prints no hash, but that rule comes from ADR-0029, which writes the command line only as a keyed hash. This record departs from ADR-0034's rule that the report holds at most one record in memory, because `self runs list` holds every run in its window.

## Context

ADR-0034 gave a tool `self usage` and `self figures list`, which reduce the
usage log to findings and per-command figures. A page that shows how a
tool is used, such as flightdeck's usage page, also needs the runs
themselves: which command followed which within a session, one lane per
session, and how a run joins the runs of its children in another tool's
log. It also needs a typical run beside the worst ones, and the figures
gave only 95th percentiles. flightdeck must not read a tool's log file
itself, because the reader's record checks live in crumbkit-usage.

ADR-0034 kept the report from printing a hash, a session or a time finer
than a day.

## Decision

- crumbkit-usage adds `self runs list`, with the public `SelfRuns`,
  `runs`, `RunRow` and `SELF_RUNS_LIST`, which a tool mounts as a `runs`
  resource with a `list` verb under `self`. Its document is
  `<tool>/self-runs-list/v1`. It prints one row per run in the `--since`
  window, grouped by session and in log order within each session, with
  the summary of `self usage`.
- A row names its session by an ordinal from 1, in the order of each
  session's first run in the window. The keyed hash of the session never
  reaches the output, so a listing cannot be joined to another listing or
  another machine's log by the hash.
- A row gives the run's end in RFC 3339 to the second in UTC. A page needs
  the order and the gaps between runs, and the second is enough for both.
  The milliseconds stay in the log.
- A row gives the run's W3C trace id, span id and parent span id. They are
  random ids that the log shares with the caller and the run's children
  (ADR-0030), so they hold no argument value, and a page needs them to join
  a run to its child runs. The reader prints them only when they are
  lowercase hex of the right length, and skips and counts any other record.
- The figures of each command gain `p50_ms`, `p50_out_bytes` and
  `cut_short`. The medians use the nearest-rank method of the 95th
  percentiles, and `cut_short` counts the runs whose line says a listing or
  stream cut the output short.
- The reader reads the log once and feeds both the tally of the summary and
  the rows from that pass, so the record checks stay in one place.

## Consequences

`self runs list` holds a row for every run in its window in memory, so its
memory grows with the runs that `--since` admits. The usage log's line
format does not change. The `self-usage-v1` and `self-figures-list-v1`
schemas gain the new figures, and readers that ignore unknown fields, such
as `cargo crumbkit weigh --usage`, read them as before.

The new public items meet API-1 through a second adopter: flightdeck's
usage page reads `self runs list`, and shelf mounts it in `examples/`.

Considered: printing the session hash, which joins listings across logs
and machines; times to the millisecond, which add nothing a page draws;
leaving the trace ids out, which stops a page from joining a run to its
children; a second reader for the rows, which would copy the record checks.
