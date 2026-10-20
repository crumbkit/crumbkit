# Adoption

This page covers how a tool adopts crumbkit: the parts it can take one at a
time, the wiring of its `main`, the types that need no clap, and the
promises of crumbkit's public API. The other pages in this directory cover
[opt-outs](opt-outs.md), with which a tool departs from a convention for a
stated reason, and [declarations](declarations.md), with which a tool keeps
its own names.

## Parts a tool takes one at a time

A tool keeps its own clap derive types and its own `main`, and each part of
crumbkit works on its own, so a tool adopts only the parts it wants. The
default build depends on few crates, which `Cargo.toml` lists with a reason
for each ([dependencies](../../project/dependencies.md)). It enables clap's
`help` feature, so a tool built with crumbkit always has `-h` and `--help`.

Each part follows an existing convention:

- `Failure` extends `std::error::Error` ([failures](../failures/README.md)).
- `Output` is added with clap's `#[command(flatten)]`, or built with
  `Output::new()` by a tool that defines the view flags itself
  ([output](../output/README.md#view-flags)).
- Results derive `Serialize` and `JsonSchema` and implement `Row` to name
  their table columns ([output](../output/README.md)).
- Each crumbkit-test check is a free function that a tool calls from an
  ordinary `#[test]`, as it calls clap's `Command::debug_assert`
  ([checks](../checks/README.md)).

Each argument type that crumbkit gives a tool to flatten or to take as a
command's arguments, such as `Output`, `Page` or `CommandsList`, clears the
command's about and long about, because clap otherwise copies the type's
rustdoc into the help of a command that has no doc comment of its own. A
derived command sets its own doc comment after the types it flattens, so it
keeps it, and a command built by hand sets its about after it adds such a
type.

Handlers take an arguments struct deriving `clap::Args`, `Serialize` and
`JsonSchema`, and never see `ArgMatches`. A command sets its own output
defaults on its arguments type, such as the default limit of the `Page` it
flattens.

Why: ADR-0015, ADR-0002.

## Wiring `main`

crumbkit has no `run` one-liner. A hand-wired `main` is a few lines of glue:
`parse`, a `match` on its result that keeps the returned `Output`, and one
`finish` per leaf, which takes the `Output` by value as the last call of the
run ([parsing](../parsing/README.md)). A `run` could not call
crumbkit-aliases or start the usage log, because crumbkit-aliases and
crumbkit-usage both depend on crumbkit.

- A tool that keeps the usage log depends on crumbkit-usage and
  calls `crumbkit_usage::start()` first in `main`, and changes nothing else
  ([usage log](../usage-log/README.md)).
- A command that hands the terminal to another program, such as an editor,
  depends on crumbkit-child and returns `crumbkit_child::hand_over`
  ([child processes](../output/child-processes.md)).
- A tool with shell completion turns on the `complete` feature and calls
  `crumbkit::complete` before `start`
  ([completion](../parsing/completion.md)).
- The trace context and the `tracing` feature need no wiring beyond that and
  a subscriber the tool installs
  ([trace context](../usage-log/trace-context.md)).
- A tool lets a person pick missing values with one `Guide::picker` line, a
  `Leaf::choices` line for each argument whose values come from a source, and
  `NoInput` flattened at its root ([pickers](../effects/pickers.md)).
- A tool with personal aliases passes its arguments through
  `crumbkit_aliases::expand` before `parse_from`
  ([aliases](../parsing/aliases.md)).

Every function that takes a `clap::Command` works on any node of the tree,
so a crate or a team can apply crumbkit and crumbkit-test to its own subtree.
The first segment of a code names the component that owns the code, so a
crate keeps its codes in whatever binary includes it. In a tool built from
one crate, the component and the tool have the same name.

`show` accepts many ids and returns a `Batch`.
[`fetch_each`](../../../crates/crumbkit/src/fetch.rs) runs a per-id lookup on
a bounded number of scoped threads for a tool whose backend has no batch
lookup. crumbkit has no default worker count, so each tool chooses one. An
async tool uses its own bounded stream, because `fetch_each` blocks the
calling thread, and a tool that needs rate limits or retries writes its own
loop.

Why: ADR-0015.

## The types that need no clap

crumbkit holds the types that compile without clap beside the parsing, help
and output that read a `clap::Command`: `Code` and `Spelling`, `Failure`,
`Recovery`, `Error` and `AsFailure`, `Document`, `Call` and `CallError`,
`Row`, `Rows`, `Shape` and `NoRow`, `Listing`, `Batch`, `Prose`, `Bytes`,
`Age` and `Stream`. Every crate that uses them builds clap through crumbkit. A crate
that is not a command-line tool, such as a web server that maps a `Failure`
to an RFC 9457 problem-details response, can ask for these types in a crate
of their own when such a crate exists, and this list says what that crate
would hold.

- `Rows::SHAPE` names a result's kind, so `schemas::document_schema` builds
  the fields of a document from its result type, and `Document::untyped`
  returns the same id as a `Document<()>`, so a `Leaf` keeps a document
  without its type.
- `Call` holds its data without reading clap, and crumbkit's sealed
  `RenderCall` trait gives it the `render` method that reads a
  `clap::Command`. `Call::help` and `Call::target` are public and
  documented.
- `FieldValue::from_json` reads a failure field back from a
  `serde_json::Value`, so a test or a tool that reads failure fields from
  JSON does not copy the conversion. It puts serde_json's major version into
  crumbkit's API.
- The `derive` feature re-exports
  [`#[derive(Failure)]`](../failures/derive.md) from crumbkit-derive beside
  the trait of the same name, and crumbkit holds
  [`Error::unclassified`](../failures/README.md#unclassified-errors).
- The `miette` feature adds `Failure::diagnostic` and the conversion of a
  `miette::Report` into `Error`.

Why: ADR-0097, ADR-0002, ADR-0001.

## API promises

Each release pull request runs cargo-semver-checks against the last
published version ([releases](../../project/releases.md)).

- Configuration types have private fields and `const fn` builders. A type
  that holds data read at run time, such as `RuntimeLeaf` or crumbkit-test's
  `RuntimeFailing`, has ordinary builders where a `const fn` cannot add to a
  `Vec` or replace an owned value, and leaks into the `'static` type that the
  `const` builders make. `RuntimeFailing::at` makes a `RuntimePlace`, a
  separate type without a code, from which `RuntimePlace::line` builds each
  command line, so a place passed where a command line belongs is a compile
  error. The two types share one private settings struct.
- Enums that may grow are `#[non_exhaustive]`, and adopter-implemented
  traits, such as `Picker`, gain only methods with defaults. `Answer` is
  `#[non_exhaustive]`.
- Failure codes are never reused.
- The keys of `#[failure(...)]` and the code the derive writes for them are
  public API, as [deriving `Failure`](../failures/derive.md) states.
- The run observer's `Observer` trait gains only methods with defaults, and
  its `Refused` and `Finished` events are `#[non_exhaustive]` with getters
  ([run observer](../run-observer/README.md)).
- `parse` returns a [`Stop`](../../../crates/crumbkit/src/parse/stop.rs) in
  place of a bare `ExitCode` when it ends the run
  ([parsing](../parsing/README.md#the-stop-of-a-run-that-parse-ended)).
- The `complete` feature's signatures use only clap and the standard
  library, but what it offers and how a shell registers it may change in a
  crumbkit minor release, and the changelog names each such change
  ([completion](../parsing/completion.md#stability-and-tests)).

A run exits with 0 when it succeeds, 1 by default on a failure and 2 on a
refused command line. A tool may give any failure its own code from 3 to 125.
It may also declare a code from 129 to 159 for a run that a signal ended, and
a range of codes that a command passes on from another program
([exit codes](../failures/exit-codes.md)). A shell reports 126 and above for
its own reasons, such as 130 when SIGINT ends the process and 143 when
SIGTERM does, and the usage log's signal handling leaves those codes in
place.

Why: ADR-0002, ADR-0012, ADR-0001, ADR-0057, ADR-0103.
