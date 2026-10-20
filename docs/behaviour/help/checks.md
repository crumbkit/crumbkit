# Help checks

This page covers crumbkit-test's checks of a tool's help pages: where they
read the pages, the budgets on page sizes and path weights, and the golden
files of help. The [checks](../checks/README.md) page covers crumbkit-test's
other checks. Each check's rustdoc lists every problem it finds.

## Where the checks read help pages

[`check_help_sizes`](../../../crates/crumbkit-test/src/sizes.rs),
[`check_help_goldens`](../../../crates/crumbkit-test/src/goldens.rs) and
[`check_path_weights`](../../../crates/crumbkit-test/src/weights.rs) each
take a [`Pages`](../../../crates/crumbkit-test/src/pages.rs), which says where
they read a tool's help pages:

- `Pages::rendered` runs `crumbkit::parse_to` in memory on a clone of the
  tree. A page is then the plain text that a caller reads on a pipe, with
  every line crumbkit adds except the line that names a person's aliases.
- `Pages::binary` runs the tool's binary for each page, as `check_examples`
  runs an example. A tool that builds part of its help at run time, such as
  from settings it reads at start, passes it. The example tool reading reads
  all three checks' pages through its binary.
- `Pages::clap` renders each page with clap alone, for a tool that parses
  with clap alone, and `PathBudgets::help_only` budgets that tool's paths.

A tool passes the same source to all three checks. A tool that mixes sources
measures its budgets on pages that no golden file pins.

The help checks skip hidden commands and everything below them. They take
budgets in place of opt-outs.

Why: ADR-0025, ADR-0015, ADR-0027.

## Page budgets

A budget counts the bytes of the plain-text `-h` page. `--help` has no
budget. [`Budgets::new`](../../../crates/crumbkit-test/src/sizes.rs) holds
these default budgets:

| Command | Default budget |
| --- | --- |
| The root | 1,500 bytes |
| A group, which is a command below the root with subcommands | 800 bytes |
| A leaf, which is a command below the root without subcommands | 1,200 bytes |

When a page is larger than its budget, `check_help_sizes` fails and names
the command, the page's size and the budget. A tool can raise or lower any
budget with `Budgets`, for a whole class or for single commands, and gives a
reason for each budget it sets. crumbkit-test's `check_opt_outs` lists each
budget that a tool set with its reason, crumbkit's default, and the extra
bytes and tokens of a raised budget ([opt-outs](../adoption/opt-outs.md)).

Why: ADR-0025, ADR-0036.

## Path budgets

A path is every `-h` page from the root down to a command plus the command's
output weight, which is the size of the largest golden file of its examples
([help weight](weight.md#output-weight)). A command without an example
golden file, such as one whose examples an opt-out skips, counts 0 bytes of
output. [`PathBudgets::new`](../../../crates/crumbkit-test/src/weights.rs)
holds a default budget of 8,000 bytes for every path, about 2,000 tokens at
4 bytes a token. A depth-2 leaf at its default page budgets reads 3,500
bytes of help, so 8,000 bytes leaves 4,500 bytes for output. When a path weighs more than its budget,
`check_path_weights` fails and names the command, the weight, the bytes of
its pages and of its output, and the budget. A tool raises or lowers path
budgets with `PathBudgets` and a reason, as it does page budgets. shelf
lowers its paths to 4,300 bytes.

`check_path_weights` sums the paths with the same `path_totals` and
`output_weight` functions that `cargo crumbkit weigh` calls, so the check and
`weigh` agree whenever the pages that `Pages` chooses equal the pages that
`weigh` reads from the binary. A cargo-crumbkit test compares weigh's `short_help`
values for shelf with the sizes of shelf's help golden files.

Why: ADR-0025, ADR-0033.

## Help golden files

`check_help_goldens` compares each `-h` and `--help` page with a golden file
through snapbox ([golden files](../checks/golden-files.md)). It fails on a
stale file, so its directory holds only help golden files. The help checks
print help without a person's aliases, through
`Guide::without_personal_aliases`, so the golden files are the same on every
machine.

Why: ADR-0025, ADR-0004, ADR-0027.

## Checks that stay separate

crumbkit-test has no bundled check that runs every check. A tool calls each
check from its own `#[test]`, so a failure names the rule that broke.

- `check_guide` holds the leaf rules, and takes opt-outs of its example
  rule and its document rule alone
  ([leaf checks](examples.md#checks-of-leaves)).
- [`check_exit_codes`](../../../crates/crumbkit-test/src/exits.rs) checks the
  declared exit codes.
- `check_path_weights` reads golden files, and `check_help_sizes` reads only
  memory.
- `check_mounts` is separate from `check_document_ids`, because a crumbkit
  command at the wrong path is fixed by moving it
  ([schema commands](../schemas/commands.md)).
- `check_output_budget` is separate from `check_examples`, because it runs
  each example twice ([output budget](../output/budget/README.md#the-budget-check)).

Why: ADR-0025, ADR-0037, ADR-0035, ADR-0046.
