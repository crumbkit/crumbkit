# ADR-0095. crumbkit-child hands over text, the usage log records its own signals, and `Menu` lives in crumbkit

- Date: 2026-10-19
- Status: amended by ADR-0144 and ADR-0162
- Supersedes: ADR-0042, ADR-0029 and ADR-0043, which it amends

## Context

ADR-0042 gave crumbkit-child `hand_over_json` and `Json`, so a command
could hand the terminal to a child that prints a JSON document when asked,
and the codes `crumbkit::child::no_json`, `crumbkit::child::bad_json` and
`crumbkit::child::failed` for the failures of that path. ADR-0029 gave
crumbkit-usage `start_with(Signals::Tool)`, so a tool with its own SIGINT
or SIGTERM handler could keep the log running, and `section`, so a tool
could add fields of its own to the run's line under `ext`.

No tool in this repository calls any of them. The one hand-over in the
examples runs a pager, and the example tool starts the log with `start`
and adds no section. Each item still cost tests, documentation and a
promise of compatibility after the first release, and under API-1 a seam
needs a second use.

ADR-0043 placed `Menu` in the separate crumbkit-picker crate and listed a
terminal library for that crate under Considered. No such library was
added: the menu depends on crumbkit alone and implements crumbkit's own
`Picker` trait, so the crate held one type that crumbkit could hold, with
a README, a release entry and a dependency line in each tool that uses
it.

## Decision

- `hand_over_json`, `Json`, `codes::NO_JSON`, `codes::BAD_JSON` and
  `codes::FAILED` are removed from crumbkit-child. `hand_over` stays, and
  it refuses `--json` with `<tool>::cli::usage` and the exit code of a
  refused command line, as it refuses `--fields`, because the child's
  output is text that the terminal shows. A tool that runs another
  crumbkit tool as a child and wants its JSON runs the child itself, reads
  the bytes of its standard output with its own serde_json call, and
  prints the value with `finish`.
- `start_with`, `Signals` and `section` are removed from crumbkit-usage,
  with the record's `ext` field and the `Section` definition of the
  `<tool>/usage/v1` schema. `start` keeps the behaviour that
  `start_with(Signals::Crumbkit)` gave it: the log's thread writes the line
  with `"interrupted": true` on SIGINT or SIGTERM and lets the signal end
  the process. A tool that handles Ctrl-C itself keeps its own handler and
  runs without the log.
- crumbkit-test's exit check no longer treats `crumbkit::child::no_json`
  as a refused command line, and its list of the add-on crates' codes holds
  crumbkit-child's two remaining codes, `crumbkit::child::not_found` and
  `crumbkit::child::not_started`.
- `Menu` moves into crumbkit as `crumbkit::pick::Menu`, and the
  crumbkit-picker crate goes, with its workspace entry, its release entry
  and its README. A tool that sets `Menu` on its guide depends on crumbkit
  alone. A picker with a terminal dependency goes in the tool or in a
  crate of its own, under ADR-0003.

## Consequences

A test that names a removed item fails to compile, and a compile_fail
doctest in each crate's `removed.rs` keeps each removal in place. The
usage record loses the one field that crumbkit could not check, so every
field of the line is one that crumbkit writes from a name, a count, a
flag, a time, a keyed hash or a trace id. crumbkit-child no longer depends
on serde or serde_json. crumbkit's `Output::refuse_view` and
`Output::listed_exit` keep no caller in this repository, and a later
record decides whether they stay. A removed item can return through a new
record when a second use appears (API-1). The workspace publishes nine
crates in place of ten, shelf and reading drop one dependency, and
crumbkit's default build gains no dependency, because the menu uses the
standard library alone.

Considered: keeping `hand_over_json` for a tool that wraps another
crumbkit tool, which no tool in this repository does; keeping
`Signals::Tool` for a tool with its own signal handler, which no tool in
this repository has; keeping `section` for a tool's own fields, which no
tool sets and which crumbkit could not check; keeping crumbkit-picker as
the home of a picker with a terminal dependency, which goes in the tool or
in a crate of its own under ADR-0003.
