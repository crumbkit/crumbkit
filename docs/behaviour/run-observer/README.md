# Run observer

crumbkit tells each installed run observer what each run did. The usage log
is such an observer, and a tool can install its own beside it. The rustdoc of the
[`events`](../../../crates/crumbkit/src/events.rs) module describes the
`Observer` trait, the `observe` function that installs an observer, and the
reports `aliased`, `tolerated` and `ended`, which a tool or another crate
calls. This page states the rules that the rustdoc leaves to the records.

## Installing an observer

An observer is a `&'static dyn Observer`. A process holds up to four, in a
fixed array of four `OnceLock` slots, and `observe` fills the first free
slot. When all four slots are full, `observe` installs nothing and returns
false, and a tool that needs more installs one observer that forwards to
several. The [usage log](../usage-log/README.md)'s `start` calls `observe`,
so a tool's own observer and the log both hear each run, in either order of
installation.

Each report goes to every installed observer in the order they were
installed. An observer installed after a report hears only the reports that
follow it. `observe` installs an observer again when it is called with it
again, so that observer would hear each report twice. The usage log's
`start` therefore installs its observer only on its first call in a
process.

`observe` installs nothing in a run whose completion variable asks for
completion, by the rule that `complete` reads, so no observer hears a Tab
press ([shell completion](../parsing/completion.md)). The `complete` module
owns that read in every build.

The `events` module has no cargo feature, so a call site in crumbkit has no
`cfg` of its own and every build compiles the same reports. The reports take
no guide, because `ended`, `tolerated`, crumbkit-aliases' `aliased`, the
usage log's signal thread and `trace::child_env` have none.

Why: ADR-0057, ADR-0075, ADR-0048.

## What crumbkit reports

- `parse` reports the typed words, the parsed command line, a help page
  with its command path and exit, a refusal, a picker's count and any other
  exit. A refusal reports `refused`, or `exited` for a refused view flag,
  when it prints. A refusal that `try_parse` returns and the tool never
  prints reports neither, and the tool reports its exit through `ended`.
- `finish`, `finish_to` and a stream report the code of a failure in the
  tool's spelling, then the exit code, the bytes written to standard output
  and whether a listing left rows out or had none.
- A [child hand-over](../output/child-processes.md) in the crumbkit-child
  crate reports through the public `events::handed_over`, `taken_back` and
  `interrupted` functions that the child took the terminal, on Windows that
  it gave the terminal back, and that a signal ended the child. The usage log's signal thread needs the hand-over,
  because it lets SIGINT end the process only while no child holds the
  terminal.

Each observer answers three questions, and crumbkit combines the answers:

- whether it handles SIGINT, so a hand-over installs no handler of its own.
  The answer is yes when any observer says yes.
- the variables that put a child in the run's
  [trace](../usage-log/trace-context.md). `trace::child_env` returns each
  observer's variables in the order the observers were installed, and
  leaves out a variable whose name an earlier observer already gave.
- the trace id and run id for the `tracing` spans. The spans record the ids
  of the first observer that gives them.

Why: ADR-0057, ADR-0075, ADR-0030, ADR-0042, ADR-0100.

## Compatibility and cost

Every method of `Observer` has a default body that does nothing or answers
that there is nothing, so crumbkit can add an event in a minor release. A
refusal and a finished run arrive as the structs `RefusalEvent` and
`Finished`, whose fields are private, so their getters let them gain a
field. `RefusalEvent::nearest_flag` reads clap's suggestion only when the observer
asks, and returns no hidden flag.

A report with no observer installed is one atomic load and a branch. A
report with observers installed loads one more slot after the last
observer, unless all four slots are full. Work that only an observer needs,
such as the command path of a help topic, happens after the first check. `finish_to` always counts the bytes it writes,
which adds one addition per write.

Why: ADR-0057, ADR-0075, ADR-0002, ADR-0160.
