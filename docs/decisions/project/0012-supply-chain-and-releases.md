# ADR-0012. Supply chain and releases

- Date: 2026-10-07
- Status: accepted

## Context

An unexpected licence binds adopters, an unknown source can serve code
that nobody reviewed, and an unused dependency adds build time for
nothing. crumbkit also publishes several crates that depend on each other,
and it needed a release process that a reviewer can check and that stores
no long-lived publishing token.

## Decision

- cargo-deny's licence, ban and source checks and cargo-shear ran in the
  pull-request `lint` job, because they take seconds and stop a bad
  dependency before it merges. `deny.toml` also kept clap out of
  crumbkit-core and rejected scripts that run at compile time.
- The advisories check ran nightly, because a new advisory can appear when
  no code has changed.
- Dependabot grouped each ecosystem's weekly updates into one pull request
  and waited 7 days after a release, so the ecosystem has time to find and
  yank a compromised release.
- The published crates shared one version, one root `CHANGELOG.md` and one
  tag, because they re-export or depend on each other and so release
  together.
- release-plz opened a release pull request on each push to `main` and
  published when it merged, so a release is one reviewed pull request. The
  changelog entries stayed hand-written, because each links to its item,
  which an entry generated from a commit message would not.
- A GitHub App token opened the release pull request, because GitHub runs
  no workflow on a pull request that a job's own token opens, and the
  `gate` check is required.
- crates.io trusted publishing replaced a stored token. Because trusted
  publishing cannot publish a first version, the owner publishes 0.1.0 by
  hand, and the release jobs run only once the repository variable
  `RELEASE_ENABLED` is `true`.

## Consequences

No crates.io token exists to leak. `docs/project/releases.md` describes
the current release steps.

Considered: cargo-audit, cargo-udeps, cargo-machete, advisories on every pull request, one pull request per update, Renovate, cargo-release, a stored crates.io token, changelog entries generated from commits, a personal access token or the job's own token for the release pull request, a tag per crate, `release = false` in `release-plz.toml` as the guard before 0.1.0 (release-plz fails a run in which no crate can be released).
