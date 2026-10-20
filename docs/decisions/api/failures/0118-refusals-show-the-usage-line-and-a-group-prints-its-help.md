# ADR-0118. Refusals show the usage line and a group prints its help

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0001 and ADR-0106, which it amends
- Missed: ADR-0106 listed a group's subcommands on one line, and a person exploring wants the group's page with each one's purpose. It also dropped clap's usage line.

## Context

A review found an adopting tool's text refusals worse for a person after
crumbkit than before.

## Decision

- A command line that stops at a group prints the group's `-h` page on
  standard error and exits with the usage exit.
- A refusal from clap prints clap's usage as `usage:` lines under `error:`.
- The failure document does not change.

## Consequences

The failures and parsing docs describe both. Each clap refusal golden gains
a usage line.

Considered: a `usage` field, which widens the schema to repeat the recovery;
a code line under the page, which clap omits; the subcommand list above the
page.
