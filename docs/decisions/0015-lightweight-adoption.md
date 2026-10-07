# 0015. Lightweight adoption

- crumbkit is light to adopt in concepts and in dependencies. A tool keeps its own clap derive types and its own `main`, and each part works on its own, so a tool adopts only the parts it wants. The default build depends only on clap, serde, serde_json and schemars.
- Each part follows an existing convention: `Failure` extends `std::error::Error`, `Output` is added with clap's `#[command(flatten)]`, results derive `Serialize` and `JsonSchema`, and `crumbkit_test::check(Cli::command())` has the shape of clap's `debug_assert` test.
- The optional `crumbkit::run` one-liner is a thin layer over the public parts. It adds no concept, does nothing a hand-wired `main` cannot, and no part requires it.
- Help pages, the `Call`s in `Next:` lines and recoveries, and MCP tool lists are rendered from the `clap::Command` a tool builds at runtime, which may hide or remove commands. A `Call` that does not resolve in that `Command` is left out.
- A command sets its own output defaults on its arguments type. An optional table passed to `crumbkit::parse` sets defaults for a branch by command path, and the nearest entry wins, written into each flag's default before parsing.
- A command that refuses the caller returns an ordinary `Failure`. crumbkit-test and the golden files check the full tree.
- Every function that takes a `clap::Command` works on any node of the tree, so a crate or a team can apply crumbkit and crumbkit-test to its own subtree. The `schema` and `self usage` commands and the usage log stay at the root.
- The first segment of a code names the component that owns the code, so a crate keeps its codes in whatever binary includes it. In a tool built from one crate, the component and the tool have the same name.
- `show` accepts many ids. `crumbkit::fetch_each(ids, workers, lookup)` runs a per-id lookup on a bounded number of scoped threads and returns one result per id, in the order given. A tool whose backend has a batch lookup calls that lookup directly instead.

Considered: an `App` runner that owns dispatch, a required derive macro, rendering from the compile-time tree, a trait with a provided `show_many`, an async runtime for concurrent lookups.
