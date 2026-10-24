# ADR-0148. A length of time is a row field type

- Date: 2026-10-24
- Status: accepted
- Supersedes: ADR-0146, which it amends
- Missed: ADR-0146 gave a measured duration a text form only, so a row field that held a length of time had no JSON form and a tool wrote its own type for one.

## Context

ADR-0146 added `Age::compact`, which prints a measured duration in the
compact form of an `Age` column and has no JSON form. It rejected a public
duration type, because that type would add a second name for one text
form. A tool that reports how long a request took, how long a lock was
held or how long a job ran puts that length in a row or a failure field,
where JSON needs a form too. Without a crumbkit type, each tool writes its
own, and each picks its own units: seconds as a number, milliseconds as a
number, or text. ADR-0147 made a value type with two forms a `Display`, two
methods and one macro call, so a duration type costs crumbkit little.

## Decision

- `crumbkit::Duration` wraps a `std::time::Duration` and implements
  `Shown` through `shown!`. It derives the comparison, hashing and copy
  traits, and converts from a `std::time::Duration`.
- The text form is the compact form of ADR-0108, ADR-0141 and ADR-0146.
  `Duration`'s `Display` holds that formatter, and `Age`'s `Display` and
  `Age::compact` write through it, so one function chooses the unit.
- The JSON form is an ISO 8601 duration exact to the nanosecond: `PT`,
  whole hours with `H`, minutes from 1 to 59 with `M`, and seconds with
  `S`, leaving out each part that is zero. The seconds carry a fraction
  without trailing zeros. A zero duration is `PT0S`. Hours are the largest
  unit, so three days are `PT72H`, because ISO 8601 reads a day as a
  calendar day, which a change of daylight saving time can make 23 or 25
  hours long.
- The schema is an inline string in the `duration` format.
- `Duration` has no `Deserialize`, because crumbkit prints documents and
  does not read them back.

## Consequences

A tool puts a length of time in a row or a failure field without a type of
its own, and every tool's JSON holds it in one form that a reader parses
with an ISO 8601 library. `Age::compact` returns a `Duration` behind
`impl Display`, so it prints as before, and a later decision may remove
it in favour of `Duration`. RFC 3339's grammar for the `duration` format,
in its appendix A, has no fraction of a second, so a validator that
follows it strictly refuses a duration with a fraction, which ISO 8601
allows.

Considered: whole days with `D` above a day, such as `P3D`, which reads a
fixed 24 hours into a calendar unit; a number of seconds or milliseconds in
JSON, which needs the unit in the field's name or its description; RFC
3339's grammar without fractions, which loses the part of a duration under
a second; a `Deserialize` of the ISO 8601 form, which needs a parser for
weeks, days and fractions on any part.
