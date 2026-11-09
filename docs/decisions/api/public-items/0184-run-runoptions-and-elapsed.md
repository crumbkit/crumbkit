# ADR-0184. A run's handle, a run's settings and a printed length of time get their own names

- Date: 2026-11-09
- Status: accepted
- Supersedes: ADR-0148, ADR-0178 and ADR-0180, which it amends
- Missed: ADR-0178 named the process crate's settings `Limits`, although standard input and a cancel token limit nothing. ADR-0180 gave the name `Output` to two unrelated types and left only the module path to tell them apart.

## Context

`crumbkit::Output` is the handle of one run: `parse` returns it, and a tool
passes it by value to `finish` at the end of the run. `crumbkit::tool::Output`
is the section of the `Tool` record that declares what a tool prints.
ADR-0180 kept both names, so a reader had to check the module path to learn
which type a line meant.

`crumbkit_process::Limits`, which ADR-0178 named, holds standard input, the
capture of each output stream, a deadline, a cancel token, an escalation and a
scope. Only some of those are limits.

`crumbkit::Duration`, which ADR-0148 added, is a printable value beside `Age`
and `Bytes`. It shares its name with `std::time::Duration`, so a handler that
measures a time and prints it has to qualify or alias one of the two.

## Decision

- `crumbkit::Output` is `crumbkit::Run`, because it is the handle of one run.
  `crumbkit::tool::Output` keeps its name, because it declares output.
- A public function that takes the handle names its parameter `run`.
  Crumbkit's documentation and examples say `run` as well. Private code and
  tests may keep a local variable named `output`.
- `Stop::output` and `Refused::output`, which return the handle, are
  `Stop::run` and `Refused::run`.
- Prose writes `Recovery::Run` in full wherever a bare `Run` could mean that
  variant, because `Run` alone now means the handle.
- `crumbkit_process::Limits` is `RunOptions`, because it holds every option of
  one `run`. The parameter of `run` is `options`.
- `crumbkit::Duration` is `crumbkit::Elapsed`, because it holds a length of
  time that has passed, and it lives in `elapsed.rs`.
- Its text and JSON forms do not change, and neither does the JSON Schema of a
  field that holds it.

No deprecated alias keeps an old name. The only users of these crates are the
project's own tools, and they change in the same round.

## Consequences

A tool that names any of the three types changes the name at 0.2. ADR-0002
allows a breaking change in a 0.x minor release.

Dated decision records keep the old names (ADR-0017).

Considered for the handle: renaming the record to `tool::Printing` and keeping
`crumbkit::Output`, which keeps a noun that says less about the handle. For
the length of time: `TimeSpan`, which also suits a timeout, while most uses in
a row report a length of time that has passed. For each of the three: keeping
the name, which leaves the problem each section above describes.
