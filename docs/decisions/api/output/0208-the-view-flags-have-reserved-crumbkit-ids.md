# ADR-0208. The view flags have reserved crumbkit clap ids

- Date: 2026-11-12
- Status: accepted
- Supersedes: ADR-0115, which it amends

## Context

`ViewFlags` gave `--json`, `--fields`, `--long` and `--full` the clap ids
`json`, `fields`, `long` and `full`. A tool could declare its own argument
with one of those ids, so every read of a view flag in crumbkit and
crumbkit-test first tested that the argument was global and had the type
that `ViewFlags` gives it. crumbkit's other reserved flags already had
reserved ids, such as `crumbkit_dry_run`. A `Call` named a view flag by a
string, such as `.arg("json")`, and nothing checked that string.

## Decision

- The view flags have the clap ids `crumbkit_json`, `crumbkit_fields`,
  `crumbkit_long` and `crumbkit_full`. Their long names and help output do
  not change.
- The public constants `ViewFlags::JSON_ID`, `ViewFlags::FIELDS_ID`,
  `ViewFlags::LONG_ID` and `ViewFlags::FULL_ID` hold the ids. A call names
  a view flag with them, as in `.arg(ViewFlags::JSON_ID)`, and a tool that
  declares the flags by hand gives its arguments these ids. crumbkit and
  crumbkit-test read the same constants.
- `parse` reads a view flag by its id alone, without the test that the
  argument is global.
- `Refused::flags` and `Refused::value` report a typed view flag by its
  new id, such as `crumbkit_json`.
- The document rule of `check_help` applies to a leaf whose command
  declares an argument with one of the ids, or whose ancestor declares one
  global. This amends ADR-0115, which named the old ids.

## Consequences

A tool that flattens `ViewFlags` changes nothing. A tool that declares the
flags by hand, or names one in a `Call` by its old id, reads the flag as
off until it moves to the constants, and the changelog says so. A tool's
own argument with the id `json` parses beside the view flags. It cannot
take the long name `--json` on a command that the global flag reaches,
because clap's debug asserts panic on two arguments with one long name;
before, it hid the global flag on its command.
`docs/behaviour/output/README.md` and `docs/behaviour/parsing/README.md`
describe the current behaviour.

Considered: keeping the ids and the global test that every read repeats;
and a `Call` method that names a view flag, which adds a second way to
name an argument.
