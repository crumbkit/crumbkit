# ADR-0215. A process capture is built with constructors

- Date: 2026-11-13
- Status: accepted
- Supersedes: ADR-0188, which it amends
- Missed: ADR-0188 added `Capture::Inherit` to an enum whose `Whole` and `Lines` variants had public named fields, so a new setting of either kind would still stop every caller that wrote that variant from compiling.

## Context

crumbkit-process's `Capture` was a `#[non_exhaustive]` enum with the
variants `Discard`, `Inherit`, `Whole { limit, over }`, `Tail { limit }`
and `Lines { queue, line, total, over, long, end }`. `#[non_exhaustive]`
lets the enum gain a variant, but a new setting of `Whole` or `Lines` adds
a field, and every caller that builds that variant with a struct literal
then fails to compile. The API promises say that configuration types have
private fields and builders, and `Capture` broke that promise. Every caller
of `Lines` also had to name all six fields, although most callers change
only the limits.

## Decision

- `Capture` is a struct that holds a private enum of its kinds. It keeps
  `Clone`, `Debug` and `Default`, and its default is the discard.
- Each kind has a `const fn` constructor: `Capture::discard()`,
  `Capture::inherit()` and `Capture::tail(limit)` return a `Capture`.
  `Capture::whole(limit)` returns a `WholeCapture`, and
  `Capture::lines(queue, total)` returns a `LineCapture`.
- `WholeCapture::over` sets what the run does over the limit, and the
  default stops the child. `LineCapture::line`, `long`, `over` and `end`
  set the limit of one line, what a long line does, what the run does over
  a limit and the byte that ends a line. Without them, one line has no limit
  of its own, a long line takes the stream over its limit, the run stops the
  child, and a line ends at a newline.
- `WholeCapture` and `LineCapture` have private fields and convert into a
  `Capture`. `RunOptions::stdout` and `RunOptions::stderr` take
  `impl Into<Capture>`, so a caller passes a builder as it is.
- `Capture` has no getters, because no caller reads a capture back.

## Consequences

Every caller that builds a capture changes, and a `match` on `Capture`
outside the crate no longer compiles. `Capture::Whole { limit, over:
OverLimit::Stop }` becomes `Capture::whole(limit)`, and a `Lines` capture
with the default policies becomes `Capture::lines(queue, total).line(line)`.
A setting that a kind does not read cannot be written, because only that
kind's builder has the method. A new setting of a kind is a new builder
method, which breaks no caller. What each kind keeps, how lines split and
how limits stop the child do not change.

Considered: one `Capture` struct whose builder methods have no effect on
the kinds that do not read them; keeping the enum with a payload struct of
private fields in each variant, which makes every call name both the
variant and the payload type; and `Capture::lines(queue, line, total)`
with both limits required, which puts two `usize` arguments side by side.
