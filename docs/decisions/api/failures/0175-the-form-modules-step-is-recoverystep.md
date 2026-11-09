# ADR-0175. The form module's step is RecoveryStep

- Date: 2026-11-08
- Status: amended by ADR-0183
- Supersedes: ADR-0088, which it amends

## Context

ADR-0088 restored `crumbkit::form` with `ReadFailure` and `Step`. The crate
also has the listing's `crumbkit::Step`, so one name stood for two concepts.
A tool that adds next steps to listings and reads failures in its tests had
to alias one of the two imports.

## Decision

`crumbkit::form::Step` is `crumbkit::form::RecoveryStep`, with the same
variants. The listing's `crumbkit::Step` keeps its name, because tools build
it in handlers.

## Consequences

A tool that names `form::Step` changes the name at 0.2. ADR-0002 allows a
breaking change in a 0.x minor release.

Considered: keeping the name and aliasing it in crumbkit-test, which would
leave both types with one name in the public item list.
