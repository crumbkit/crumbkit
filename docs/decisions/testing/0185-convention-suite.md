# ADR-0185. One suite runs every convention check

- Date: 2026-11-09
- Status: amended by ADR-0186
- Supersedes: ADR-0036, which it amends

## Context

Each tool that adopted crumbkit-test wrote its own list of convention tests,
between 7 and 20 of them, and each test called one `check_*` function with the
same inputs as the others. A check added to crumbkit-test reached no tool until
someone added a test by hand. A tool that left a check out recorded no reason.

## Decision

`crumbkit_test::Suite` holds the inputs of every check. It composes the records
that the checks already take: `Allow`, `Budgets`, `PathBudgets`, `Examples`,
`Recoveries` and `Twins`, with the code lists and the file locations.
`crumbkit_test::conventions!` expands a `Suite` to one `#[test]` for each
`Check`, named as a sentence.

A `Check` variant and its test name sit in one table, which declares both the
enum and the macro, and `Suite::run` matches every variant. A check added to the
table therefore reaches every tool that uses the macro with no edit to the tool.

Five inputs have no default, because no value suits every tool: `.path_budgets`,
`.examples`, `.codes`, `.recoveries` and `.twins`. A check that needs one and
does not have it fails with a message that names the setter, so a check that
needs a new input fails the same way until the tool sets it. Other inputs have
defaults: no opt-outs, no extra verbs, crumbkit's default budgets and
`tests/golden`. The command reference is always `docs/commands.md`, where
`cargo crumbkit docs` writes it.

`Suite::skip` leaves out one check with a reason, which a `const` rejects at
compile time when it is empty or holds a line break. ADR-0036 rejected an
exemption that skips every rule of a check, because it would hide other
problems and outlive its reason. A whole-check skip is acceptable now for two
reasons. The macro runs each new check on every tool with no edit, so a tool
needs a way to decline a check that cannot apply to it. And each skip is listed
under the rule `suite/skip` in the report that `check_opt_outs` pins, so a
reviewer sees a skip appear or change in the golden files. A stale skip is not
detected, unlike a stale `Allow`, because finding one means running the check
that the tool chose not to run, which can write its golden files under
`SNAPSHOTS=overwrite` and does the slow or impossible work the skip avoids.

`Suite::new` takes one function that returns the command tree and the tool
record, which also serves a tool that builds them at run time. With `.examples`
set, the help checks read what the binary prints. Without it, they read pages
rendered in the test. The `check_*` functions stay public, and a tool may still
call them by hand.

## Consequences

A tool's convention tests are one constant and one macro call, and a skipped
check has a written reason. When crumbkit-test gains a check, a tool's tests
can fail after an upgrade with no change to the tool, and the failure says what
to set or skip.

Considered: one `Suite::check` method called from one test, which needs no
macro but hides later failures behind the first and stops `cargo test` from
selecting one check; a documented template of test functions, which leaves a
new check out of every tool until someone copies it; a required setter for
every input, which adds lines to every tool for values that all current tools
share.
