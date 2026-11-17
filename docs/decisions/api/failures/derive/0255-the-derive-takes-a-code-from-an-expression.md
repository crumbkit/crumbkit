# ADR-0255. The derive takes a code from an expression and forwards a foreign error through a newtype

- Date: 2026-11-17
- Status: accepted
- Amends: ADR-0001

## Context

ADR-0001 made `code` a path to a constant. A variant that chooses its code
at run time, such as by `io::ErrorKind`, or that returns a code held in a
`CodeBuf` field, could not derive, so two adopters and one other tool kept
hand-written `Failure` impls of up to 379 lines. Part of that size comes
from variants that wrap an error from a library with no crumbkit dependency.
`#[failure(forward)]` needs the wrapped type to implement `Failure`, and an
adopter cannot implement it for a foreign type.

## Decision

- `code = EXPR` takes any expression of type `Code<'_>`. It reads the
  variant's fields by name, and the generated `code()` matches the variant
  with every field bound, so the expression can borrow from a field.
- `codes = [PATH, ...]` names the codes that a non-path `code` returns. `CODES`
  lists them in the written order, each once, with the paths of the other
  variants. `codes = []` states that the variant's codes are in no declared
  list.
- A `code` that is not a path without `codes` is a compile error that names
  the variant. `codes` beside a `code` that is a path, or beside `forward`,
  is a compile error too.
- A foreign error is forwarded through a newtype that implements `Failure`
  and is named in a variant with the existing `#[failure(forward)]`. The
  derive gains no `forward_with` key, crumbkit gains no `FailureOf` trait, and
  there is no derive for the newtype.

## Consequences

Every type that uses a path `code` expands as before. The newtype's `Failure`
impl takes the trait's defaults and every method that `Failure` gains later,
so there is no mirror trait to keep in step, and a test can call the newtype
on its own. The adopter writes the newtype with `Display`, `Error` and `From`,
and a pattern on the variant has one more level.
A `codes` list that misses a code the expression returns compiles, and
`check_recoveries` and `check_twins` report the undeclared code when a test
provokes it.

Considered: a `forward_with = FUNCTION` key, which cannot compile for a
function that returns a borrowed `&dyn Failure` or a `Box<dyn Failure>`
(error E0515 on Rust 1.89 and 1.99.1, found by
the eval `crumbkit-forward-foreign-errors`), a `FailureOf<E>` trait that mirrors
`Failure` for a foreign type, and a serde-style derive that mirrors a foreign
enum.
