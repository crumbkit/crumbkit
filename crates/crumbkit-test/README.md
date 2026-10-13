# crumbkit-test

crumbkit-test checks a tool built with crumbkit from an ordinary `#[test]`.
Each check collects every problem it finds and fails the test with all of
them. crumbkit is in design and is not published yet.

## Adding it

A tool adds crumbkit-test as a dev-dependency, from this repository until
the first release:

```toml
[dev-dependencies]
crumbkit-test = { git = "https://github.com/crumbkit/crumbkit" }
```

The `main` branch can change without notice before the first release.

## Example

This test checks that no two of a tool's failure codes spell the same:

```rust
use crumbkit::{Code, Spelling};

const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
const TIMED_OUT: Code = Code::new("shelf", "network", "timed_out");

const BOOK_CODES: &[&Code] = &[&NOT_FOUND];
const NETWORK_CODES: &[&Code] = &[&TIMED_OUT];

crumbkit_test::check_codes(Spelling::COLONS, &[BOOK_CODES, NETWORK_CODES]);
```

## Where to read more

The rustdoc of each check says what it fails on and shows an example:

- Failure codes: [`check_codes`].
- Command and flag names: [`check_names`], [`check_grammar`],
  [`check_listings`] and [`check_filters`], which take the tool's
  opt-outs, each an [`Allow`] with a reason.
- Effects: [`check_effects`], which requires each leaf to declare its
  effects and each destructive leaf to take `--yes`, and takes the tool's
  opt-outs.
- Opt-outs: [`check_opt_outs`], which pins the tool's opt-outs and
  budgets with their reasons and costs in golden files.
- The help guide: [`check_guide`], and [`check_exit_codes`], which takes
  the tool's opt-outs.
- Help pages: [`check_help_sizes`] with [`Budgets`], and
  [`check_help_goldens`], which read the pages that [`Pages`] chooses.
- Paths through the help: [`check_path_weights`] with [`PathBudgets`], and
  [`path_totals`] and [`output_weight`], which `cargo crumbkit weigh` also
  calls.
- Example output: [`check_examples`] with [`Examples`], and
  [`example_golden_name`], which names each example's golden file.
  `Examples` sets each run's working directory, or a fixture that each run
  starts in a new copy of, and the values that the check replaces with
  placeholders, and an example's `.stdin` file holds
  its standard input.
- The output budget: [`check_output_budget`], which runs each `--json`
  example with half the bytes it prints and checks that the tool cuts or
  refuses the result as crumbkit does ([ADR-0046]).
- The command reference: [`check_command_reference`], which compares a
  tool's checked-in Markdown reference with one written from its help
  ([ADR-0044]).
- Recoveries: [`check_recoveries`] with [`Recoveries`], which runs failing
  command lines and follows each recovery they print, and [`check_twin`],
  which runs a refused command line beside one that succeeds.
- JSON documents: [`check_schema_goldens`], which pins the JSON Schema of
  each document and file format, [`check_document_ids`], which ties each
  document's name to its command, requires an opt-out for a document
  without crumbkit's envelope and takes the tool's opt-outs, and
  [`check_mounts`], which keeps crumbkit's own commands at their fixed
  paths.

[ADR-0025] records why these checks exist, [ADR-0036] records opt-outs,
[ADR-0035] records the document checks, and [ADR-0004] records how golden
files work.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`check_codes`]: src/codes.rs
[`check_command_reference`]: src/reference.rs
[`Allow`]: src/allow.rs
[`check_opt_outs`]: src/opt_outs.rs
[`check_names`]: src/names.rs
[`check_grammar`]: src/grammar.rs
[`check_effects`]: src/effects.rs
[`check_listings`]: src/listings.rs
[`check_filters`]: src/filters.rs
[`check_guide`]: src/guide.rs
[`check_exit_codes`]: src/exits.rs
[`check_help_sizes`]: src/sizes.rs
[`Budgets`]: src/sizes.rs
[`check_help_goldens`]: src/goldens.rs
[`Pages`]: src/pages.rs
[`check_path_weights`]: src/weights.rs
[`PathBudgets`]: src/weights.rs
[`path_totals`]: src/weights.rs
[`output_weight`]: src/weights.rs
[`check_examples`]: src/examples.rs
[`Examples`]: src/examples/settings.rs
[`example_golden_name`]: src/examples.rs
[`check_output_budget`]: src/output_budget.rs
[`check_recoveries`]: src/recoveries.rs
[`Recoveries`]: src/recoveries.rs
[`check_twin`]: src/twins.rs
[`check_schema_goldens`]: src/schemas.rs
[`check_document_ids`]: src/documents.rs
[`check_mounts`]: src/documents.rs
[ADR-0025]: ../../docs/decisions/api/help/0025-help-checks.md
[ADR-0035]: ../../docs/decisions/api/schemas/0035-schema-commands.md
[ADR-0036]: ../../docs/decisions/api/adoption/0036-opt-outs.md
[ADR-0004]: ../../docs/decisions/testing/0004-golden-and-snapshot-tests.md
[ADR-0044]: ../../docs/decisions/api/help/0044-command-reference.md
[ADR-0046]: ../../docs/decisions/api/output/0046-output-budget.md
