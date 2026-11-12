# ADR-0209. observe says whether it installed an observer

- Date: 2026-11-12
- Status: accepted
- Supersedes: ADR-0075, which it amends

## Context

ADR-0075 kept `events::observe` returning a `bool`. `true` covered an
observer that hears the whole run and an observer installed after `parse`
had reported the typed command line, which misses `typed` and may miss
`parsed`. `false` covered four full slots and a run that answers a
completion request. A caller could not tell these cases apart, so a usage
log installed after `parse` wrote a line without the command, and nothing
said why.

## Decision

- `observe` returns the `#[must_use]` enum `events::Observing`, with the
  variants `Installed`, `InstalledLate`, `Full` and `Completing`.
- `Installed` means the observer was installed before `parse` reported the
  typed command line. `InstalledLate` means it was installed after that
  report. Both install the observer, so ADR-0075's rejection of refusing a
  late install stands.
- `Full` means four observers are installed, and `Completing` means the
  run answers a completion request. Neither installs anything. The
  completion check comes first, so a completion run returns `Completing`
  whatever the slots hold.
- `Observing::installed` is a `const fn` that is true for `Installed` and
  `InstalledLate`. A caller that needs only the old answer calls it.
- The enum is not `#[non_exhaustive]`, so a caller's `match` names all four
  variants, and a new variant is a breaking change.
- crumbkit keeps one process-wide `AtomicBool` that the `typed` report sets
  on its first report, with or without an observer. An observer installed
  after that report counts as late, even when it then hears every report
  that remains.

## Consequences

A caller that compared the result of `observe` with `true` fails to build
and calls `.installed()`. A tool can warn when its observer was installed
late. The `typed` report stores one atomic value more, with or without an
observer.

Considered: keeping the `bool` and adding a function that reports a late
install, which leaves the two refusals indistinguishable; refusing a late
install, which ADR-0075 rejected; and `#[non_exhaustive]`, which lets a
caller's `match` miss a meaning that it should act on.
