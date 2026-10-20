# ADR-0107. Refused exposes clap's error and Stop says what printed

- Date: 2026-10-20
- Status: amended by ADR-0109
- Supersedes: ADR-0100, which it amends
- Missed: ADR-0100 gave `Refused` getters for the words and the culprit and missed one for clap's error, which a tool needs to word the refusal of a conflicting or repeated argument itself, and it left a `Stop` unable to say whether help, the version or a topic printed.

## Context

ADR-0100 made `try_parse` return a `Refused` that holds clap's error, and
gave it getters for the message, the code, the recovery, the words and the
culprit. A tool that prints a refusal in a sentence of its own needs to
know what kind of refusal clap made and which argument it named. For a
conflicting or repeated argument, `culprit` names a flag at most, and
`message` holds clap's English sentence. The tool could learn the kind
and the argument only by parsing the command line again with clap.

A tool that records the outcome of each run in a history of its own reads
`Stop::number` for a run that `parse` ended. The number is 0 after help,
the version and a topic alike, so the history could not tell them apart.

## Decision

- `Refused::error` returns `Option<&clap::Error>`: clap's error, or `None`
  for a refused view flag, which crumbkit refuses itself after clap
  accepted the command line. A tool reads the error's `kind()` and its
  `get(ContextKind)` values, which clap fills through its `error-context`
  feature. crumbkit already enables that feature (ADR-0003).
- `Stop::printed` returns a `Printed`, a `#[non_exhaustive]` enum whose
  variants are `Help`, `Version`, `Topic`, `Refusal` and `Failure`, the
  last for the failure of a picker's source. `Refusal` covers a refusal
  that `Refused::print` prints and the refusal of view flags that
  `Output::new` prints. A page that could not be written reports what
  `parse` tried to print, with the number 1.

## Consequences

A tool can word the refusal of a conflicting or repeated argument from
clap's `ErrorKind::ArgumentConflict` and `ContextKind::InvalidArg` without
a second parse. clap's error type becomes part of crumbkit's public API
through this getter, so a major version of clap that changes `clap::Error`
is a breaking change for crumbkit. crumbkit already exposes clap's
`Command`, so the getter adds no dependency for a tool.

A tool can record whether a stopped run printed help, the version, a
topic, a refusal or a failure. `Help` does not say whether the page was
the long one, which the observer's `helped` event already reports. A new
kind of page that `parse` prints adds a variant without a breaking change.

Considered: a crumbkit enum of refusal kinds with the named arguments,
which would copy clap's `ErrorKind` and fall behind it; leaving the tool
to parse again, which costs a second parse; separate `Help` variants for
the short and the long page, which a tool can already tell apart through
the observer.
