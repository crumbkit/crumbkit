# ADR-0086. A tool sets its spelling on its guide

- Date: 2026-10-18
- Status: accepted
- Supersedes: ADR-0001 and ADR-0076, which it amends
- Missed: ADR-0001 did not consider that a tool passing the spelling to every call that prints a code could pass two different ones. ADR-0076 did not consider that the `Output` could carry the spelling with the guide's other settings.

## Context

Every tool declared a `Spelling` constant and passed it beside the guide or
the `Output` to `parse`, `parse_from`, `parse_to`, `finish`, `finish_to`,
`stream`, `stream_to`, `hand_over` and `hand_over_json`, and to the
crumbkit-test entry points `check_codes`, `Recoveries::new`, `Twins::new`
and `check_twin`. Almost every tool passed `Spelling::COLONS`, so the
constant carried no information. The example tools named it a dozen times
each. `parse` already took the guide and applied the guide's tool name to
the spelling, and the `Output` already held every other setting of the
guide.

## Decision

- `Guide::spelling` is a const builder that sets the tool's spelling, and
  `Guide::get_spelling` returns it. A guide that sets none uses
  `Spelling::COLONS`, so a tool with colon codes declares no spelling.
- `parse`, `parse_from` and `parse_to` read the spelling from the guide,
  apply the guide's tool name as ADR-0079 decided, and keep it in the
  `Output` that they return. `Output::new` does the same for a tool that
  parses with clap alone.
- `finish`, `finish_to`, `stream`, `stream_to`, `hand_over`,
  `hand_over_json`, `Output::print_failure`, `Output::refuse_view` and
  `Output::refuse_fields_for_text` lose their `spelling` parameter and
  print in the spelling that the `Output` holds.
- `Recoveries::new`, `Twins::new` and `check_twin` lose their `spelling`
  parameter, because each check already takes the guide.
- `check_codes` takes the guide in place of a `Spelling`. It was the one
  entry point that took a spelling without a guide. Every tool that prints
  through crumbkit has a guide, because `parse` and `Output::new` both take
  one, so a clap-only tool passes the guide it gives `Output::new`. The
  check then spells codes with the same value that the tool prints them
  with, and a test cannot pass a different spelling by mistake.
- The run observer's `typed` and `failed` keep their `Spelling` parameter,
  because the usage log records a code in the tool's spelling and an
  observer sees no guide. crumbkit passes the spelling that the guide or
  the `Output` holds.
- `Spelling` stays public, because it is the type that `Guide::spelling`
  takes and a tool with its own spelling builds one with
  `Spelling::custom`.
- crumbkit-test spells the codes in its problem messages in the guide's
  spelling with the tool's name, so a message names a code the way the
  tool prints it.
- The opt-out report lists a guide's spelling as a `code-spelling`
  customisation when it spells crumbkit's usage code other than
  `Spelling::COLONS` does, the way it lists renames and exit codes.
- The rest of ADR-0001 and ADR-0076 is unchanged.

## Consequences

A tool states its spelling in one place, and every printed code and every
check reads that place. A tool with colon codes deletes its `Spelling`
constant and one argument from each call. Every tool changes its calls to
the functions above, which is a breaking change. A tool cannot print one
run's codes in two spellings, which no tool did.

Considered: keeping the parameter on `finish` alone; a setter on `Output`
that changes the spelling after `parse`; `check_codes` keeping its
`Spelling` parameter beside the other checks' guide; a second
`check_codes` form for a tool without a guide; dropping the parameter from
the observer's methods and passing the guide to the observer.
