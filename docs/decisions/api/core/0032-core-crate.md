# ADR-0032. The clap-free core crate

- Date: 2026-10-11
- Status: superseded by ADR-0097

## Context

crumbkit's failures, codes, documents and result types are useful outside
a command-line tool, such as in a web server that maps a `Failure` to an
RFC 9457 problem-details response. Such a crate should not build clap.

## Decision

- crumbkit-core took the types that need no clap, and crumbkit
  re-exported each at its old path, so a tool built on crumbkit keeps its
  code.
- An item belonged in crumbkit-core when it compiles without clap and a
  caller that is not a command-line tool would use it. Parsing, help,
  output, refusals, the run observer and the usage log stayed in
  crumbkit, because each is built on clap or serves only a process run
  from a command line.
- `Call` kept its data in crumbkit-core, and crumbkit's sealed
  `RenderCall` trait added the rendering that reads a `clap::Command`.
- crumbkit-core took serde_json, because `Batch::has_room` measures rows
  as compact JSON, and `FieldValue::from_json` went beside `FieldValue` so
  that callers do not copy the conversion. This couples nothing new,
  because `FailureForm::json` already exposes serde_json.
- The `derive` feature re-exported `#[derive(Failure)]` beside the trait,
  because a procedural macro must live in its own crate.
- `Error::unclassified` and its code went in crumbkit-core, because a
  crate that is not a command-line tool also meets errors it does not
  classify.
- `deny.toml` banned clap outside its listed users, so `cargo deny check
  bans` fails when clap enters crumbkit-core's tree.

## Consequences

A crate can depend on crumbkit-core alone and build no clap.
`docs/behaviour/adoption/README.md` describes the crates an adopter
chooses between.

Considered: clap behind an optional default feature of crumbkit, which needs cfg attributes across many modules and an extra CI build without the feature, and would not stop clap-free code from starting to use clap; `render` as a free function; the failure document in crumbkit-core; `fetch_each` in crumbkit-core; `Call::help` hidden from the docs, `Call::word` for runtime words that skip the check against the tree, `TryFrom<&serde_json::Value>` for `FieldValue` with an error type, a fraction read as its JSON text, the conversion in crumbkit or crumbkit-test.
