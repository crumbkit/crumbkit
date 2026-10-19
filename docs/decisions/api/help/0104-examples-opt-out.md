# ADR-0104. An opt-out for the example rule of check_guide

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0024, which it amends

## Context

ADR-0024 made `check_guide` require an example for every visible command
that runs, and `check_guide` took no list of opt-outs. A tool that adopts
goal headings and leaves therefore had to write an example for every
command before the check passed. An adopting tool reported that it could
not adopt goal headings before it had examples, because the check failed
on every command without one. The tool wrote a test of its own that
checked its goal headings as a stand-in for `check_guide`.

The other rules of `check_guide` report a name that matches no command or
a declaration that is missing or contradicts the tree. A tool corrects each
of those in the guide at once, and a tool that wants clap's own list for a
command leaves that command out of the guide.

crumbkit-test already had `Rule::Examples`, whose id `examples/run` belongs
to `check_examples`.

## Decision

- `check_guide` takes a third parameter, `allow: &[Allow]`, the list that a
  tool passes to its other checks. It reads only the opt-outs of its own
  rule.
- The example rule has an opt-out, `Rule::GuideExamples`, whose id is
  `guide/examples`. An opt-out from it covers the command it names and
  every command below it, so one opt-out covers a branch that has no
  examples yet. It needs a stated reason, as every opt-out does.
- `check_guide` fails on an opt-out from `guide/examples` that names no
  command or excuses no problem, so the opt-out goes once the branch has
  its examples.
- The other rules of `check_guide` take no opt-out.

## Consequences

Every caller of `check_guide` passes a third argument. A tool without
opt-outs passes `&[]`. `docs/behaviour/help/examples.md` and
`docs/behaviour/adoption/opt-outs.md` describe the current behaviour. No
example tool in this repository uses the opt-out, because each of their
commands has an example, so the adopting tool is its first user.

Considered: an opt-out for every rule of `check_guide`, which would let a
tool keep a guide that names commands it does not have; an opt-out that
covers only the command it names, which needs one opt-out per command of a
branch without examples; a separate check for the example rule, which
every tool would have to add to its tests; reusing `Rule::Examples`, whose
id belongs to `check_examples`.
