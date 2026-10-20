# ADR-0123. A refusal recovers with the tool's hint

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0078, which it amends
- Missed: ADR-0078 let a leaf or the guide declare only calls for a refusal, so a tool whose next step is a sentence, such as where to put the words that a command passes to a program, could not make that sentence the recovery.

## Context

A command can pass the words after `--` to a program that it runs, as a
clap positional with `last = true` does. A caller who types the program's
flag before `--` is refused for an unknown argument, and the step that
repairs the line is to move the flag after `--`. That step is an
instruction, and the refused line with the flag moved is only one of the
lines that it allows, so a tool that wants to say it needs a sentence.
A leaf or the guide could declare `refusal_alternatives`, which returns
calls, and no function returned a sentence. An adopting tool printed the
sentence in its own failure document, and crumbkit's text form left it
out.

## Decision

- `Leaf::refusal_hint` and `Guide::refusal_hint` take a function from a
  `RefusalContext` to an optional sentence. A leaf's function replaces the
  guide's for a command line that reaches the leaf, as a leaf's
  alternatives function does.
- When the function returns a sentence, it becomes the recovery as
  `Recovery::Hint`. The `-h` call becomes the first alternative, followed
  by the calls that crumbkit keeps from the alternatives function, in the
  order that the function returned them. The text form prints the sentence
  on the first `help:` line, the failure document prints it as a `hint`,
  and `Refused::recovery` returns it.
- When the function returns `None`, or no function is set, the refusal
  recovers as ADR-0078 decided.
- A refused `--limit` above its bound keeps its `--all` recovery. The
  sentence follows crumbkit's own alternatives and comes before the kept
  calls.
- `RefusalContext::command` returns the leaf's command after a build, so a
  function can test `get_positionals().any(Arg::is_last_set)` beside
  `kind()`.

## Consequences

`docs/behaviour/failures/README.md` describes the order. crumbkit-test's
`check_opt_outs` lists a `refusal-hint` customisation for the guide and
for each leaf that declares a function. No example tool has a command that
passes the words after `--` to a program, so no example declares a hint.

Considered: an alternatives function that returns `Recovery` values, which
would change the signature of every existing function; a sentence that
crumbkit words itself for every command with a `last` positional, which
cannot name the program that the tool runs; the sentence as an
alternative after `-h`, which leaves an agent that reads only the
recovery running help; the sentence in place of the `--all` recovery of a
`--limit` above its bound.
