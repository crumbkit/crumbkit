# ADR-0107. Refused exposes clap's error

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0100, which it amends
- Missed: ADR-0100 gave `Refused` getters for the words and the culprit and missed one for clap's error, which a tool needs to word the refusal of a conflicting or repeated argument itself.

## Context

ADR-0100 made `try_parse` return a `Refused` that holds clap's error, and
gave it getters for the message, the code, the recovery, the words and the
culprit. A tool that prints a refusal in a sentence of its own needs to
know what kind of refusal clap made and which argument it named. For a
conflicting or repeated argument, `culprit` names a flag at most, and
`message` holds clap's English sentence. The tool could learn the kind
and the other argument only by parsing the command line again with clap.

## Decision

- `Refused::error` returns `Option<&clap::Error>`: clap's error, or `None`
  for a refused view flag, which crumbkit refuses itself after clap
  accepted the command line.
- A tool reads the error's `kind()` and its `get(ContextKind)` values,
  which clap fills through its `error-context` feature. crumbkit already
  enables that feature (ADR-0003).

## Consequences

A tool can word the refusal of a conflicting or repeated argument from
clap's `ErrorKind::ArgumentConflict` and `ContextKind::InvalidArg` without
a second parse. clap's error type becomes part of crumbkit's public API
through this getter, so a major version of clap that changes `clap::Error`
is a breaking change for crumbkit. crumbkit already exposes clap's
`Command` and `ArgMatches`, so the getter adds no new dependency for a
tool.

Considered: a crumbkit enum of refusal kinds with the named arguments,
which would copy clap's `ErrorKind` and fall behind it; leaving the tool
to parse again, which costs a second parse and can disagree with the
words that crumbkit gave clap.
