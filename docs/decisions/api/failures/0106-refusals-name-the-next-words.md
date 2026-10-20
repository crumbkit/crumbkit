# ADR-0106. Refusals name the next words

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0001, which it amends

## Context

A refused command line fails with a one-line message, a recovery and a
code (ADR-0001). A command line that stopped at a group printed
``the command `shelf books` needs a subcommand``, and its recovery ran
`shelf books -h`. An agent that reads the message learned that a word was
missing but not which words the group accepts, so it ran the recovery to
read the group's help before it could type the next word. A command line
that left out a required argument with possible values printed
`the following required arguments were not provided: <SHELL>`, although
the refusal of an invalid value for the same argument listed the possible
values. The names of a group's visible subcommands and an argument's
possible values are short and already known when crumbkit builds the
message.

## Decision

- The message of a command line that stops at a group names the group's
  visible subcommands after a colon, separated by `, `, in the order that
  the tool declares them, as in
  ``the command `shelf books` needs a subcommand: list, show, summarize, remove``.
- A hidden subcommand stays out of the message, as it stays out of help.
  clap's `help` subcommand never appears, because `parse` disables it.
- The message of missing required arguments lists the visible possible
  values of each missing argument right after its name, in the form that
  clap prints for an invalid value, as in
  `<SHELL> [possible values: bash, fish, zsh]`. crumbkit finds the argument
  on the deepest typed command by comparing clap's name for it with the
  argument's `Display`. A missing argument without possible values is
  named alone.
- The message of a leaf that needs arguments, the recovery, the code and
  the fields did not change. A picker still offers the possible values
  when one runs, so the message changes only when the run refuses.

## Consequences

`docs/behaviour/failures/README.md` describes both messages. shelf's
golden file for `shelf books` holds the new line. A group with many
subcommands or an argument with many possible values prints a long first
line. The possible values come from clap's layout of the refusal, one
missing argument to a line, which clap does not promise to keep, and the
tests of `parse` fail when it changes.

Considered: the names under the goal headings that a compact group's `-h`
prints, the names and values in a field of the failure document, the names
as alternatives that run each subcommand's `-h`, a message built from the
error's context alone in clap's wording.
