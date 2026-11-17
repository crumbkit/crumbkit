# ADR-0235. Recoveries and Twins take one flat code list

- Date: 2026-11-16
- Status: accepted
- Supersedes: ADR-0217, which it amends
- Missed: ADR-0217 made a list of lists the only argument of `Recoveries::new` and `Twins::new` and missed that a `const fn` takes a flat slice from a `Vec` through deref coercion, so a test with run-time codes needed an extra `let lists` line.

## Context

ADR-0217 gave `Recoveries::new` and `Twins::new` the argument
`&'a [&'a [Code<'a>]]`, one list for each of the tool's error types. A test
whose codes come from a function holds a `Vec<Code>` and had to write
`let lists = [codes.as_slice()];` and pass `&lists`, because rustc drops a
temporary array at the end of the statement (E0716). Every caller in the
workspace passes exactly one list, so each one also wrote `&[shelf::CODES]`
where `shelf::CODES` would do.

## Decision

- `Recoveries::new(examples, codes)` and `Twins::new(examples, codes)` take
  one list, `codes: &'a [Code<'a>]`. The list can be a derived `CODES`, a
  literal `&[A, B]`, or `&codes` of a `Vec<Code>` that a test built at run
  time. Both stay `const fn`.
- `Recoveries::lists(examples, lists)` and `Twins::lists(examples, lists)`
  take `&'a [&'a [Code<'a>]]`, one list for each error type, because a
  `const` cannot join slices.
- Both types hold a private `Copy` enum of one list or many lists, which the
  checks read through one function. A code that two lists share counts once
  in the twin check, as it already did in the recovery check.
- `impl AsRef<[Code]>` is not used, because it is not allowed in a `const fn`
  on the 1.89 MSRV and the example tools build these settings in a `const`.

## Consequences

A caller that passes `&[x::CODES]` to `new` gets a compile error E0308 and
writes `x::CODES`, or calls `lists`. A test with run-time codes drops its
`let lists` line. Printed failures, help pages, the twin problems and
`recoveries.txt` do not change.
