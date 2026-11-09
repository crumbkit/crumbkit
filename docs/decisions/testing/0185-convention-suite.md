# ADR-0185. One suite runs every convention check

- Date: 2026-11-09
- Status: accepted

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
defaults: no opt-outs, no extra verbs, crumbkit's default budgets, `tests/golden`
and `docs/commands.md`.

`Suite::skip` leaves out one check with a reason, which a `const` rejects at
compile time when it is empty or holds a line break. `Suite::new` takes the
function that returns the command tree and the tool record, and `Suite::built`
takes one function that builds both, for a tool that builds them at run time.
The `check_*` functions stay public for a tool that needs one alone.

## Consequences

A tool's convention tests are one constant and one macro call, and a skipped
check has a written reason. When crumbkit-test gains a check, a tool's tests
can fail after an upgrade with no change to the tool, and the failure says what
to set or skip.

`Suite::skip` reasons are not part of the report that `check_opt_outs` pins.

Considered: one `Suite::check` method called from one test, which needs no
macro but hides later failures behind the first and stops `cargo test` from
selecting one check; a documented template of test functions, which leaves a
new check out of every tool until someone copies it; a required setter for
every input, which adds lines to every tool for values that all current tools
share.
