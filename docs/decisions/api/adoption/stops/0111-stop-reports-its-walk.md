# ADR-0111. Stop reports the path, the flags and an option's value

- Date: 2026-10-20
- Status: amended by ADR-0136
- Supersedes: ADR-0109, which it amends
- Missed: ADR-0109 gave the getters to `Refused` and missed that a tool which writes a note after a help page needs the same getters on a `Stop`.

## Context

A tool that writes a note after a help page writes it in the format that a
global option such as `--error-format` asks for. `try_parse` returns a
`Stop` after help, the version or a topic, and a `Stop` held only its exit
code, what printed and the `Output`, so the tool read the words again.

## Decision

- A `Stop` stores the path that the walk reached and the words after the
  program name that crumbkit gave clap.
- `Stop::path`, `Stop::flags` and `Stop::value` have the signatures and
  the meaning of the getters of the same names on `Refused`.
- One private module reads the flags and the values for both types.

A topic prints only at the guide's root, so its path is empty. After
`Output::new` refused its view flags, the words are the ones passed to it.

## Consequences

A tool reads the three on any run that `parse` ended. The path and the
words sit in the `Stop`'s `Box` beside the `Output`, so a `Result` that
holds a `Stop` stays the same size.

Considered: getters on `Unparsed` that dispatch to either variant, which
leave out the `Stop` that `Refused::print` and `Output::new` return;
storing clap's matches, which clap does not produce for a help page.
