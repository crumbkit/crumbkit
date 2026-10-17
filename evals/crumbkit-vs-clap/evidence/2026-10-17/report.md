# crumbkit re-review at origin/main (514e3d0), against the review at 1013fe2

This is the report of the second run of the `crumbkit-vs-clap` eval. A Fable
agent wrote it on 2026-10-17 against commit 514e3d0. The programs it built
(`ex-old`, `ex-new`, `ex-clap`, `ex-edge`), its measurement script and its
outputs are beside this file. `ws-test-summary.out` holds the result lines
of the 323 KB workspace test log. In that log, three xtask tests fail
because `git archive` leaves out `.git`. `xtask-test.out` is the rerun with
a `.git` directory, and those three tests pass there. The path:line citations
refer to commit 514e3d0, except where a citation names the old tree.

I read both trees from `git archive` extracts in the scratchpad, rebuilt the README example and a clap-only equivalent on both trees, ran the workspace tests on the new tree, and built two throwaway binaries to exercise the "rough edges" fixes. The working copy was not touched. The workspace suite passes: 800 crumbkit unit tests, 431 crumbkit-test tests, 148 core tests, 98 xtask tests and every doctest pass (`cargo test --workspace`, then `--doc`, in `scratchpad/rereview/new`; the three xtask tests that failed on the first run only needed a `.git` directory, which `git archive` leaves out).

## Status of each earlier finding

| Finding | Status now | Evidence |
|---|---|---|
| 1. Root without a doc comment showed Page's rustdoc as about | Fixed | `crates/crumbkit/src/page.rs:89` and `output.rs:81` set `#[command(about = None, long_about = None)]`; the same on `expect.rs:72`, `commands.rs:113`, `schemas.rs:146,162`. Tests in `src/output/tests/about.rs:31-80` cover six flattened types. My `nodoc` binary (Output + Page, no doc comment) prints `--help` with an empty first line and no stray description. |
| 2. Forgotten `set_command` silently drops Next: lines in release | Partly fixed | `output.rs:338` warns only under `cfg!(debug_assertions)`. My `forgot` binary in release prints the listing without a Next: line and the failure as `help: no next step: the suggested command is not available`, exit 0/1, with no warning. The debug build prints `warning: crumbkit leaves out every Next: line…`. crumbkit-test's `check_examples` and `check_recoveries` fail on that line (`crumbkit-test/src/examples.rs:396`, `recoveries/chain.rs:559`), so a tool that runs those checks catches it in CI. |
| 3. Failure not found through `source()`; hand-written forwarding; no derive | Partly fixed | A `#[derive(Failure)]` exists in the new `crumbkit-derive` crate behind the `derive` feature, with `forward` and `forward = FIELD` keys (`crumbkit-derive/src/lib.rs:22-35`), 22 unit tests plus 14 integration tests in `crumbkit-core/tests/derive.rs`; shelf uses it (`examples/shelf/src/books.rs:373`). The `source()` limitation stands and is now documented as a rule at `crumbkit-core/src/failure.rs:49-53`. |
| 4. `miette::Report` maps to one fixed code; no anyhow integration | Partly fixed | `crumbkit-core/src/report.rs:99-101` still returns `REPORT_UNCHECKED` for every report. New `Error::unclassified` (`crumbkit-core/src/unclassified.rs:72`) wraps any `Box<dyn Error>` and documents a downcast-first pattern for anyhow; ADR-0001 lists an `anyhow` feature as considered and rejected. |
| 5. `--full/--json` and `--fields/--long` refusals used clap's wording | Fixed | Old `output.rs:82,91` used `conflicts_with_all` and `requires`. New README binary prints `error: --full prints every field of every row in JSON, so it needs --json / help: add --json, or drop --full / code: crumbkit::cli::usage`, exit 2; golden `examples/shelf/tests/golden/books-list-full.stderr` pins it. |
| 6. Goldens depend on clap's error wording; `complete` tracks unstable clap_complete | Unchanged | Six shelf goldens still hold clap sentences such as `the following required arguments were not provided`; `Cargo.toml:148` still pins `clap_complete = "~4.6.11"`; ADR-0021 line 40 states the golden files fail when clap's wording changes. |
| 7. No short tutorial | Fixed | `docs/tutorial.md` (1,227 words, five steps) with code in `examples/tutorial/src/bin/{start,step_1,step_2,step_3}`; `examples/tutorial/tests/page.rs` fails when a Rust block on the page differs from the file it links. |
| 8. crumbkit-test needed a `Guide` for nearly every check | Partly fixed | `Pages::clap()` (`crumbkit-test/src/pages.rs:76`) renders clap's own help pages, and `PathBudgets::help_only` counts help alone. The README section "Checks for any clap tool" runs as a doctest and passes. `check_names` and `check_grammar` still take `&Guide`; a clap-only tool passes `&Guide::new()`. crumbkit-test still depends on crumbkit (`crumbkit-test/Cargo.toml:17`), so the clap-only tier builds all of crumbkit as a dev-dependency. |
| 9. schemars and serde_json mandatory | Unchanged | `crates/crumbkit/Cargo.toml` lists both as non-optional dependencies. |
| a. Usage log into its own crate behind an observer seam | Fixed | `crates/crumbkit-usage` (7,091 lines including tests) holds the log, `self usage` and `self figures list`; crumbkit's Cargo.toml no longer lists siphasher, etcetera, humantime, signal-hook or getrandom. The seam is `crumbkit::events::Observer` (`events.rs:61`). |
| b. Guide optional in the five checks | Partly fixed | See finding 8. The signatures are the same as at 1013fe2; the change is `Pages::clap` and `PathBudgets::help_only`. |
| c. Child into its own crate | Fixed | `crates/crumbkit-child` (1,312 lines) exports `hand_over`, `hand_over_json` and `Json`; crumbkit has no `child` module and no `hand-over` feature. Output gained `print_failure` and related methods in `src/output/direct.rs` so another crate can print as `finish` does. |
| d. Budget and batches stay in crumbkit | Done as suggested | `Output::DEFAULT_BUDGET` at `output.rs:175`; `Batch` still re-exported from `lib.rs`. |

