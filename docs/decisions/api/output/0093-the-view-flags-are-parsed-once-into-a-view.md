# ADR-0093. The view flags are parsed once into a View

- Date: 2026-10-19
- Status: accepted
- Supersedes: ADR-0076, which it amends
- Missed: `Output` carried the four flags as independent bools with the pairing checked at three sites after construction, so an `Output` could hold a combination that every consumer then had to refuse again.

## Context

`Output` held `json`, `fields`, `long` and `full` as four fields, and
clippy's `struct_excessive_bools` lint was silenced on it, on `Prepared`,
on `Streaming` and on the `Typed` copy that the checks read. Three pairs
of the flags cannot work together: `--fields` with `--long`, `--fields`
with `--full`, and `--full` without `--json`. `parse` refused them before
the handler ran, and `finish`, `stream` and a failed run each refused them
again, because an `Output` built by `Output::new` had never been checked.
Every printer then read the four fields and derived the same answers: which
columns the table shows, whether a JSON row holds every field, and whether
a row prints its own line.

## Decision

- A crate-private `View` holds what the four flags select: `Text(Table)`
  with `Compact`, `Long` or `Named(Vec<String>)` columns, or
  `Json(Selection)` with `Columns`, `Long`, `Full` or `Named(Vec<String>)`
  fields. `View::try_from(ViewFlags)` refuses the three pairs in the order
  that `parse` refused them, with the same text, and otherwise builds the
  one view that the flags select. With `--json`, `--long` and `--full`
  together, `--full` wins, as it did in the column selection.
- `Output` holds a `View` in place of the four fields. `parse` builds it
  after the unoffered and headed checks and before the unknown-field
  check, so the refusal order stays unoffered, headed, pairing, unknown
  field. `Prepared`, `Streaming`, the column selection and the text-line
  choice read the view.
- `Output::new` returns `Result<Output, Stop>`. On a refused pair it prints
  the refusal on standard error in crumbkit's format and returns the
  `Stop` of a refused command line, as `parse` does. `Output::new_to`
  prints the refusal to a sink. A tool that parses with clap alone now
  gets the pairing refusal from `Output::new`, before its handler runs,
  and no longer from `finish`, `stream` or a failed run. Such a tool that
  passed `--full` alone to a result whose JSON already holds every field
  got the unoffered refusal from `finish` before, and now gets the pairing
  refusal from `Output::new`, because `Output::new` has no document to
  check against.
- `View`, `Table` and `Selection` are `pub(crate)`, so none carries
  `#[non_exhaustive]`, which ADR-0002 asks of public enums only. A public
  `View` cannot name the crate-private refusal as the error of its
  `TryFrom`, and no tool outside the crate reads the view, so ADR-0085
  leaves it private until a tool needs it. A public `View` with a getter on
  `Output` comes through a new record when a second user asks for it.
- The lint exceptions on `Output`, `Prepared`, `Streaming` and `Typed` go,
  and `Typed` goes with them.

## Consequences

`docs/behaviour/output/README.md` describes the three checks with
`Output::new` refusing the pairs, and `docs/behaviour/parsing/README.md`
describes the `Stop` that `Output::new` returns. The rustdoc of `Output`,
`ViewFlags`, `Output::new`, `finish_to` and `stream` says where each
refusal comes from. The changelog records the changed signature.

Considered: a public `View` with `#[non_exhaustive]` and a public error
type for its `TryFrom`; keeping the four bools and refusing the pairs
only in `Output::new`; a `View` that also carries the unoffered and
unknown-field checks, which need the document that `Output::new` lacks.
