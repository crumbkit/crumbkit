# crumbkit-test

crumbkit-test checks a clap tool against crumbkit's conventions from an
ordinary `#[test]`. Each check runs on its own, collects every problem it
finds and fails the test with all of them, so a tool turns checks on one at
a time. crumbkit is in design and is not published yet.

## Adding it

A tool adds crumbkit-test as a dev-dependency, from this repository until
the first release:

```toml
[dev-dependencies]
crumbkit-test = { git = "https://github.com/crumbkit/crumbkit" }
```

The `main` branch can change without notice before the first release.

## Checks for any clap tool

These checks read a `clap::Command`, so a tool that parses with clap alone
can run them without crumbkit's runtime:

```rust
use clap::{Arg, ArgAction, Command};
use crumbkit_test::{Budgets, Guide, Pages};

let tool = Command::new("shelf").subcommand(Command::new("books").subcommand(
    Command::new("list").arg(
        Arg::new("author").long("author").value_delimiter(',').action(ArgAction::Append),
    ),
));

crumbkit_test::check_filters(&tool, &[]);
crumbkit_test::check_names(&tool, &Guide::new(), &[]);
crumbkit_test::check_help_sizes(&tool, Pages::clap(), &Budgets::new());
```

- Names: [`check_names`] and [`check_grammar`], which take an empty
  [`Guide`].
- Arguments: [`check_listings`] and [`check_filters`].
- Help pages: [`check_help_sizes`] with [`Budgets`], [`check_help_goldens`],
  and [`check_path_weights`] with [`PathBudgets::help_only`], each reading
  clap's pages through [`Pages::clap`].

A check that takes opt-outs accepts `&[]`, or an [`Allow`] with a reason
for each rule the tool skips.

## Checks for tools built with crumbkit

These checks read the tool's guide, failure codes or crumbkit's output:

- Failure codes: [`check_codes`].
- Effects: [`check_effects`].
- The guide: [`check_guide`], and [`check_exit_codes`].
- Help pages: the checks above with [`Pages::rendered`] or
  [`Pages::binary`], and [`PathBudgets`] with the example golden files, and
  [`path_totals`] and [`output_weight`], which `cargo crumbkit weigh` also
  calls.
- Example output: [`check_examples`] with [`Examples`], and
  [`example_golden_name`]. `Examples` sets each run's working directory or
  fixture, and the values replaced with placeholders.
- The output budget: [`check_output_budget`] ([ADR-0046]).
- The command reference: [`check_command_reference`] ([ADR-0044]).
- Recoveries: [`check_recoveries`] with [`Recoveries`], [`check_twin`], and
  [`check_twins`] with [`Twins`]. [`RuntimeFailing`] builds command lines
  read at run time, and a [`FailureReader`] reads failures printed in a
  tool's own shape.
- JSON documents: [`check_schema_goldens`], [`check_document_ids`] and
  [`check_mounts`].
- Opt-outs: [`check_opt_outs`] pins the opt-outs and budgets that a tool
  passes to the other checks, with their reasons and costs, in golden
  files.

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
[`Guide`]: ../crumbkit/src/help.rs
[`Pages::clap`]: src/pages.rs
[`Pages::rendered`]: src/pages.rs
[`Pages::binary`]: src/pages.rs
[`PathBudgets::help_only`]: src/weights.rs
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
[`check_twins`]: src/twins.rs
[`Twins`]: src/twins.rs
[`RuntimeFailing`]: src/recoveries/runtime.rs
[`check_schema_goldens`]: src/schemas.rs
[`check_document_ids`]: src/documents.rs
[`check_mounts`]: src/documents.rs
[ADR-0025]: ../../docs/decisions/api/help/0025-help-checks.md
[ADR-0035]: ../../docs/decisions/api/schemas/0035-schema-commands.md
[ADR-0036]: ../../docs/decisions/api/adoption/0036-opt-outs.md
[ADR-0004]: ../../docs/decisions/testing/0004-golden-and-snapshot-tests.md
[ADR-0044]: ../../docs/decisions/api/help/0044-command-reference.md
[ADR-0046]: ../../docs/decisions/api/output/0046-output-budget.md
