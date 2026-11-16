# ADR-0230. Help names aliases with a scope

- Date: 2026-11-16
- Status: accepted
- Supersedes: ADR-0228, which it amends
- Missed: ADR-0228 gave `Help::personal_aliases` a function of the clap root alone, so crumbkit-test could not print help with the shipped aliases only without a separate switch, and the opt-out check could not tell a tool that names only shipped aliases from one that names a person's.

## Context

ADR-0228 said that `Help::personal_aliases` takes a function pointer that a
tool wraps around its `Aliases`, and that the root's help names the entries of
shipped sources. A function pointer that takes only the clap root cannot tell a
run that prints help for a person from crumbkit-test, which prints pages that
must match on every machine.

## Decision

- `Help::aliases` takes a function of the clap root and a `Scope`, in place of
  `Help::personal_aliases`. `Scope::Shipped` is the default, and the function
  returns `Aliases::shipped_names` for it. `Scope::All` also names a person's
  entries, and the function returns `Aliases::names` for it.
- `Help::alias_scope` sets the scope. crumbkit-test prints help with
  `Scope::Shipped`, so a function must not read a personal source for it.
- The root's help line reads `Aliases:`.
- `Rule::PersonalAliases` applies when a tool sets `Scope::All`. The check reads
  the tool's scope and never a person's file.

## Consequences

`docs/behaviour/parsing/aliases.md` describes the current behaviour. A tool
that used `Help::personal_aliases` calls `Help::aliases` and `Help::alias_scope`.
