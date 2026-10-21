# ADR-0136. Refused and Stop read their flags from the words the caller typed

- Date: 2026-10-21
- Status: accepted
- Supersedes: ADR-0109 and ADR-0111, which it amends
- Missed: ADR-0109 read the words that crumbkit gave clap and missed that the word `help` becomes `--help`, so a tool that records the flags the caller typed recorded a flag the caller never typed.

## Context

ADR-0109 gave `Refused` the getters `path`, `flags` and `value` for a tool
that records the flags that the caller typed in a history of its own, and
ADR-0111 gave the same getters to `Stop`. `flags` and `value` read
`Refused::parsed`, the words that crumbkit gave clap. crumbkit turns the
word `help` into `--help` before clap parses, so `tool help nosuch`
reported the flag `help`. An adopting tool records each refused run's flags
in its usage history, and that history said the caller passed `--help`.

The tool could not read the flags from `Refused::typed` itself without a
walk of its own, because crumbkit's reader of flags and values is private
and `command_path` returns only the path. A picker has the same effect: a
picked value goes into the words as `--name=value`, so a run whose later
source failed reported a flag that the caller did not type.

## Decision

- `Refused::flags`, `Refused::value`, `Stop::flags` and `Stop::value` read
  the caller's words: the words after the program name that the caller
  typed, with the tool's own name for each dashed form, without the word
  `help` that crumbkit answered, and without the values that a picker
  added. They read them as `command_path` reads words.
- The word `help` gives no flag. A `--help` or `-h` that the caller typed
  still gives clap's help id, `help`.
- `value` reads the same words as `flags`, so the two never disagree. The
  rewrite of `help` cannot change a value. crumbkit answers `help` only
  where the reader reads a name, never as an option's value, and it puts
  `--help` directly after a word that names something, where no option is
  waiting for a value. So `tool help --format json` gives `json` from
  either set of words. The one value that changes is a value that a picker
  added, which now comes back as `None`.
- `path` is unchanged. The word `help` names no command, so leaving it out
  reaches the command that `path` names, and the flags and values are read
  with that command's options. Reading `Refused::typed` with the word
  `help` in place would stop at `help` and miss the options of the command
  named after it.
- After `Output::new` refused its view flags, the getters read the words
  passed to it, as before.
- A `Refused` stores the caller's words beside `typed` and `parsed`. A
  `Stop` stores the caller's words in place of the words given to clap,
  which it never returned. The change adds no public API.

## Consequences

A tool that records the flags of a refused or stopped run records only
flags that the caller typed. A tool that read the `help` id to learn that
the caller typed the word `help` reads `Stop::printed` or `Refused::typed`
instead. Each `Refused` holds one more copy of the words, and a run that
parses holds no copy.

Considered: making the reader public, which adds API that only these
getters need; dropping the `help` id when crumbkit rewrote the word, which
also hides a `--help` that the caller typed on the same line.
