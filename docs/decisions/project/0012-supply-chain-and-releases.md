# ADR-0012. Supply chain and releases

- cargo-deny's licence, ban and source checks and cargo-shear run in the pull-request `lint` job, because they take seconds and stop a bad dependency before it merges. `deny.toml` allows only the licences the graph uses and only crates.io as a source, and it bans duplicate versions, wildcard version requirements and the async runtimes and C build tools that ADR-0003 rules out. It also fails on an executable or a script inside a crate that runs at compile time, except the helper scripts that `deny.toml` names with the reason no build runs them.
- cargo-deny's advisories check runs in the nightly lane of ADR-0011, because a new advisory can appear when no code has changed.
- Dependabot checks the cargo and GitHub Actions dependencies weekly and puts each ecosystem's version updates in one pull request. It proposes a release only after a 7-day cooldown, so the ecosystem has time to find and yank a compromised release.
- The three published crates, crumbkit, crumbkit-test and crumbkit-aliases, share one version, one root `CHANGELOG.md` and one tag. release-plz publishes them through crates.io trusted publishing.

Considered: cargo-audit, cargo-udeps, cargo-machete, advisories on every pull request, one pull request per update, Renovate, cargo-release.
