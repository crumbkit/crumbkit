# Review of crumbkit against a clap-only build

This is the report of the first run of the `crumbkit-vs-clap` eval. A Fable
agent wrote it on 2026-10-13 against commit 1013fe2. The two programs it built
are in `clap-only/` and `with-crumbkit/` beside this file. Its path:line
citations refer to commit 1013fe2.

## What crumbkit is

crumbkit is a set of Rust types, traits and test checks that sit on top of a tool's own clap derive types and give the tool a fixed set of conventions aimed at agents as callers: global `--json`/`--fields`/`--long`/`--full` flags, a JSON envelope with a schema id, failures with stable codes and a rendered next command, an output byte budget, help pages with examples and a `Next:` line, a `commands list` introspection command, and an optional local usage log (crates/crumbkit/README.md, crates/crumbkit/src/lib.rs:85-101). The tool keeps its clap structs and its own `main`, and replaces `Cli::parse()` with `crumbkit::parse` and its printing with `crumbkit::finish` (crates/crumbkit/src/parse.rs:80, crates/crumbkit/src/output.rs:227). The project is at version 0.0.0, is unpublished, and its 495 commits all land between 2026-10-07 and 2026-10-13.

I built the README example and a clap-only equivalent in the scratchpad and ran both. The README example compiles and behaves as documented. The clap-only version took about 70 lines of hand-written output code to cover a table, `--json`, `--limit`/`--all`, a footer and one coded failure, and it covered none of `--fields` validation, `Next:` lines, recoveries, budgets or the failure JSON document.

## Capability comparison

| Capability | What crumbkit gives | Hand-rolling on clap | Verdict |
|---|---|---|---|
| Table and JSON output, `--long`, `--fields` | `Row::COLUMNS`/`LONG_COLUMNS` (crumbkit-core/src/row.rs:44), `Output` flags (output.rs:71), `--fields` validated against the schemars schema with the valid names in the refusal (verified: `--fields nope` lists `author, id, title, year`) | 100-200 lines with serde_json::Value; validation of field names and column order needs the schema or a hand-kept list | Real but modest value; the validation message is the part people skip |
| JSON envelope `{"schema","data","total","truncated"}` | `Document::new(component, name, version, about)` plus `Listing`, `Batch`, `Prose`, `Bytes` shapes | 20-40 lines, low subtlety | Thin on its own; the value is that the envelope is shared with `schemas list/show` and the failure document |
| Paging `--limit`/`--all` with bounds | `Page<N, MOST>` const generics, compile-time checked, refusal of `--limit 101` renders a recovery with `--all` (page.rs:87, golden books-list-limit-101.stderr) | 30 lines; the recovery rendering is the only subtle part | Modest value; the const-generic default needs clap's `string` feature, which crumbkit turns on for you |
| Failures with code, recovery, alternatives, fields, JSON form | `Failure` trait (failure.rs:115), `Recovery::Run(Call)` rendered and validated against the live clap tree, debug-build warnings for calls that do not resolve (ADR-0001) | thiserror gives the type; a code enum and a JSON form take 80-150 lines; rendering a suggested command against the tree and refusing values clap would refuse is 200+ lines and is the part most teams get wrong or skip | Real value, and the tree-validated recovery is hard to replicate |
| Output budget (1 MiB default, `CRUMBKIT_OUTPUT_BUDGET` lowers it) | Binary search over rows, batch `left_out` keys with a single recovery run, stream cut-off (ADR-0046 line 3) | 300+ lines with several semantic decisions; almost nobody builds this | Real and distinctive, and only matters when agents read the output |
| Help: goal headings, examples, columns, `Effects:` line, `Global flags:` line, `Next:` | `Guide`/`Leaf`/`Example`/`Topic` consts applied per typed path (parse/lines.rs:1-60) | `after_help` covers examples in 10 lines; grouped subcommands need a custom template per command; column docs from schema descriptions take ~100 lines | Moderate value; the token-budget framing is specific to agent callers |
| `commands list --json --full` introspection | Walks `clap::Command` getters into a stable document (commands.rs:204, golden shelf-commands-list-books-list-json-full.txt) | 150-300 lines of clap getter code with pitfalls around global args and unbuilt subtrees | Real value for MCP-style discovery; clap has nothing built in |
| `schemas list/show` | schemars schemas of every document, including crumbkit's own | 50 lines with schemars | Thin |
| Confirm, `--yes`, `--dry-run`, agent detection | `Output::confirm`, `Yes`, `DryRun`, is-ai-agent so no question reaches an agent (confirm.rs:47) | 60 lines plus is-ai-agent | Thin to moderate |
| Pickers for missing required args | `Picker` trait and `crumbkit-picker::Menu`, re-parse after picking (ADR-0043) | 200 lines, interactive edge cases | Moderate, niche |
| Per-id batches | `Batch`, `fetch_each` on scoped threads | 100 lines | Thin |
| Usage log and `self usage` findings | Keyed SipHash of args, no values recorded, lock file, trimming, signal thread, 5 finding kinds (ADR-0029) | 2-3k lines; nobody hand-rolls this | Real engineering, speculative need; it also replaces the default SIGINT/SIGTERM action (ADR-0029 line 21) |
| Shell completion | Wraps clap_complete dynamic completion, pinned `~4.6.11` (Cargo.toml:125) | 20 lines on clap_complete directly | Thin; crumbkit adds dashed-form lookup and log suppression only |
| Hand-over to a child (editor, pager) | `child::hand_over` with signal masking and a JSON child form | 100 lines, signal handling is subtle | Moderate, niche |
| tracing spans | `parse` and `finish` spans behind a feature | 10 lines | Thin |
| Convention tests (21 `check_*` functions, crates/crumbkit-test) | Grammar, one name per command, effects declared, help size and path budgets, goldens, twins (a refused line beside an accepted line differing by one word), recovery chains that run each suggested command and check it exits 0 (recoveries.rs:441, twins.rs), opt-out report | trycmd/snapbox give goldens cheaply; the recovery chain and twin checks have no equivalent and would cost 2-5k lines | This is the strongest part; it is also the most opinionated |

