# 0012. Supply chain and releases

- cargo-deny and cargo-shear check the dependency graph, and Dependabot proposes grouped weekly updates after a 7-day cooldown.
- The three published crates share one version, one root `CHANGELOG.md` and one tag. release-plz publishes them through crates.io trusted publishing.

Considered: cargo-audit, cargo-udeps, Renovate, cargo-release.
