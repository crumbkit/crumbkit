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
read the group's help before it could type the next word. The names of the
group's visible subcommands are short and already known when crumbkit
builds the message.

## Decision

- The message of a command line that stops at a group names the group's
  visible subcommands after a colon, separated by `, `, in the order that
  the tool declares them, as in
  ``the command `shelf books` needs a subcommand: list, show, summarize, remove``.
- A hidden subcommand stays out of the message, as it stays out of help.
  clap's `help` subcommand never appears, because `parse` disables it.
- The message of a leaf that needs arguments, the recovery, the code and
  the fields did not change.

## Consequences

`docs/behaviour/failures/README.md` describes the message. shelf's golden
file for `shelf books` holds the new line. A group with many subcommands
prints a long first line, which the group's `-h` would also print.

Considered: the names under the goal headings that a compact group's `-h`
prints, the names in a field of the failure document, the names as
alternatives that run each subcommand's `-h`.
