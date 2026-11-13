# ADR-0217. A code list holds its codes by value

- Date: 2026-11-13
- Status: accepted
- Supersedes: ADR-0205, which it amends
- Missed: ADR-0205 kept `Exit::failures` and `CODES` lists as references so that the lists an author writes would not change, although 0.5.0 already makes every author rewrite each code constant and each `fn code`, and the reference lists made crumbkit-test keep a second constructor for codes held by value.

## Context

ADR-0205 made `Code` a `Copy` value and had `Failure::code` return it by
value, but left every list of codes as `&'static [&'static Code<'static>]`:
`Exit::failures` and `Exit::get_failures`, the `CODES` list that
`#[derive(Failure)]` writes, and the `codes::ALL` lists of crumbkit,
crumbkit-child and crumbkit-aliases. A run-time code from `Code::runtime` is
held by value, so crumbkit-test kept two constructors for each setting:
`Unprovoked::new` took `&Code` and `Unprovoked::from_code` took `Code`, and
`Recoveries::new` and `Twins::new` took lists of references while
`Recoveries::from_codes` and `Twins::from_codes` took one list of values.

## Decision

- Every crumbkit signature takes and returns a code by value, singly or in a
  list. This includes `Observer::failed`, `Spelling::spell` and
  `ToolName::code`, which took a reference until the change before this one.
- A code list is `&'static [Code<'static>]`: `Exit::failures`,
  `Exit::get_failures`, a derived `CODES` and each `codes::ALL`. A tool writes
  `.failures(&[LOCKED])`, and `&[A, B]` of constants promotes to a static
  slice.
- crumbkit-test has one constructor for each setting. `Failing::new` and
  `Unprovoked::new` take a `Code<'a>`. `Recoveries::new` and `Twins::new`
  take `&'a [&'a [Code<'a>]]`, one list for each of the tool's error types,
  because a `const` cannot join the `CODES` lists of several error types.
  `Unprovoked::from_code`, `Recoveries::from_codes` and `Twins::from_codes`
  are removed.
- A test whose codes come from `Code::runtime` writes
  `let lists = [codes.as_slice()];` and passes `&lists` to `new`.

## Consequences

A tool drops the `&` before each code it lists on an exit and before the code
it passes to `Failing::new` or `Unprovoked::new`. A derived `CODES` changes
type with no source change, so only a comparison such as
`assert_eq!(E::CODES, &[&A])` changes. A test with run-time codes writes one
more line. Printed failures, help pages, schemas and the twin and recovery
reports do not change.

Considered: one flat list for `Recoveries::new`, which forces a tool with
several error types to join its `CODES` lists by hand; keeping `from_codes`
beside `new`, which keeps two constructors for one setting; and keeping the
reference lists, as ADR-0205 decided.
