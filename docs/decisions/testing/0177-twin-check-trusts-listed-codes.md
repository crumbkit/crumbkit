# ADR-0177. The twin check trusts only listed codes

- Date: 2026-11-08
- Status: accepted
- Supersedes: ADR-0058, which it amends

## Context

`check_twins` treated every code whose component is the text `crumbkit` as
a code that crumbkit or one of its add-on crates raises (ADR-0058). Such a
code needed no twin and no declaration. A tool could write
`Code::new("crumbkit", "books", "gone")`, and the check accepted the code
although no library lists it. ADR-0058 recorded this as a consequence.

The recovery check already reads explicit lists of codes. Each library
publishes such a list as `codes::ALL`.

## Decision

- `check_twins` takes a fourth parameter, `trusted: &[&[&Code]]`, the lists
  of library codes, such as
  `&[crumbkit::codes::ALL, crumbkit_child::codes::ALL]`.
- A code is a library's code only when one of the lists holds it, or when a
  `Rename` of the guide or of a leaf prints it in place of a code that one
  of the lists holds.
- Every other code is the tool's code, whatever its component. It must be
  declared, and it needs a twin, an `Unprovoked` entry or an
  `Allow::untwinned` opt-out.

## Consequences

Every caller of `check_twins` names the libraries it trusts, which is a
breaking change to crumbkit-test. A tool that adds a library with its own
codes adds that library's list to the call. A code in the `crumbkit`
component that no list holds now needs a twin, and an `Allow::untwinned`
opt-out for such a declared code is no longer stale.
`docs/behaviour/checks/end-to-end.md` describes the current behaviour.

Considered: a `Twins::trusting` setter, which would add a field and a
default that trusts nothing or everything; always trusting
`crumbkit::codes::ALL` and passing only the add-on lists, which would leave
the check with two sources of library codes; keeping the component test,
which accepts a code that no library lists.
