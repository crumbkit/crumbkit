# ADR-0088. Public items that a tool calls restored

- Date: 2026-10-18
- Status: amended by ADR-0089 and ADR-0160
- Supersedes: ADR-0085, which it amends
- Missed: ADR-0085's review counted only the callers inside this repository. A tool outside it calls `Code::runtime`, `Spelling::split`, `FailureReader`, `Examples::failure_reader`, `crumbkit::form` and `Examples::fresh_dir`, so removing them broke that tool.

## Context

ADR-0085 removed public items that no tool, example or sibling crate in
this repository called. A tool outside this repository calls some of them.
It splits a code that a plugin answers with into its segments and makes a
`&'static Code` from them. It prints its failures in a document of its own,
and its tests read the codes, fields and next steps of those failures back.
Its examples write to a fixture directory, so each run needs a new copy of
that fixture.

These items return because an adopter outside this repository calls them.
API-1 covers `Document` options, `Row` constants, `Guide` settings and
`Leaf` settings, and asks for a second adopter or a use in `examples/`.
None of the restored items is one of those, so API-1 does not decide them.
Inside this repository, the uses of `Code::runtime`, `Spelling::split`,
`FailureReader` and `crumbkit::form` are crumbkit's and crumbkit-test's own
unit tests, and `Examples::fresh_dir` has a use in `examples/reading`.

## Decision

- `Code::runtime`, `RuntimeCodeError` and `Spelling::split` return.
  `Spelling::COLONS` splits at `::`, and a spelling from
  `Spelling::custom` splits nothing, because a custom spelling may drop a
  segment or change its letters. The code spelling still lives on the
  guide, so a tool splits a code with `guide.get_spelling().split(text)`.
  A tool with a custom spelling gets `None` for every code from that call,
  so it splits the text itself before it calls `Code::runtime`.
- The `crumbkit::form` module returns with `ReadFailure` and `Step`, and
  crumbkit-test reads failures into these types again. `Step` is
  `#[non_exhaustive]`.
- In crumbkit-test, `FailureReader`, `Examples::failure_reader` and
  `Examples::fresh_dir` return. `check_examples`, `check_twin`,
  `check_twins` and `check_recoveries` read each failed run through the
  reader first. Each run of `check_examples`, `check_output_budget` and
  `Pages::binary` starts in a new copy of the fixture of `fresh_dir`, each
  line of a twin starts in its own copy, and each recovery chain starts in
  one copy. `check_examples` replaces the copy's path with `[ROOT]`.
- A run-time code still reaches neither help nor `check_codes`, because
  `RuntimeLeaf` cannot declare it, as ADR-0085's Missed line says. Such a
  failure supplies its own recovery and exits with 1.
- `check_recoveries` keeps finding a printed code's component from the
  text that the spelling writes before the area, as ADR-0085 decided. It
  does not use `Spelling::split`, so it still works for a custom spelling.
- Every other removal of ADR-0085 stays, including `Spelling::DOTS`,
  `Spelling::SLASHES`, `Examples::redact`, `Recoveries::from_codes`,
  `Twins::from_codes` and `Unprovoked::from_code`.

## Consequences

The compile_fail doctests in crumbkit's and crumbkit-test's `removed.rs`
no longer hold the restored items. crumbkit's output tests make a code
from a plugin's answer with `Spelling::split` and `Code::runtime`.
crumbkit-test's recovery tests read a tool's own failure document through
a `FailureReader` that builds a `ReadFailure`. The reading example's
examples run in a new copy of its fixture lists, and one of them appends
an entry to a list.

Considered: keeping the items removed and asking the outside tool to copy
them, which would leave every tool that prints its own failure document
without a way to run the recovery check.
