# ADR-0125. A tool chooses the form of a refusal it prints

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0100, which it amends
- Missed: ADR-0100 let a tool print a refusal through `Refused::print` and `Refused::print_to` only in the form that the walk of the words chose, and missed a tool whose own flag chooses the form of a failure, which could not make crumbkit print text when the words held `--json`, or JSON when they did not.

## Context

ADR-0100 made `try_parse` return a `Refused` that a tool prints with
`Refused::print` or `Refused::print_to`. Both printed the
`<tool>/failure/v1` document on standard output when the walk of the words
found `--json`, and text on standard error otherwise. ADR-0109 let a tool
read a global option's value from the refusal with `Refused::value`.

An adopting tool has a global flag of its own, `--error-format text` or
`--error-format json`, that chooses the form of every failure. It reads
the flag with `Refused::value` and resolves the form. For `tool
--error-format text list --nosuch --json`, crumbkit then printed a JSON
document on standard output although the tool's flag asked for text on
standard error. The tool had no way to pass the form it resolved, short of
printing the refusal itself from the message, the code and the recovery,
which loses the usage line and the group page that crumbkit prints.

A flag of the tool's own that chooses the form of its failures is an
ordinary need that other tools are likely to share, so one adopter's
request meets API-1 on the third ground of ADR-0122.

## Decision

- `Refused::as_json(self, json: bool) -> Self` fixes the form in which the
  refusal prints. With `true`, it prints one `<tool>/failure/v1` document
  on the output writer. With `false`, it prints text on the error writer.
  The words do not change that form.
- Without the call, the refusal prints in the form that the words chose,
  as before.
- `Refused::print` and `Refused::print_to` honour the choice. `Unparsed`
  gains no method. A tool matches `Unparsed::Refused(refused)` and calls
  `refused.as_json(..)`.
- `Refused::output` keeps reporting the output flags as the walk found
  them, so `Output::is_json` on it still says whether the caller typed
  `--json`.
- The `Stop` that printing returns holds an `Output` whose `is_json`
  reports the form that printed, so a tool that prints more after the
  refusal follows the same form.
- The exit code and the observers' events are the same in either form.

## Consequences

A tool with its own failure-format flag prints crumbkit's refusal in the
form that its flag chose, and keeps crumbkit's usage line, group page and
recovery. `Output::is_json` on a refusal's `Stop` no longer always agrees
with `Output::is_json` on the `Refused` that printed it, and
`docs/behaviour/parsing/README.md` says when they differ.

Considered: a `Guide` setting that names the tool's flag and its values,
which would encode one tool's flag in crumbkit; an argument on
`Refused::print_to`, which would change the signature that every tool
calls; changing what `Refused::output` reports, which would hide whether
the caller typed `--json`; leaving `Stop` reporting the words, which would
make a tool that prints more after the refusal switch form.
