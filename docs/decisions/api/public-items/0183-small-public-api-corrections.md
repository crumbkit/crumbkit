# ADR-0183. Small corrections to the public API before 0.2.0

- Date: 2026-11-09
- Status: accepted
- Supersedes: ADR-0174, ADR-0175, ADR-0178 and ADR-0088, which it amends
- Missed: ADR-0178 renamed `Run`, `Exit` and `Finished` because crumbkit's own items have those names, and it left `Stop`, which `crumbkit::Stop` also names. ADR-0174 turned down `Exit::Ended(Status)` because it changes every match, and it did not weigh that one variant removes the repeated status for every later caller. ADR-0175 kept `RecoveryStep` at `crumbkit::form`, and it did not ask which crate should own that path: crumbkit-test reads the printed failure, and an outside tool that imports the path moves to `crumbkit_test`.

## Context

Before crumbkit's 0.2.0 release, several small parts of the public API made a
caller write more than a call needs, or kept a name or a path that serves no
caller. Each is small on its own and none needs a record of its own.

- A command that ends without `finish` wrote three calls: `print_failure`,
  `failure_exit` and `events::ended`. A caller that left out the last compiled
  and lost the end-of-run record.
- `crumbkit-process` had `Outcome::Code`, `Outcome::Signal` and a `Status` with
  the same two cases. Its `Stop` clashed with `crumbkit::Stop` as ADR-0178's
  other names had.
- `Recovery::None` needed a struct literal and `.to_owned()` at each use, and
  `#[failure(...)]` had no key for a failure with no next step.
- `crumbkit::form` held the type that crumbkit-test reads a printed failure
  into. An outside tool also imports it, and moves to `crumbkit_test`.
- Items that only a sibling crate reads were marked inconsistently.
  `crumbkit::NoInput` had a second public path, and `crumbkit_test::Tool` had no
  caller.
- `Output` had no getter for `--no-input`.

## Decision

- `Output::fail` prints a failure, reports its exit code with `events::ended`
  and returns it. `print_failure` stays for a command that passes on another
  exit code.
- `Outcome::Ended(Status)` replaces `Outcome::Code` and `Outcome::Signal`, and
  `Stop` is `StopReason`.
- `Recovery::none` and `Recovery::hint` build those recoveries from text. The
  derive takes `no_next_step = "reason"`, formatting braces as `hint` does.
- `ReadFailure` moved to crumbkit-test, which also exports `RecoveryStep`. The
  `form` module is gone. `RecoveryStep` stays defined in crumbkit, which prints
  it, and is hidden from crumbkit's documentation.
- A `pub` item that exists only for a sibling crate carries `#[doc(hidden)]` and
  a comment naming the sibling. The seven line helpers that crumbkit-markdown
  reads are one such set: `Effects::line`, `Effects::writes_line`,
  `help::STREAM_LINE`, `help::BYTES_LINE`, `help::HEADED_LINE`,
  `help::text_line` and `help::children_line`. `NoInput` has one path,
  `crumbkit::NoInput`, and `crumbkit_test::Tool` is gone.
- `Output::no_input` reports whether the caller passed `--no-input`.

## Consequences

A command that ends on a failure makes one call, and the run's end is reported.
A match on `Outcome` handles the child's own end once, in `Outcome::Ended`.
Each `Recovery::None` keeps its literal form for the code that matches on it.

Considered: making `print_failure` return the exit code, which would leave no
call for a command that passes on another code; adding `Recovery::none` and
removing the variant's public fields, which would stop a caller from reading
the reason; leaving `ReadFailure` in crumbkit and hiding it, which keeps a
module with one test helper in the crate that tools link.
