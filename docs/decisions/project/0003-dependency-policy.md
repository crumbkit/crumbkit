# ADR-0003. Dependencies and MSRV

- Date: 2026-10-07
- Status: accepted

## Context

crumbkit is a library that every adopting tool builds into its binary, so
each dependency that crumbkit takes adds build time, binary size and
supply-chain risk to every adopter. crumbkit also needed a minimum
supported Rust version and a statement of the platforms it supports.

## Decision

- crumbkit admitted a dependency only when it had active maintainers,
  judged case by case, a compatible `rust-version` and licence, a measured
  weight, and no async runtime or C build. `Cargo.toml` gave each
  dependency, feature and dev-dependency a reason and set
  `default-features = false`, so that a binary, and never a published
  crate, chose features such as clap's colour. gungraun was the exception,
  because it exports its benchmark API only under its default feature.
- The default build kept to clap, serde, serde_json, schemars and
  is-ai-agent. crumbkit enabled clap's `help` feature, because crumbkit's
  refusals tell the caller to run `-h`, and clap's `error-context`
  feature, because a refusal reads the refused argument, value and
  subcommand from clap's error. clap's `suggestions` feature already turns
  `error-context` on.
- Everything else sat behind a feature or in a separate crate, so a tool
  that does not use it builds nothing for it: the usage log, miette,
  `hand-over`, `tracing`, `complete` and `derive` were features, and
  crumbkit-aliases, crumbkit-picker and cargo-crumbkit were separate
  crates. The usage log later moved to the separate crumbkit-usage crate
  (ADR-0029). miette came with no features, because crumbkit draws the source
  line itself. crumbkit-derive used syn 3, which clap_derive already brings
  into the lock file with the same features. Each binary-size cost was
  measured on shelf's stripped release build when the feature was added.
- crumbkit raises the MSRV only when a change needs it, and never above
  the newest stable release that is at least six months old, so adopters
  on an older toolchain are not cut off by an unrelated change.
- Linux and macOS were tier 1 and Windows on `x86_64-pc-windows-msvc` was
  tier 2. A Windows failure does not block a release, because Windows has
  no job in the pull-request lane. The usage log refuses to build on a
  target that is neither Unix nor Windows, because its file and signal
  handling exist only for those two.

## Consequences

- An adopter's default build stays small, and each optional part names
  what it costs in `Cargo.toml`.
- Windows has limits that come from crumbkit's ban on unsafe code: POSIX
  quoting in `help::quote`, the inherited access control list of the usage
  log, and console events that end a tool without a usage line.
- `docs/project/dependencies.md` lists the current dependencies, features
  and measured sizes.

Considered: tabled, comfy-table, ctrlc, dirs, blake3, jiff, hmac with sha2 and getrandom, the usage log in the default build, is-ai-agent behind a feature, crossterm, inquire or dialoguer in crumbkit-picker, miette's `derive` and `fancy` features, tracing-subscriber for the span tests, Windows at tier 1 with a job in the pull-request lane, the GNU Windows target, `windows-sys` for a console control handler, `ctrlc` for Windows Ctrl-C, completion scripts generated at build time with clap_complete's stable API, syn 2 for crumbkit-derive.
