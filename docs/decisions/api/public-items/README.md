# ADR-0085. Public items without a caller removed

- Date: 2026-10-18
- Status: amended by ADR-0088, ADR-0089, ADR-0160 and ADR-0162
- Supersedes: ADR-0001, ADR-0005, ADR-0045 and ADR-0081, which it amends
- Missed: ADR-0001 added `Code::runtime` for plugin codes, but no tool called it and `RuntimeLeaf` has no way to declare a code, so a run-time code could reach neither help nor `check_codes`. ADR-0045 and ADR-0081 kept `FailureReader` for a tool with its own failure shape, but no tool implemented it outside a doc example.

## Children

- [ADR-0160. Names and visibility before the first release](0160-names-and-visibility-before-the-first-release.md)

## Context

crumbkit 0.1.0 had not been published, so removing a public item cost no
adopter a breaking change. A review of the public items found some that no
tool, example or sibling crate called. Each one still cost documentation,
tests and a promise of compatibility after the first release. Under API-1, a
seam needs a second use, and none of these items had a caller.

This record amends ADR-0001 by removing `Code::runtime`. It amends ADR-0005
by removing the fresh copy of a fixture and the redactions that the
examples could name. It amends ADR-0045 by removing `FailureReader`, which
a tool could set in place of a failure form. It amends ADR-0081 by
removing `FailureReader` and the public `crumbkit::form` module, which that
record kept for a tool with its own failure shape.

## Decision

- `Code::runtime` and `RuntimeCodeError` are removed. A tool whose plugins
  bring codes declares a `Code::new` for each code it knows.
- `Page::fetch_limit`, `Stop::number`, `Expect::get`, the `Expected` enum
  and `Bytes::into_vec` are removed. A tool fetches one row past
  `Page::limit`, or every row when it is `None`, returns `Stop::code`, and
  calls `Expect::check`.
- `Spelling::DOTS` and `Spelling::SLASHES` are removed. `Spelling::COLONS`
  is the one built-in spelling, and a tool with another separator uses
  `Spelling::custom`. `Spelling::split` is removed, because only its own
  tests read a code back. `check_recoveries` finds a printed code's
  component from the text that the spelling writes before the area of a
  probe code of each declared component, so it also works for a custom
  spelling that writes the component and a separator first.
- `schemas::stream_schema`, `Leaf::output_line`, `Guide::get_documents`,
  `Effects::is_idempotent` and `Effects::is_open_world` are no longer
  public. A `Guide` or `Leaf` getter stays public only while a sibling
  crate reads it, so `Leaf::get_choices`, `Guide::get_topics`,
  `Guide::get_picker` and `Guide::get_personal_aliases` stay, because
  crumbkit-test reads them.
- The members of the sealed `Rows` trait are hidden from the
  documentation. A tool names the trait only in a bound.
- The `crumbkit::form` module is removed. Its `ReadFailure` and `Step`
  move into crumbkit-test as private types, because only crumbkit-test
  reads a failure back.
- In crumbkit-test, `FailureReader`, `Examples::failure_reader`,
  `Examples::fresh_dir` and `Examples::redact` are removed, with
  `Recoveries::from_codes`, `Twins::from_codes` and
  `Unprovoked::from_code`. The checks read a failed run from crumbkit's
  failure document or its `code:` line. A run starts in the directory of
  `Examples::current_dir` or the test's directory, and a tool whose example
  prints a value that changes on every run opts the example out with
  `Allow::example`.
- `Allow::untwinned` stays, because it is the only `const` constructor of
  an opt-out from `Rule::NoTwin`. `Pages::clap` stays, because it is the
  only page source for a tool that adopts the checks without a guide.
  `RuntimeFailing` and `RuntimePlace` stay for an adopter that builds its
  failing command lines from data it reads at run time.

## Consequences

A test that names a removed item fails to compile, and a compile_fail
doctest in crumbkit's and crumbkit-test's `removed.rs` keeps each removal
in place. `check_recoveries` takes no component text from a custom
spelling that writes the area first, writes the reason before the area,
changes the area's letters, leaves out the component's name, or ends the
text before the area in a lowercase letter, a digit or `_`, such as
`shelf_`, which would also begin a code of `shelf_cache`. For such a
spelling the check counts each undeclared code of that component as one it
does not follow. The checks no longer copy a fixture for each run, so an
example or a recovery chain that writes a file writes it into the
directory of `Examples::current_dir` itself, and the next run sees it. A
removed item can return through a new record when a second use appears
(API-1).

Considered: deprecating the items for one release, which no adopter needs
before 0.1.0; hiding them with `#[doc(hidden)]`, which keeps their cost in
code and tests; keeping `Spelling::DOTS` as a second built-in spelling,
which no tool used.
