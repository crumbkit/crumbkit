# ADR-0097. crumbkit holds the types that need no clap

- Date: 2026-10-19
- Status: accepted
- Supersedes: ADR-0032

## Context

ADR-0032 moved the failure model, the codes, the document ids, the calls and
the row types into crumbkit-core, so that a crate that is not a command-line
tool could use them without building clap. crumbkit depended on
crumbkit-core and re-exported each item at the path it had before the split.

No crate took crumbkit-core alone. ADR-0032's split anticipated a clap-free
consumer that has not appeared, and crumbkit-core removed nothing from any
adopter's dependency tree, because crumbkit depends on it. Every tool and
every crate in the workspace that used these types reached them through
crumbkit, which builds clap in every case. The split still cost a published crate with its
own README, feature list and publish step, a `derive` and a `miette` feature
on each of two crates, getters on `Document` and `Call` that were public and
hidden from the docs only so that crumbkit could read them across the crate
boundary, and a `deny.toml` ban that kept clap out of one crate.

## Decision

- crumbkit-core's modules moved into crumbkit, each at the path at which
  crumbkit re-exported its items, so every public path that a tool uses is
  unchanged. `Call::render` stays on the sealed `RenderCall` trait, and
  `Call`, `Document` and the other moved types keep their signatures.
- The `derive` feature moved to crumbkit, which re-exports
  `#[derive(Failure)]` from crumbkit-derive beside the trait of the same
  name. crumbkit's `miette` feature holds `Failure::diagnostic` and the
  conversion of a `miette::Report`, which crumbkit-core's `miette` feature
  held. crumbkit-derive's README example builds on crumbkit.
- A getter that was hidden only so that crumbkit could read it across the
  crate boundary became `pub(crate)`. A getter that crumbkit-test reads
  stays public and hidden.
- The `deny.toml` ban on clap went, because no crate stays free of clap any
  more: every published crate except the crumbkit-derive macro reaches clap
  through crumbkit.
- A crate boundary in this workspace exists only when a consumer that
  exists today needs the smaller crate, or when the boundary keeps a
  measured set of dependencies out of every adopter's tree, as
  crumbkit-child keeps signal-hook out and crumbkit-usage keeps the log's
  dependencies out. A boundary made for a consumer that may appear later is
  not made; the split can be made again when that consumer exists.
- A crate that wants the failure and document types without clap can ask
  for the split again when such a crate exists. The record of that request
  will say which types it needs and what it builds without clap.

## Consequences

- The workspace publishes nine crates, and a tool adds one crate for the
  types and the runtime.
- A crate that is not a command-line tool builds clap when it uses
  crumbkit's failure or document types, until a split returns.
- `docs/behaviour/adoption/README.md` lists the types that need no clap, so
  a later split can start from that list.

Considered: keeping crumbkit-core for a crate that is not a command-line
tool, which no crate in the workspace is and no known adopter is; clap behind an
optional feature of crumbkit, which ADR-0032 rejected because it needs cfg
attributes across many modules and a second CI build, and would not stop
clap-free code from starting to use clap; keeping the clap ban with the
remaining crates as its wrappers, which would protect only the macro crate.
