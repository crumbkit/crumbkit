# ADR-0180. Tool and its records replace Guide

- Date: 2026-11-08
- Status: amended by ADR-0184
- Supersedes: ADR-0036, ADR-0056 and ADR-0086, which it amends

## Context

`crumbkit::help::Guide` had 17 fields and lived in `help`, while most of its
fields were not about help. They set the spelling and renames of failure
codes, the refusal functions, the output budget, the JSON layout, terminal
detection, the picker and dashed forms. The docs called these settings
declarations, and the opt-out report called them customisations.

## Decision

`Guide` is `crumbkit::Tool`, at the crate root. `Tool` composes three records,
each `const`-constructible and defaulting to crumbkit's behaviour:

- `help::Help` holds groups, leaves, topics, file formats, the root, root
  verbs and personal aliases.
- `tool::Failures` holds the spelling, renames, exits, refusal alternatives
  and refusal hint.
- `tool::Output` holds the budget, the JSON layout and terminal detection.

`Tool` itself holds the picker and dashed forms, which form no group. A
getter is on the record that owns its field. `Tool::printed`,
`Tool::failure_exit`, `Tool::tool_name` and `Tool::with_runtime_leaves` stay
on `Tool`, because each reads more than one record.

The record of what a run prints has the path `crumbkit::tool::Output`,
because `crumbkit::Output` is the handle of one run. `Failures` shares its
module so a tool imports both records from one path.

`Exit`, `Rename` and `RefusalContext` move from `crumbkit::help` to
`crumbkit::tool`, so a tool imports the failure declarations from one path.

The opt-out report says "declarations" where it said "customisations". Its
JSON array is `declarations`, and because that renames a field the report's
id is `opt-outs/v2`.

The word "guide" leaves crumbkit-test's public names: `check_guide` is
`check_help`, `Rule::GuideExamples` and `Rule::GuideDocument` are
`Rule::HelpExamples` and `Rule::HelpDocument`, and the opt-out ids are
`help/examples` and `help/document`. Dated decision records keep the name
`Guide` (ADR-0017).

## Consequences

A tool sets only the records it needs, and a new tool-wide setting joins one
small record. A tool that named `Guide` changes its imports and its builder
chain at 0.2. ADR-0002 allows a breaking change in a 0.x minor release.

Considered: one flat record renamed and moved, which keeps the 17 fields
together, and a fourth `Parsing` record for the picker and dashed forms,
which would hold two settings.
