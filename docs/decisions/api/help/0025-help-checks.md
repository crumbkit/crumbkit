# ADR-0025. Help checks

- crumbkit-test's [`check_help_sizes`](../../../../crates/crumbkit-test/src/sizes.rs) and [`check_help_goldens`](../../../../crates/crumbkit-test/src/goldens.rs) read the help pages that crumbkit prints, by running `crumbkit::parse_to` in memory on a clone of the tree. A page is therefore the plain text that a caller reads on a pipe, with every line crumbkit adds.
- The help checks skip hidden commands, because no help page shows them. They take no exemptions, because a tool sets a budget per command and pins every page.
- A budget counts the bytes of the plain-text `-h` page, which agents read first and most often. `--help` has no budget, because a caller asks for it to read the full descriptions and the columns. The default budgets are 1,500 bytes for the root, 800 for a group, which is a command below the root with subcommands, and 1,200 for a leaf, which is a command below the root without subcommands.
- A tool can lower a budget with [`Budgets`](../../../../crates/crumbkit-test/src/sizes.rs) and cannot raise one.
- `check_help_goldens` compares each page with a golden file through snapbox (ADR-0004). It fails on a stale file, so the directory holds only help golden files.
- `check_guide` holds the leaf rules (ADR-0024).
- crumbkit-test has no bundled check that runs every check. Each check has its own settings and a tool calls each from its own `#[test]`, so a failure names the rule that broke. By the same rule, `check_document_ids` and `check_mounts` are separate checks (ADR-0035): a wrong document name is fixed by renaming the document, and a crumbkit command at the wrong path is fixed by moving the command.

Considered: measuring clap's `render_help` after `extend`, a plain `&[(path, bytes)]` slice of node budgets, a budget on `--help`, leaf rules in a separate `check_leaves`, a bundled `check(command, guide)`.
