# ADR-0071. One envelope choice

- Date: 2026-10-17
- Status: accepted
- Supersedes: ADR-0066, and ADR-0059 and ADR-0016, which it amends
- Missed: ADR-0059 and ADR-0066 added options that only one tool's output needed, without asking whether a second tool would use them, and ADR-0059 spread one choice of envelope over three builders whose combinations needed panics to rule out.

## Context

A document chose its envelope through three builders: `without_envelope`,
`with_keys` and `failures_in_order`. Some combinations of them were
invalid, so each builder panicked on the others. ADR-0059 also added
`with_type` and `summary_at_top`, and ADR-0066 added `without_total`. Each
of those three options reproduced bytes that one tool's readers already
parsed, and no example used them. API-1 now asks for a second use before
crumbkit adds such an option (ADR-0074).

A failure form could also ask whether a failure was among several keys
through `PreparedFailure::among_several`. Only the same tool needed it, and
a form can count the keys from the arguments it already receives.

## Decision

- `Document::envelope(Envelope)` chooses the envelope. `Envelope` is a
  public enum with the variants `Crumbkit`, `Keys { type_key, data_key }`,
  `KeysInOrder { type_key, data_key }`, `Without` and `WithoutInOrder`, and
  it is `#[non_exhaustive]`. A new document has `Envelope::Crumbkit`.
  `envelope` panics on a declared key that is not one non-empty line, on
  two keys that are the same, and on a key that crumbkit writes beside the
  result.
- The two in-order variants print a batch's failures among its rows in
  input order, as `failures_in_order` did. They apply to a batch alone, and
  crumbkit-test's `check_guide` fails a leaf whose document is not a batch
  and has one of them.
- `with_type`, `summary_at_top`, `without_total`, the `documents/total`
  opt-out and `PreparedFailure::among_several` are removed. The type key
  holds the document's id, a listing's summary prints under `summary`, and
  a listing's JSON holds `total` whenever the tool counted the rows. A tool
  whose readers need other bytes adds them in its own glue.
- The getters that only crumbkit and crumbkit-test read are
  `#[doc(hidden)]`: `get_envelope`, `always_prints_first`, `empty_note`,
  `is_headed`, `answers_left_out_per_key` and `headed_bytes`. A hidden item
  is outside the semver promise, so a later release may change it in a
  minor version.

## Consequences

A document states its envelope in one call, and the compiler rejects a
combination that the panics used to reject. A tool that matches on
`Envelope` needs a wildcard arm, because a later release may add a
variant. A tool that used one of the removed options prints those bytes
itself, or prints the document without the envelope and opts out of the
envelope rule. `schemas list` still shows a document with declared keys
with the kind `own-envelope`.

Considered: keeping the builders and adding the enum beside them, an
`Envelope` with a separate `in_order` flag, a const panic for an in-order
envelope on a document that is not a batch, which a generic `impl` cannot
express, and keeping `among_several` as a declared seam.
