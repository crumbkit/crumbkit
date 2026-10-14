# ADR-0025. Help checks

- Date: 2026-10-10
- Status: accepted

## Context

Agents read `-h` pages first and most often, and every byte of a page is
context the agent pays for. Help pages grow one line at a time, so a tool
needs a budget that fails a test when a page or a path through the help
gets too large, and golden files that show every change to a page.

## Decision

- The help checks read the pages crumbkit prints by running `parse_to` in
  memory, so a page is the plain text a caller reads on a pipe. A tool
  that builds help at run time passes `Pages::binary`, and the checks run
  its binary.
- A budget counted the bytes of the `-h` page, because agents read it
  first and most often. `--help` has no budget, because a caller asks for
  it to read the full descriptions. The defaults are 1,500 bytes for the
  root, 800 for a group and 1,200 for a leaf.
- `check_path_weights` budgeted each path: every `-h` page from the root
  down plus the command's largest example output, 8,000 bytes by default,
  about 2,000 tokens. A depth-2 leaf at its default page budgets leaves
  4,500 bytes for output. It shares its sums with `cargo crumbkit weigh`,
  so the two agree.
- A tool could raise or lower any budget with a reason, and the opt-out
  report lists it, so crumbkit's defaults fit most tools and a tool with
  larger pages can still adopt the checks.
- `check_help_goldens` pinned each page in a golden file and fails on a
  stale file.
- The help checks skip hidden commands, because no page shows them.
- crumbkit-test had no bundled check that runs every check, so a failure
  names the rule that broke and each check keeps its own settings.

## Consequences

shelf lowers its path budgets to 4,300 bytes. `docs/behaviour/help/checks.md`
describes the current checks.

Considered: measuring clap's `render_help` after `extend`, a plain `&[(path, bytes)]` slice of node budgets, a budget on `--help`, leaf rules in a separate `check_leaves`, a bundled `check(command, guide)`, path budgets in `Budgets` and `check_help_sizes`, a budget on output weight alone, a default path budget of 6,000 bytes, a budget that a tool cannot raise, a separate binary function for each help check, a binary setting on `Budgets`.
