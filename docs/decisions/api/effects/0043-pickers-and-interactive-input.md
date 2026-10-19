# ADR-0043. Pickers and interactive input

- Date: 2026-10-12
- Status: amended by ADR-0095

## Context

A person who leaves out a required argument often does not know its
values, such as the id of a book, and must read a refusal and run a
second command to find them. An agent cannot answer a question, and a
question asked of an agent hangs the run.

## Decision

- A tool registered one `Picker` on its guide, a `&'static dyn Picker`,
  so `Guide` stays a constant, the picker costs nothing until clap
  refuses a line, and crumbkit depends on no prompt library. crumbkit
  checks every position the picker returns, so a picker cannot add a
  value the list does not hold.
- A picker offered an argument's possible values, or the choices from a
  source that its leaf names, usually the rows of the `list` command.
- `parse` picked only when clap refuses a line for a missing `required`
  argument, asked about each argument at most once, and parsed again
  with the picked values. Other refusals stay refusals, and picking a
  subcommand or free text was left out of scope.
- A run with picked values printed `Running:` and the whole command line
  on standard error, so the person learns the command to type next time.
  A cancelled question prints the ordinary refusal.
- crumbkit asked only when standard input and standard error are both
  terminals and no agent is detected through is-ai-agent. Detection runs
  only when a question is about to be asked.
- `NoInput` added a global `--no-input` flag, and `check_effects` fails a
  tool that can ask a question without it, so an agent with a terminal
  can always turn questions off.
- The usage log recorded how many arguments were picked and never the
  values.
- The separate crumbkit-picker crate held `Menu`, a picker built on the
  standard library alone. A tool that wants arrow keys wraps another
  prompt library in its own `Picker`.

## Consequences

A person typing in a terminal that an agent started inherits the agent's
variables and gets no question. `docs/behaviour/effects/pickers.md`
describes the current behaviour.

Considered: a picker that opens the terminal itself, a picker that returns value strings, the trait in crumbkit-core, `parse_with` entry points that take a picker, a picker held on `Sink`, an ids source for each noun found by the value name `ID`, running the tool's own `list` command as a child process, a second parse with clap's `ignore_errors`, reading the missing arguments from clap's error context, a `cancelled` failure with exit code 1, no echo of the command line, detecting only `AGENT` and `AI_AGENT`, detection behind a feature, `--no-input` as a field of `Output`, `--no-input` answered before clap, the `NoInput` rule only for tools with a picker, a terminal library in crumbkit-picker.
