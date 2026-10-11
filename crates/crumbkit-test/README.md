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
  [`check_listings`] and [`check_filters`].
- The help guide: [`check_guide`].
- Help pages: [`check_help_sizes`] with [`Budgets`], and
  [`check_help_goldens`].
- Paths through the help: [`check_path_weights`] with [`PathBudgets`], and
  [`path_totals`] and [`output_weight`], which `cargo crumbkit weigh` also
  calls.
- Example output: [`check_examples`] with [`Examples`], and
  [`example_golden_name`], which names each example's golden file.
- JSON documents: [`check_schema_goldens`], which pins each document's
  JSON Schema, [`check_document_ids`], which ties each document's name to
  its command, and [`check_mounts`], which keeps crumbkit's own commands at
  their fixed paths.

[ADR-0025] records why these checks exist, [ADR-0035] records the document
checks, and [ADR-0004] records how golden files work.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`check_codes`]: src/codes.rs
[`check_names`]: src/names.rs
[`check_grammar`]: src/grammar.rs
[`check_listings`]: src/listings.rs
[`check_filters`]: src/filters.rs
[`check_guide`]: src/guide.rs
[`check_help_sizes`]: src/sizes.rs
[`Budgets`]: src/sizes.rs
[`check_help_goldens`]: src/goldens.rs
[`check_path_weights`]: src/weights.rs
[`PathBudgets`]: src/weights.rs
[`path_totals`]: src/weights.rs
[`output_weight`]: src/weights.rs
[`check_examples`]: src/examples.rs
[`Examples`]: src/examples.rs
[`example_golden_name`]: src/examples.rs
[`check_schema_goldens`]: src/schemas.rs
[`check_document_ids`]: src/documents.rs
[`check_mounts`]: src/documents.rs
[ADR-0025]: ../../docs/decisions/api/help/0025-help-checks.md
[ADR-0035]: ../../docs/decisions/api/0035-schema-commands.md
[ADR-0004]: ../../docs/decisions/testing/0004-golden-and-snapshot-tests.md
