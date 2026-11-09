# ADR-0177. The twin check trusts only the codes that crumbkit's crates list

- Date: 2026-11-08
- Status: amended by ADR-0186
- Supersedes: ADR-0058, which it amends

## Context

`check_twins` treated every code whose component is the text `crumbkit` as
a code that crumbkit or one of its add-on crates raises (ADR-0058). Such a
code needed no twin and no declaration. A tool could write
`Code::new("crumbkit", "books", "gone")`, and the check accepted the code
although no crumbkit crate lists it. ADR-0058 recorded this as a consequence.

The recovery check decided whether a code that a run prints is crumbkit's
from two built-in lists: `crumbkit::codes::ALL` and crumbkit-test's list of
the add-on crates' codes, which a test compares with those crates' own
`codes::ALL`. It decided the same for the code of a start from the
component.

## Decision

- `check_twins` and the recovery check, for a start's code as for a printed
  code, read the two lists through one crate-private function of
  crumbkit-test.
- A code is crumbkit's only when one of the lists holds it, or when a
  `Rename` of the guide or of a leaf prints it in place of a code that one
  of the lists holds.
- Every other code is the tool's code, whatever its component. It must be
  declared, and it needs a twin, an `Unprovoked` entry or an
  `Allow::untwinned` opt-out.

## Consequences

The signature of `check_twins` does not change. A code in the `crumbkit`
component on neither list now needs a twin and a declaration, and an
`Allow::untwinned` opt-out for such a declared code is no longer stale. A
start of `check_recoveries` with such a code fails the check unless the
tool declares the code. A new add-on crate adds its codes to
`ADD_ON_CODES`. `docs/behaviour/checks/end-to-end.md` and
`docs/behaviour/checks/recovery-chains.md` describe the current behaviour.

Considered: a required `trusted` parameter of `check_twins`, which would
make every tool repeat lists that crumbkit-test already holds; a
`Twins::trusting` setter, which would add a field and a default for the same
lists; keeping the component test, which accepts a code that no crumbkit
crate lists.
