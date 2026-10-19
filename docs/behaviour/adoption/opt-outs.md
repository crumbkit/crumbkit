# Opt-outs

This page covers how a tool departs from one of crumbkit's conventions for a
stated reason, and the report that pins every departure.
[`Allow`](../../../crates/crumbkit-test/src/allow.rs) and
[`check_opt_outs`](../../../crates/crumbkit-test/src/opt_outs.rs) describe
the items themselves.

## An opt-out

crumbkit's conventions are the defaults, and a tool can opt out of a rule
with a stated reason. An opt-out is an `Allow` that names one `Rule`, the
command it covers with every command below it, and the reason. Each check
that takes opt-outs reads only those of its own rules, so a tool keeps one
list and passes it to every such check.

An opt-out skips one rule, or with `Allow::example` one example, or with
`Allow::untwinned` the twin of one code. With `Allow::option`, an opt-out
from a filter rule skips one option of one command, so two options of a
command each give their own reason. The other rules of the same check
still apply to the command. A check fails on an opt-out of its rules that
names no command, names no option of its command or excuses no problem, so
the opt-out of a removed or fixed command or option cannot stay in a tool's
list.

Why: ADR-0036, ADR-0090.

## Rules that take an opt-out

Each rule is a variant of `Rule`, and the report names it by its id. Each
rule's page gives its reason.

| Rule | Id | When a tool needs it |
| --- | --- | --- |
| `RootLeaf` | `grammar/root-leaf` | a leaf sits directly under the root ([checks](../checks/README.md)) |
| `ResourceName` | `grammar/resource-name` | a resource name is not lowercase dashed words and answers to no [dashed form](declarations.md#dashed-forms) |
| `OneName` | `names/one-name` | a built-in alias shows in help ([aliases](../parsing/aliases.md)) |
| `PersonalAliases` | `names/personal-aliases` | the root's help names a person's aliases ([aliases](../parsing/aliases.md#personal-aliases-in-help)) |
| `ListPages` | `listings/list-pages` | a `list` command lacks `Page`'s flags ([checks](../checks/README.md)) |
| `ShowIds` | `listings/show-ids` | a `show` command takes one id |
| `RepeatsAndCommas` | `filters/repeats-and-commas` | an option with many values does not take both repeats and commas |
| `OneValuePerUse` | `filters/one-value-per-use` | an option takes several values per use |
| `Examples` | `examples/run` | an example is skipped ([end-to-end checks](../checks/end-to-end.md)) |
| `GuideExamples` | `guide/examples` | a visible command without subcommands has no example yet ([leaf checks](../help/examples.md#checks-of-leaves)) |
| `Envelope` | `documents/envelope` | a command keeps the shape of an existing document with `Envelope::Without` ([result shapes](../output/shapes/README.md#documents-without-the-envelope)) |
| `RowsWhole` | `documents/rows-whole` | a command's row type sets `Row::WHOLE`, so `--json` prints every field of each row ([result shapes](../output/shapes/README.md#rows-printed-whole)) |
| `Effects` | `effects/declared` | a visible leaf declares no effects ([effects](../effects/README.md)) |
| `DryRun` | `effects/dry-run` | a tool keeps its own `--dry-run` |
| `Yes` | `effects/yes` | a destructive command has no `--yes` |
| `NoInput` | `effects/no-input` | a tool that can ask a question has no `--no-input` at its root ([pickers](../effects/pickers.md#turning-questions-off)) |
| `Expect` | `effects/expect` | a replacing leaf has no `--expect` ([write guards](../effects/write-guards.md)) |
| `ExpectFlag` | `effects/expect-flag` | a command has its own `--expect` |
| `Items` | `effects/items` | a leaf that writes many items declares neither `all_or_none` nor `per_item` |
| `ShellExitCode` | `exit-codes/shell-code` | a tool declares an exit code from 126 to 255 ([exit codes](../failures/exit-codes.md#checks)) |
| `SharedExit` | `exit-codes/shared-exit` | a tool declares the exit code 1 or 2 |
| `Recoveries` | `recoveries/follow` | the check cannot read a failure that a run printed ([failure readers](../failures/excerpts-and-readers.md#reading-failures-back)) |
| `SameCode` | `recoveries/same-code` | a command that a recovery runs fails on purpose with a code its chain already had |
| `FieldValue` | `recoveries/field-value` | a recovery takes a placeholder's value from a field of its failure |
| `NextStep` | `recoveries/next-step` | a failure with a declared code offers no `run`, `retry` or `hint` step |
| `NoTwin` | `twins/no-twin` | no single word of a command line causes a declared code |

An opt-out made with `Allow::untwinned` from `Rule::NoTwin` names the failing
line in place of a command path, `check_twins` runs that line, and
`check_opt_outs` reports it by that line. An opt-out made with
`Allow::option` is reported by its command line and option, such as `shelf
books list --author`. The [recovery
chains](../checks/recovery-chains.md) page covers the `recoveries/*` rules.

Why: ADR-0036, ADR-0015, ADR-0053, ADR-0005, ADR-0065, ADR-0071, ADR-0090, ADR-0104.

## Checks without opt-outs

- The rule that reserves `help` has no opt-out, because `parse` answers
  `help` itself.
- A tool declares its verbs to `check_grammar`.
- `check_guide` takes an opt-out for its example rule alone. A tool that
  wants clap's list for a command leaves the command out of its guide, and
  each other problem of the guide is a name or a declaration that the tool
  can correct.
- The help checks take budgets in place of opt-outs
  ([help checks](../help/checks.md)).
- `check_mounts` exists to keep crumbkit's commands at fixed paths.
- `check_codes` keeps failure codes unique.
- The name rule of a failure's fields has no opt-out, because the failure
  document's schema promises it to every reader.

Why: ADR-0036, ADR-0015, ADR-0081, ADR-0104.

## The opt-out report

A tool pins its opt-outs in a report with `check_opt_outs`, which compares a
text and a JSON golden file. Each entry names the rule, the command line, the
reason and what skipping the rule costs an agent. The report also lists each
help or path budget that the tool set and the guide's
[output budget](../output/budget/README.md), as `output-budget/run`, and
each guide setting that departs from the conventions with a reason:

- a [JSON layout](../output/json-layout.md), as `json-layout/pretty`;
- [terminal detection](../output/terminal.md) turned off, as
  `terminal-detection/off`.

The text report's first line counts the opt-outs, and counts the budgets
apart from them when the tool sets any, such as `shelf has 3 opt-outs from
the conventions and sets 1 budget.` A budget is an entry with a `Budget:`
line: a help, path or output budget.

A reviewer then sees a new opt-out as a change to the golden files, and an
agent reads the JSON to learn where the tool departs from crumbkit's
conventions. After the opt-outs, the report lists the tool's
[declarations](declarations.md#the-customisations-section) under the
customisations.

The report is a test check only, because a tool's opt-outs live in its
tests. Every other check runs without it, so a tool turns checks on one at a
time and adds the report when it wants its opt-outs pinned.

Why: ADR-0036, ADR-0025, ADR-0046, ADR-0049, ADR-0052.
