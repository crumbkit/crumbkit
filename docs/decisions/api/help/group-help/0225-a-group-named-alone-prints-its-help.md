# ADR-0225. A group named alone prints its help

- Date: 2026-11-16
- Status: accepted
- Supersedes: ADR-0118, which it amends

## Context

ADR-0118 made a command line that stops at a group print the group's `-h`
page on standard error and exit with the usage exit. A person who types a
group's name, such as `tool workspace`, is asking what the group holds, and
a refusal exit makes a script or a shell prompt report a failure for that
question.

## Decision

- A command line that stops at a group with visible subcommands, below the
  root, prints the group's short `-h` page on standard output and exits 0,
  as `tool workspace -h` does. The usage log records the run as a help run,
  as it records `-h`. With `--json`, the line prints what `-h --json` prints.
- The bare root, and a group followed by `--`, stay refusals as ADR-0118
  decided: the page on standard error, the failure document with `--json`,
  and the usage exit. For a tool whose `Help::root` names a subcommand, that
  subcommand named alone counts as the bare root.
- A group whose help flag the tool turned off stays a refusal.

## Consequences

The parsing, failures and end-to-end check pages describe the change. A
line that stops at a group and prints a page with code 2 now needs a `--`
after the group, so the twin check reads the code of such a line from a
`--json` run only for that form.

Considered: printing the page on standard output and keeping the usage exit,
which still reports a failure for a question; counting a group followed by
`--` as a help request, which hides the empty positional that `--` adds.
