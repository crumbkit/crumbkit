# Pickers and interactive input

This page covers the questions that crumbkit asks a person: a picker that
fills a missing required argument, how crumbkit decides that a person can
answer, and the `--no-input` flag that turns questions off.
[`crumbkit::pick`](../../../crates/crumbkit/src/pick.rs) describes `Picker`,
`Question`, `Answer` and `Choices`.

## Registering a picker

A person who leaves out a required argument often does not know its values,
such as the id of a book. A tool can let that person pick the values from a
list instead of reading a refusal and running a second command. An agent
cannot answer a question, so every question that crumbkit asks first checks
that a person can answer, and a caller can turn questions off.

A tool registers one picker with `Guide::picker`, a `const fn` like the
guide's other builders, so the picker costs nothing until clap refuses a
command line. The picker is a `&'static dyn Picker`, which keeps `Guide` a
constant and lets a tool use any picker without crumbkit depending on it.
crumbkit checks every position and the count that a picker returns, so a
picker cannot add a value that the list does not hold.

Why: ADR-0043.

## Choices

A picker offers the visible possible values of the argument, with each
value's help as its label. An argument without possible values gets its
choices from the source that its leaf names with `Leaf::choices` and
`Choices::new`, a function that returns the choices or fails with a crumbkit
`Error`. Possible values win when an argument has both. `pick::from_rows`
turns a command's rows into choices: the value is the named column, and the
label is the row's other default columns, with each control character
escaped. A tool usually reads the same rows as its `list` command.
crumbkit-test's `check_guide` fails choices that name no argument of their
leaf.

Why: ADR-0043.

## When crumbkit asks

`parse`, `parse_from` and `parse_to` pick when clap refuses a command line
because a required argument is missing and the guide has a picker. crumbkit
walks the typed words to the leaf and finds the first required argument that
the words leave out. It refuses as before when the words name no leaf, when
any missing argument has neither possible values nor a source, or when the
picker was already asked about the argument in this run. Otherwise it asks,
adds the picked values to the words, and parses again, until clap accepts the
words or nothing more can be picked.

- An option's picked values go before any `--`, each as `--long=value`, so a
  value that starts with `-` stays a value. A positional argument's values go
  at the end, after a `--` that crumbkit adds when a value starts with `-`.
- A run whose values were picked prints `Running:` and the whole command line
  on standard error before the command runs, so the person learns the
  command to type next time. The `Output` that `parse` returns holds the words with the picked values,
  so the `--yes` recovery of `confirm` repeats them
  ([confirmation](README.md#confirmation)).
- A cancelled question, or an answer that names no valid choice, prints the
  ordinary refusal with its exit code. A source that fails prints its failure,
  and the run exits with the code that the guide gives the failure, 1 by
  default ([exit codes](../failures/exit-codes.md)).
- A run with `--json` still picks, because the question goes to standard
  error and the result format is unchanged.

crumbkit treats an argument as required only when clap marks it `required`.
An argument that `required_unless_present`, `required_if_eq` or a required
group requires is refused without a question. crumbkit does not pick a
missing subcommand or ask for free text.

The usage log's run records `picked`, the number of arguments that the
picker filled, and records none of the picked values
([usage log](../usage-log/README.md)).

Why: ADR-0043, ADR-0029.

## Who can answer

A person can answer when standard input and standard error are both terminals
and no agent is detected. [`agent`](../../../crates/crumbkit/src/agent.rs)
detects an agent with is-ai-agent, which reads the `AGENT` and `AI_AGENT`
variables and the variables that individual agents set.
`Sink::standard_with_input` makes that decision, and it runs only when a
question is about to be asked. The usage log reads the same detection for
its `caller` field. A person typing in a terminal that an agent started
inherits the agent's variables and gets no question. A tool whose guide turns
[terminal detection](../output/terminal.md) off never asks.

Why: ADR-0043, ADR-0049.

## Turning questions off

[`NoInput`](../../../crates/crumbkit/src/pick/no_input.rs) adds a global
`--no-input` flag that a tool flattens at its root, as it flattens `Yes`.
With `--no-input`, crumbkit refuses a missing argument without asking, and
`confirm` fails with `<tool>::confirmation::needs_yes` without asking.
crumbkit finds the flag by its argument id, so a tool can rename it with
`Command::mut_arg` and then opts out of `Rule::NoInput`, because that rule
gives `--no-input` one name in every tool.

crumbkit-test's `check_effects` fails a tool that can ask a question, because
its guide has a picker or a command takes `--yes`, and that has no
`--no-input` from `NoInput` at its root. A tool opts out of `Rule::NoInput`
with a reason, and the opt-out report says that an agent with a terminal then
cannot turn questions off ([opt-outs](../adoption/opt-outs.md)).

Why: ADR-0043, ADR-0036.

## `Menu`

[`Menu`](../../../crates/crumbkit/src/pick/menu.rs) is crumbkit's own
picker, `crumbkit::pick::Menu`, built on the standard library alone. It
prints a numbered list, reads one line, narrows the list when the line is
text, and cancels on an empty line or the end of input. It shows 20 choices
at a time by default and says how many more match. A tool keeps one `Menu`
in a `static` and sets it with `Guide::picker`, so a tool with a picker
depends on crumbkit alone. A tool that wants arrow keys or fuzzy search
wraps another prompt library in its own `Picker`, and that picker lives in
the tool or in a crate of its own, so crumbkit depends on no terminal
library.

Why: ADR-0043, ADR-0095, ADR-0003.
