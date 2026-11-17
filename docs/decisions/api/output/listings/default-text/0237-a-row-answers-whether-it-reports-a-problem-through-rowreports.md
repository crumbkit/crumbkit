# ADR-0237. A row answers whether it reports a problem through RowReports

- Date: 2026-11-17
- Status: accepted
- Amends: ADR-0212

## Context

ADR-0212 removed `Row::is_problem` and left a caller to write
`R::PROBLEM.is_some_and(|problem| problem.reports(row))`. crumbkit-skills,
cargo-crumbkit and crumbkit's own tests repeat that expression five times,
and an adopter has asked for a one-call form, which ADR-0212 deferred until
one did.

## Decision

- crumbkit adds the public trait `RowReports: Row` with the method
  `reports_problem(&self) -> bool`, and a blanket impl for every
  `R: Row + ?Sized` that answers as `R::PROBLEM` decides.
- The blanket impl means no row type can implement the trait itself, so the
  method and `Row::PROBLEM` cannot disagree. A row type without `PROBLEM`
  answers false.
- The trait is re-exported as `crumbkit::RowReports`. `row::problems::reports`
  is removed, and crumbkit's own code calls `reports_problem`.

## Consequences

A caller imports `RowReports` to call the method. A tool whose row type has
an inherent method of the same name keeps that method silently ahead of the
trait's.

Considered: a provided method on `Row`, which a row type could override so
that crumbkit's exit and the method disagree.
