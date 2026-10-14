# ADR-0048. Shell completion

- Date: 2026-10-12
- Status: accepted

## Context

People at a shell expect Tab completion, and completion that offers
something parsing would refuse misleads them. clap_complete's dynamic
completion answers from the same command tree a tool parses, but its
`CompleteEnv` exits the process or writes to standard output directly, and
it sits outside clap's semver promise.

## Decision

- A `complete` cargo feature, off by default, answered completion requests
  through clap_complete's dynamic completion, so the default build has no
  completion code.
- A tool's `main` calls `crumbkit::complete` first and passes the function
  that builds its command. The function runs only for a completion
  request, so an ordinary run builds its command once, and a tool that
  hides commands at run time hides them there too, so completion offers
  what parsing accepts. `complete_with_words` passes the shell's words for
  a tool whose completers depend on an earlier word.
- `complete` called clap_complete's shell adapters itself and never exits
  the process, so the answer goes through crumbkit's `Sink` and tests can
  complete in memory.
- A completion never prints a failure, because the shell shows standard
  error over the person's prompt.
- The completion variable stays set for the rest of the run, because
  removing an environment variable needs unsafe code.
- Candidates followed clap_complete's rule for hidden items, so a word
  that matches only hidden names completes to them. This is the one place
  where crumbkit prints a hidden name the caller typed only in part.
  Filtering it out would need the cursor position from an undocumented
  protocol and a second completion per Tab press.
- No run observer is installed for a completion request, so the usage log
  writes no line for a Tab press.
- crumbkit added no help topic for completion, because a topic would add
  a name to every root help page that agents read.
- clap_complete was pinned to `~4.6.11`, and a crumbkit minor release may
  change what the feature offers.

## Consequences

`docs/behaviour/parsing/completion.md` describes the current behaviour,
and the rustdoc of `complete` gives each shell's registration line.

Considered: `CompleteEnv::complete` with its process exit, a function that takes the words in `complete` and `complete_with`, which every tool would then write as a closure that ignores them, a `Command` that the tool edits after `complete` parses its words, `CompleteEnv::try_complete` writing to standard output, a `&mut Command` that the tool then passes to `parse`, a command passed by value and returned, the variable's name declared on the guide, which `complete` cannot read without building the guide on every run, a fixed variable name with an opt-out for a tool that keeps its own, filtering every hidden candidate, a `completion` help topic, a caret requirement on clap_complete, a usage log that skips only when the `complete` feature is on.