## Measurements

My machine differs from the earlier reviewer's, so I rebuilt the old tree too. Each pair below uses one method on both trees.

| Measure | 1013fe2 (my rebuild) | origin/main | clap-only equivalent |
|---|---|---|---|
| Unique crates in README example dependency tree | 41 | 41 | 28 |
| `Compiling` lines, cold release build | 34 | 34 | 25 |
| Cold release build time | 9.19 s | 8.65 s | 5.49 s |
| Warm rebuild after touching main.rs | 0.61 s | 0.67 s | 0.37 s |
| Release binary | 1,718,688 B | 1,767,632 B | 1,005,376 B |
| crumbkit default feature tree, unique crates | 29 | 29 | |
| crumbkit all-features tree, unique crates | 50 | 41 | |
| shelf (full stack) dependency tree, unique crates | 67 | 69 | |
| `pub fn` in crumbkit + crumbkit-core (grep, excluding tests) | 272 | 311 | |
| `pub struct/enum/trait/type` in crumbkit + crumbkit-core | 81 | 78 | |
| Help module public types | 13 | 13 | |
| crumbkit-test check functions | 21 | 21 | |
| ADR files / words | 43 / 38,683 | 49 / 23,632 | |
| All docs words | 40,472 | 64,565 (44 new behaviour/project pages, 37,622 words) | |
| shelf `cli.rs` / `tests/conventions.rs` lines | 326 / 249 | 326 / 249 | |
| MSRV / version | 1.89 / 0.0.0 unpublished | 1.89 / 0.0.0 unpublished | |

The default build cost did not move. The binary grew by 49 KB. The all-features count fell only because two features became crates; a tool that adopts the whole stack pays slightly more than before (shelf 67 to 69 crates).

## New problems and regressions

1. The `Observer` seam is one global `OnceLock` slot (`events.rs:336-349`), first caller wins, with 17 methods on the trait. A tool that wants both crumbkit-usage and its own observer has to write a fan-out observer itself; crumbkit offers no composition.
2. `Pages::clap()` cannot fix the help width for tools with clap's `wrap_help`, so help-size and golden checks depend on the terminal that runs the test (`pages.rs:71-75`, marked RUST-10).
3. The public function count rose from 272 to 311 while usage and child left the crate. The leafcodes, envelope, headed and bytesline PRs added per-leaf codes, declared envelope keys and two more leaf forms; `Document` grew from 12 to 17 public functions.
4. `CHANGELOG.md` keeps "Added" entries for crumbkit's `usage-log` and `hand-over` features (lines 39-47, 159, 166, 181, 192, 287) and records the moves only under "Changed" at line 563 and 573. Nothing is published, so the Unreleased section describes features that do not exist.
5. Release builds still hide a forgotten `set_command`, as in finding 2.
6. The decision records are now frozen once dated (`docs/decisions/README.md:31`), and the behaviour docs took over the description of current behaviour. Total documentation grew by 24,000 words, so the reading load to adopt the full stack rose, although the tutorial shortens the first step.

## Whether the strongest value holds and whether the adoption cost fell

The tree-validated recoveries, the output budget and the convention tests still work as before: shelf's `conventions.rs` and `books.rs` tests pass, `check_twins` and `check_recoveries` are unchanged in signature, and the README binary still prints a rendered `help: run \`books\`` line. The refusal wording moved into crumbkit's own voice, which closes one gap between the agent-facing failure format and clap's.

The first-step cost fell: the tutorial exists, the derive removes the hand-written forwarding impls, `Error::unclassified` gives arbitrary errors a place, and a clap-only project can run `check_filters`, `check_names`, `check_grammar`, `check_listings`, `check_help_sizes`, `check_help_goldens` and `check_path_weights` without crumbkit's runtime. The build cost did not fall: the default tree is still 29 crates against 19 for clap and serde derive, the cold build is still about 1.6 times the clap-only build, and the binary is 0.76 MB larger.

## Updated verdict

Adopt crumbkit now if the tool is agent-first, has a resource-then-verb command shape, can take a git dependency on an unpublished 0.0.0 crate, and will run crumbkit-test in CI, because CI is what turns finding 2 from a silent release-build omission into a failing test. The test tier makes a second group reasonable: a plain clap tool can add crumbkit-test as a dev-dependency to get the name, grammar, filter and help-size checks, at the price of building crumbkit in test builds only.

Hand-roll on clap if the tool needs schemars-free builds, must keep its binary near 1 MB, relies on anyhow throughout, or cannot accept golden files that break on a clap minor release.

The top three remaining changes:
1. Make the release build of `finish` fail loudly, or make `parse` return an `Output` that already holds the command, so a forgotten `set_command` cannot compile.
2. Put schemars and serde_json behind features, so a tool that skips `--fields` validation and `schemas` builds neither.
3. Give `events` an observer list or a composing observer, and let crumbkit-test depend on crumbkit-core and clap alone for the clap-only checks.
