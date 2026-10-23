# ADR-0146. A measured duration prints as a compact age

- Date: 2026-10-23
- Status: accepted
- Supersedes: ADR-0108 and ADR-0141, which it amends
- Missed: ADR-0141 and ADR-0108 gave the compact form only to a time the tool knows, so a tool that measured a duration had to invent a time to print it.

## Context

ADR-0108 gave `Age` a compact text form in the largest whole unit, and
ADR-0141 added weeks from seven days on. `Age::since` takes the time that
the age counts from and the current time. An adopting tool waits on a lock,
measures the wait with a monotonic `Instant`, and prints a note such as
`It finished after 3s of waiting.` The tool has only a `Duration`, so it
reads the system clock and subtracts the duration to build an `Age` whose
time it invented, and then prints the same duration. A tool that prints how
long something took needs the same text as a tool that prints how long ago
something happened.

## Decision

- `Age::compact(elapsed)` returns a value whose `Display` writes the
  duration as a compact age, in the units and with the boundaries that
  ADR-0108 and ADR-0141 give.
- `Age`'s `Display` writes through the same code, so one function chooses
  the unit for both.
- The returned value has no JSON form and no time. Its type is private, and
  the function returns `impl Display`, so the only public addition is the
  function.

## Consequences

A tool prints a measured duration in the same form as an `Age` column,
without reading the clock or inventing a time. A change to the units or
their boundaries changes both forms together. A row field that JSON should
hold as a time is still an `Age`, because a duration alone has no RFC 3339
form.

Considered: a separate public type for a duration, which adds a second name
for one form; a free function such as `crumbkit::compact_age`, which puts
the form away from `Age`, where a reader of `Age`'s rustdoc looks for it;
`Age::from_duration`, which builds an `Age` whose time is invented and whose
JSON would print that time; and `impl From<Duration> for Age`, which has the
same invented time and hides the conversion in an `into()`.
