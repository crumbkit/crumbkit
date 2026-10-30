# ADR-0162. Compile-fail guards for removed items deleted

- Date: 2026-10-30
- Status: accepted
- Supersedes: ADR-0085, ADR-0088, ADR-0089 and ADR-0095, which it amends

## Context

ADR-0085 and ADR-0095 removed public items before the first release, and
kept each removal in place with a compile_fail doctest in a `removed.rs`
file. crumbkit, crumbkit-child, crumbkit-test and crumbkit-usage each had
one, and crumbkit's file also held the options that ADR-0081 removed.
ADR-0088 and ADR-0089 restored some of the items and took their doctests
out of the files.

No crate has been published, so no adopter can depend on an item that was
removed before 0.1.0. A doctest that checks an item stays absent guards
against nobody. Each file cost a module declaration, a cargo-shear
exception and doctest build time in every test run. After 0.1.0,
cargo-semver-checks compares each release with the last published one from
0.1.1 on (ADR-0002), so it reports a public item that a release removes.

This record amends ADR-0085, ADR-0088, ADR-0089 and ADR-0095, whose
Consequences say that the doctests in `removed.rs` keep each removal in
place.

## Decision

- The four `src/removed.rs` files are deleted, with their `cfg(doctest)`
  module declarations and the cargo-shear exceptions that let cargo-shear
  accept a file without items.
- The removals that those records decided stay. Each removed item can
  return only through a new record, under API-1.

## Consequences

Nothing in the repository fails when a removed item returns. A review of
the pull request that brings it back is the only check before 0.1.0, and
cargo-semver-checks reports changes to the public API from 0.1.1 on. The
records that decided each removal still list the removed items.

Considered: keeping the files until 0.1.1, which keeps the costs while no
adopter can be affected; moving the guards into ordinary tests, which
still checks only against unpublished items.
