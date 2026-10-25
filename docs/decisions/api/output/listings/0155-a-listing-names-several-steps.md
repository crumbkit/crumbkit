# ADR-0155. A listing can name several next steps, with commands of other programs

- Date: 2026-10-25
- Status: accepted
- Supersedes: ADR-0050, which it amends

## Context

`Listing::next` names one command of the tool and a phrase after it, and
the text prints them on one `Next:` line. An adopting tool has a command
that ends with several steps for the reader. Each step is a sentence
followed by the commands that carry it out, and some of those commands
belong to other programs, such as `git` or a version manager. The tool
could only print such steps in its note as free text, which crumbkit
neither formats nor checks.

A free command line for another program would let a tool name a program
on the spur of one step. Every new program that a tool's guidance names is
a program its readers must have, so a tool's author should decide that once
and on purpose.

## Decision

- `Listing::step` adds a `Step` after the steps already added. The text
  prints each step on standard error after the `Next:` line of
  `Listing::next`, in the order added, as the line `Next: {sentence}`
  followed by each of its commands on its own line, indented by two
  spaces. A step without commands prints its `Next:` line alone.
- `Step::new` takes the sentence. `Step::call` adds a command of the tool
  itself as a `Call`, which renders against the tool's command tree as
  `Listing::next` does. `Step::run` adds a command of another program as a
  `Program` and its arguments.
- A `Program` is the only way to name another program. A tool declares it
  once, as a constant such as `const GIT: Program = Program::new("git");`.
  `Program::new` is a `const fn` that panics, and so stops the build in a
  constant, when the name is not one word of ASCII letters, digits, `-`,
  `_`, `.` and `+` that starts with a letter or a digit. A name therefore
  holds no arguments and no path, and a step never holds a free command
  line.
- Each argument of another program prints as a literal word, quoted for a
  POSIX shell when it would be split or expanded, as a `Call`'s values
  are. crumbkit cannot read the other program's usage, so it offers no
  placeholders for it.
- A step whose `Call` does not resolve is left out whole, so a step never
  prints with one of its commands missing. The run's exit does not change.
- The JSON document leaves the steps out, as it leaves out the note and
  the `Next:` line (ADR-0016). `Listing::next`, its text and every JSON
  document are unchanged.
- The API-1 ground is one adopter's request for an ordinary need. Any tool
  whose work hands over to other programs, such as a setup or release
  command, ends with steps that name them (ADR-0122).

## Consequences

A tool names a sequence of steps without writing them into its note, and
crumbkit formats them alike in every tool. Adding a program to a tool's
guidance takes a declaration that a reviewer sees. A program and its
arguments are not checked, so a step can still name a command that the
other program refuses. A program for agents that reads `--json` does not
see the steps. `docs/behaviour/output/listings/README.md` describes the
current behaviour, and the shelf example ends the listing of an emptied
shelf with a step.

Considered: a free string for each command, which lets a tool name any
program without deciding to; programs registered on the run's `Output`
before parsing, whose handles exist only at run time, so a handler would
need the run's `Output` to build a step; a `Call` variant for other programs, which would
let a recovery name another program too; steps in the JSON document under
a new field, which would change every listing schema for text that no
program has asked to read; placeholders in another program's arguments;
and making `Listing::next` a step, which would change its text.
