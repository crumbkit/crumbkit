# ADR-0090. Opt-outs for one option

- Date: 2026-10-19
- Status: accepted
- Supersedes: ADR-0036, which it amends

## Context

ADR-0036 made an opt-out name one rule and one command, with the commands
below it. The two filter rules, `filters/repeats-and-commas` and
`filters/one-value-per-use`, each find a problem in one option of a
command. A second adopting tool had a command with two options that broke
the repeats-and-commas rule, so its one opt-out gave both options one
reason. That opt-out also excused any option that the tool added to the
command later.

## Decision

- `Allow::option(rule, path, option, reason)` opts one option of the
  command at `path` out of a filter rule. It names the option as help
  does, such as `--author`, and covers no other option and no command
  below `path`.
- Only the two filter rules take it, because their problems are each one
  option's. A `const` that names another rule fails to compile.
- `check_filters` fails on an option opt-out whose command has no such
  option, as on one that excuses no problem.
- `check_opt_outs` names the opt-out by its command line and option, such
  as `shelf books list --author`.
- An opt-out from `Allow::new` still covers every option of its command
  and of the commands below it.

## Consequences

A tool gives each option its own reason, and a new option on the same
command meets the filter rules unless the tool opts it out too. The
report's entries name the options that depart from the conventions.
`docs/behaviour/adoption/opt-outs.md` describes the current behaviour.

Considered: an option opt-out for every rule, an option opt-out that also
covers the commands below its command, naming the option by its clap id.
