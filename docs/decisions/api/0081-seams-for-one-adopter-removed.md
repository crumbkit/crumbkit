# ADR-0081. Seams for one adopter removed

- Date: 2026-10-17
- Status: amended by ADR-0084
- Supersedes: ADR-0059, ADR-0061 and ADR-0067, and ADR-0045, ADR-0068, ADR-0069, ADR-0070, ADR-0071 and ADR-0073, which it amends
- Missed: ADR-0045, ADR-0059, ADR-0061, ADR-0067 and ADR-0068 each added a seam that reproduced the output of one adopting tool, and none of them had a second use (API-1). ADR-0045 gave failure forms to "some tools", but only the one adopting tool needed a form, and no example set one. ADR-0070, ADR-0071 and ADR-0073 reshaped those seams without asking whether that tool still needed them.

## Context

Several seams let one adopting tool keep the bytes that its readers already
parsed: a document's own envelope keys and its failures in input order, a
failure form, failure field names from another system, sorted JSON keys, and
one over-budget failure for each answer left out. The adopter has since
moved to crumbkit's own shapes, so no adopting tool uses these seams. The
reading example called `Document::left_out_per_key`, but that call came in
commit 7dc15dcd with the option itself and copied the adopter's shape, so it
was not a second use under API-1. Each seam still cost code in every writer
of JSON, branches in crumbkit-test's checks, schema forms and customisation
entries.

## Decision

- `Envelope` has the variants `Crumbkit` and `Without`. `Envelope::Keys`,
  `Envelope::KeysInOrder` and `Envelope::WithoutInOrder` are removed, with
  the `own-envelope` kind of `schemas list`, the `envelope-keys` and
  `failures-in-order` customisations and the checks that read declared
  keys.
- `FailureForm`, `Guide::failure_form` and the `failure-form`
  customisation are removed. `PreparedFailure` and the label and excerpt
  types it exposed are no longer public. `crumbkit::form` holds
  `ReadFailure` and `Step`, which crumbkit-test's `FailureReader` returns.
  A tool with its own failure shape keeps `FailureReader`.
- `Guide::field_names` and the `field-names` customisation are removed, so
  every failure field name follows the rule of a code segment again.
- `JsonStyle` and sorted keys are removed. A guide sets its layout with
  `Guide::json_layout(JsonLayout)` and reads it with
  `Guide::get_json_layout`. `JsonLayout::Pretty` keeps its `reason`, as
  ADR-0073 decided.
- `Document::left_out_per_key` and the `left-out-per-key` customisation are
  removed. A batch ends with the one over-budget failure of ADR-0046.
  `Room` from ADR-0068 stays, and so does the rule of ADR-0070 for a batch
  whose first row and a failure that names no key pass the budget.
- `Guide::output_budget` accepts a budget of any size and never panics. The
  maintainer decided on 2026-10-17 that a guide has no 1024-byte floor,
  because ADR-0069 showed that the floor protects nothing:
  `<TOOL>_OUTPUT_BUDGET` already lowers a run to 1 byte, and a result that
  the budget refuses prints its over-budget failure whatever its size. A
  budget of 0 bytes also works, so the panic on 0 from ADR-0069 is removed.
  A run with it refuses or cuts every result, and a batch prints its first
  answer.
- `Headed`, `Guide::codes`, `Leaf::codes`, `help::Rename`, `Row::WHOLE` and
  `Row::LINE` stay, because they are not copies of that tool's output.

## Consequences

crumbkit prints each JSON document in one envelope and one key order, and
the failure document is the only failure shape. crumbkit-test reads a
failed run through a `FailureReader`, crumbkit's failure document or the
`code:` line. A tool that needs one of the removed shapes builds it in its
own glue until a second use appears (API-1).

Considered: keeping the seams as hidden items, which would keep their cost
in every writer; deprecating them for one release, which no adopter needs.