## Costs and risks

The learning surface is large. crumbkit and crumbkit-core expose 213 public items, the help module alone has 15 public types, and crumbkit-test adds 21 check functions with settings types (`Examples`, `Recoveries`, `Twins`, `Budgets`, `PathBudgets`, `Allow`, `Rule`). The design record is 56 ADRs totalling about 63,000 words, written densely. The shelf example needs 326 lines of `cli.rs` and 249 lines of convention tests for four domain commands (examples/shelf/src/cli.rs, examples/shelf/tests/conventions.rs).

Dependency weight is moderate. The default build has 29 crates against 19 for clap+serde derive, because schemars and serde_json are mandatory. A cold release build of the README example took 9.3 s against 3.7 s for the clap-only tool, and the binary grew from 1.0 MB to 1.7 MB. All features together reach 50 crates. MSRV is 1.89 (Cargo.toml:20).

Maturity is the largest risk. Nothing is published, the README says `main` can change without notice, every commit is from one week, and the `complete` feature tracks clap_complete's unstable API. Golden files and refusal messages depend on clap's exact wording (ADR-0021 last bullet), so a clap minor release can fail an adopter's convention tests.

Lock-in is real at the output layer. `Failure` impls, `Document` consts, `Row` impls, the `Guide` const and the `&mut Output` threading through handlers all go away if you leave; the clap structs survive. The schema ids and failure codes become a public contract to callers, which you would then have to re-implement by hand to keep compatibility.

The opinions will conflict with many tools. `check_grammar` requires `<tool> <resource> <verb>` and reserves `help` (grammar.rs:91); visible aliases fail `check_names`; the global flag names are fixed; destructive leaves must take `--yes`; the output budget cannot be removed, only raised with a written reason; a `show` command must accept many ids; `Page` defaults to 25 rows. Each departure is an `Allow` with a reason that lands in a golden opt-out report.

Abstraction leaks I found: (1) a root command without a doc comment picked up `Page`'s multi-paragraph rustdoc as its `about` and `long_about` in my scratchpad build, because clap's flatten copies a flattened struct's doc when the parent has none; (2) a forgotten `set_command` degrades `Next:` lines and recoveries to `Recovery::None` silently in release builds (ADR-0015 line 5 explains why there is no `run` helper); (3) `Failure` is not found through the `source()` chain, so each wrapping error type delegates five methods by hand (failure.rs:44-50), and no derive macro exists yet (ADR-0001 line 18); (4) `miette::Report` converts to one fixed unclassified code, and anyhow has no integration, so an anyhow-based codebase rewrites its error layer to get stable codes; (5) `--full` requires `--json` and `--fields` conflicts with `--long`, and these refusals print with clap's wording.

## Who should use it and who should not

A team building a tool whose main caller is a coding agent or another program, with a resource/verb command shape, many list/show commands, and a wish to publish stable failure codes and JSON schemas, gets real leverage from the failure model, the budget, `commands list` and the recovery-chain tests. A team that already wants a convention suite and is willing to accept crumbkit's as written saves the most.

A developer writing a single-purpose tool, a tool with free-form or verb-first commands, a tool that returns large or binary results, a tool built on anyhow, or anyone who needs a published, semver-stable dependency today should hand-roll on clap plus thiserror/miette, serde_json and clap_complete. The pieces they would reasonably build themselves (table, `--json`, paging, a code field on errors) cost under 300 lines.

## Verdict and top three changes

crumbkit delivers real value in three places: failures whose recovery commands are rendered and validated against the live tree, the output budget with batch cut-off, and the convention test suite whose recovery and twin checks run the tool. The rest is convenient but thin, and the usage log is substantial engineering for a need the project has not yet shown. Today a competent Rust developer should hand-roll unless they are building an agent-first, resource/verb tool and can tolerate an unpublished dependency. Once published and stable, the answer tips toward adoption for that class of tool.

The three changes that would most raise value or lower cost are: (1) ship `crumbkit-derive` for `Failure` and a `From<anyhow::Error>`/downcast path, so the error layer costs one attribute instead of a five-method impl per type; (2) publish 0.1 with a short tutorial that walks from `Cli::parse()` to `parse`/`finish` in under a page, since the 63,000-word ADR tree is the only deep documentation; (3) let `check_*` be adopted one at a time without the opt-out report and golden files as the first step, and relax or shrink the mandatory default build (make schemars optional for tools that skip `--fields` validation and schemas), so the entry cost in concepts and compile time drops for a tool that only wants the failure model and JSON envelope.
