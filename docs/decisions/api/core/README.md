# ADR-0099. The types that need no clap

- Date: 2026-10-19
- Status: accepted

## Decision

These records decided where the failure model, the codes, the document ids,
the calls and the row types live. Each of these types compiles without clap.
ADR-0032 put them in a crate of their own, and ADR-0097 moved them back into
crumbkit.

## Children

- [ADR-0032. The clap-free core crate](0032-core-crate.md)
- [ADR-0097. crumbkit holds the types that need no clap](0097-core-crate-merged.md)
