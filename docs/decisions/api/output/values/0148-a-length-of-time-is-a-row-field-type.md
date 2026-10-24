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
- The text form uses the unit spellings of the compact age of ADR-0108,
  ADR-0141 and ADR-0146, from one table that `Age` and `Duration` share.
  Under one second it is the largest of milliseconds, microseconds and
  nanoseconds with a whole count, such as `300ms`, `999µs` or `500ns`, and
  zero reads `0s`. From one second to under a minute it is seconds with
  one decimal place and no trailing `.0`, such as `1s`, `1.9s` or `59.9s`,
  rounded down so that it never reads `60s`. From one minute up it is the
  largest unit with a whole count, then the next smaller unit when its
  count is not zero: `2m`, `4m 10s`, `1h`, `1h 5m`, `3d 2h`. A unit two
  places down never prints, so 1 hour and 5 seconds reads `1h`, and each
  count rounds down. A length of time, such as a request's latency, is
  read for its size, so a 300 millisecond request that read `0s`, or a
  1.9 second one that read `1s`, would lose much of the value. An age is
  read for its recency, so `Age` keeps its single unit.
- The JSON form is an ISO 8601 duration exact to the nanosecond: `PT`,
  whole hours with `H`, minutes with `M`, and seconds with `S`, leaving
  out each part that is zero, except that minutes always stand between
  hours and seconds, so 1 hour and 5 seconds is `PT1H0M5S`. The seconds
  carry a fraction without trailing zeros. A zero duration is `PT0S`. Hours are the largest
  unit, so three days are `PT72H`, because ISO 8601 reads a day as a
  calendar day, which a change of daylight saving time can make 23 or 25
  hours long.
- The schema is an inline string with a `pattern` that matches exactly
  the strings that `Duration` writes, and a description that says it is
  an ISO 8601 duration with hours as its largest unit and an optional
  decimal fraction of a second. It declares no `format`.
- `Duration` has no `Deserialize`, because crumbkit prints documents and
  does not read them back.

## Consequences

A tool puts a length of time in a row or a failure field without a type of
its own, and every tool's JSON holds it in one form that a reader parses
with an ISO 8601 library. `Age::compact` keeps the single-unit form of an
age, so it prints as before, and a later decision may remove it in favour
of `Duration`. A consumer that checks the schema's pattern accepts every
duration, including those under a second.

Considered: the single-unit form of an age as the text form, which reads a
300 millisecond request as `0s`; whole seconds under a minute, which reads
1.9 seconds as `1s`; the two largest units that are not zero, which reads 1
day and 30 seconds as `1d 30s`, so the precision of a cell no longer
follows its size; `0ms` under one millisecond, which loses a microsecond
latency the same way; whole days with `D` above a day, such as `P3D`, which
reads a fixed 24 hours into a calendar unit; a number of seconds or
milliseconds in JSON, which needs the unit in the field's name or its
description; the JSON Schema format `duration`, which follows the grammar
in appendix A of RFC 3339, so a validator that checks formats, such as the
jsonschema crate, refuses `PT0.3S`, `PT1H0.5S` and `PT1H5S`, and every
latency under a second would fail it; RFC 3339's grammar itself, which has
no fraction of a second and so loses the part of a duration under a second;
a `Deserialize` of the ISO 8601 form, which needs a parser for weeks, days
and fractions on any part.
