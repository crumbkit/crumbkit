# Code coverage

`check_recoveries` and `check_twins` each take one list of the tool's declared
codes. The list can be built at run time, so a tool passes the codes its
binary can print, and the check requires each of them to be covered.

A declared code is covered in `check_recoveries` when a start provokes it or
`Unprovoked` lists it with the reason no test can provoke it. It is covered in
`check_twins` when a twin refuses it, `Unprovoked` lists it, or an
`Allow::untwinned` opt-out names it. A tool passes the same `Unprovoked` list
to both checks.

A printed code of a declared component that the list lacks still fails the
recovery check, so the list must hold every code the binary prints.

## Listing an area

`Unprovoked::area(component, area, reason)` lists every declared code of one
area, such as `("tool", "api")`, under one reason. A tool that forwards the
codes of a library crate lists the area of the codes its binary never prints
with one entry in place of one line for each code. A code added to the area
later is excused too.

A check fails on an area entry in these cases:

- It covers no declared code, which also catches a misspelt area.
- A start provokes, or a twin refuses, every declared code it covers, so the
  entry is stale.
- A code entry for a code in the area repeats it.

An `Allow::untwinned` opt-out for a code in a listed area is stale, as it is
for a listed code.

The recovery report prints the entry as `tool::api::*: reason` under "codes
that no start provokes:", with each declared code it excuses on its own line
below it. A code added to the area shows in the report's golden file, where
review sees it.

Why: ADR-0053, ADR-0177, ADR-0236.
