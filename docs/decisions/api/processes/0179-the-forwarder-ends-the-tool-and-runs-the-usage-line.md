# ADR-0179. The forwarder ends the tool and runs the usage line

- Date: 2026-11-08
- Status: accepted
- Supersedes: ADR-0172 and ADR-0095, which it amends
- Missed: ADR-0172 left crumbkit-process's forwarder and crumbkit-usage's signal thread with no one owner of a signal that ends the tool, and it did not consider a terminal hand-over beside the forwarder.

## Context

A tool that uses crumbkit-process's `forward` feature beside crumbkit-usage
has two pieces of code that each decide on their own whether the tool ends
on one signal. The log's thread can write its line and end the tool during
the forwarder's grace, so the SIGKILL and the clean-ups never happen. The
forwarder can end the tool before the line is written, and on SIGHUP, which
the log does not listen for, the tool ends with no line. Both crates' READMEs
warned against the combination.

No tool combines these crates today. The need is ordinary, and other tools
are likely to share it: a tool that runs children and keeps a log wants both
crates.

## Decision

- Whenever a tool installs the forwarder, the forwarder owns ending the tool
  on SIGINT, SIGTERM and SIGHUP.
- The tool connects the crates in `main`. crumbkit-process keeps zero
  crumbkit dependencies, as ADR-0171 decided.
- `crumbkit_usage::start_under(register)` starts the log without its signal
  thread and passes the function that writes the interrupted line to
  `register`. This restores the capability that ADR-0095 removed as
  `start_with(Signals::Tool)`, in a smaller form, and a tool with its own
  handler can use it too.
- A tool that uses the log and the forwarder passes `forward::clean_up` to
  `start_under`, so the line is the forwarder's first clean-up and the
  forwarder has stopped the groups before the line is written. The line's
  duration includes the grace.

## Consequences

A tool that calls plain `start` beside `forward::install` still races, and
neither crate can detect it, because neither sees the other. The rustdoc of
both crates names the correct call. If two tools adopt the combination and
one gets it wrong, the shared crate below is worth its cost.

An `Extension`'s `write` runs while the forwarder holds its list's lock, so
under `start_under` it must not start a `Scope::Group` run.

Considered: a shared `crumbkit-signals` crate that all three crates register
with, which stops a tool forgetting the wiring but adds a published crate and
an ordering API for a collision that no tool has yet; and a `forward` feature
on crumbkit-usage that makes `start` defer to the forwarder, which Cargo's
feature unification would apply to every binary in a workspace, and which a
tool can forget as easily as the call.
