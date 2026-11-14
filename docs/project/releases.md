# Releases

The eleven published crates of the [repository layout](README.md#repository-layout)
share one version, one `CHANGELOG.md` and, for each release, one tag,
`v` and the version. They release together, because crumbkit re-exports
crumbkit-derive's macro and seven others depend on crumbkit.
crumbkit-process and crumbkit-durable depend on no other crumbkit crate and
share the version so the changelog and tag cover every crate.
`release-plz.toml` gives the changelog and tag to crumbkit alone.

Why: ADR-0012, ADR-0097.

## The release pull request

release-plz opens a release pull request on each push to `main` and
publishes the versions it sets when that pull request merges, so a release
is one reviewed pull request. The release pull request moves the
hand-written changelog entries under the new version's heading, because
each entry links to its item, which an entry generated from a commit
message would not.

A GitHub App's token opens the release pull request, because GitHub runs no
workflow on a pull request that a job's own token opens, and the ruleset
requires the `gate` check. The token can write only this repository's
contents and pull requests.

Why: ADR-0012, ADR-0011, ADR-0013.

## Publishing

release-plz publishes through crates.io trusted publishing. crates.io
trusts the `release.yml` workflow of `crumbkit/crumbkit` in the `crates-io`
environment, and no crates.io token is stored. Trusted publishing cannot
publish a crate's first version, so the owner publishes 0.1.0 by hand. Both
jobs of `release.yml` run only while the repository variable
`RELEASE_ENABLED` is `true`, which the owner sets after that first publish,
so a merge before then publishes nothing and fails no job.

Why: ADR-0012.

## API promises

[API promises](../behaviour/adoption/README.md#api-promises) states what a
version promises to adopters.

Why: ADR-0002.
