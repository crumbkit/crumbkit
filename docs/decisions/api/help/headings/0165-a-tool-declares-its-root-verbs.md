# ADR-0165. A tool declares the verbs that sit directly under its root

- Date: 2026-10-30
- Status: accepted
- Supersedes: ADR-0015 and ADR-0056, which it amends

## Context

ADR-0015 made `check_grammar` require `<tool> <resource> <verb>` and gave
each leaf directly under the root an opt-out with a reason. Some tools put
commands at the root on purpose and keep them there. One example is a tool
with a few commands that each act on files a person names, where a resource
word would group nothing. Another is a tool whose daily commands, such as
`start`, `stop` and `logs`, people and scripts type as one word.

For such a tool the opt-out list held one entry per root command, and every
entry repeated the same reason. The list never shrank, because the choice
is permanent. An opt-out list records departures that a tool may pay down,
so these entries hid the real backlog among choices that will not change.

An adopter asked for one declaration of its root commands. API-1
(ADR-0122) accepts one adopter's request for an ordinary need that other
tools are likely to share, and a tool with commands at its root on purpose
is an ordinary shape for a command-line tool.

## Decision

- A tool declares its root verbs once, with one reason for the whole list,
  in one of two places.
  - `Guide::root_verbs(verbs, reason)` is the recommended place. Root help
    then lists each declared verb that no goal of a root group names under
    the heading `Verbs`, after the root group's goals and before the
    `Commands` heading of the other subcommands. A root without a group of
    its own gets the two headings in place of clap's `Commands:` list.
  - `Allow::root_verbs(verbs, reason)` in the opt-out list is the
    alternative, for a tool that does not want the heading. It changes no
    help page.
- A tool that declares root verbs more than once, on the guide and with
  `Allow::root_verbs` or with `Allow::root_verbs` twice, fails
  `check_grammar`. Refusing a second declaration is simpler than accepting
  the union: root help lists only the guide's verbs, so a union would leave
  some declared verbs out of the heading with no sign of it, and the report
  would show two reasons for one choice.
- `check_grammar` passes each declared root verb without an opt-out from
  `grammar/root-leaf`. A root leaf that is not declared still needs the
  opt-out, so a tool that plans to move its commands under resources can
  keep listing them one by one.
- A new rule with no opt-out fails a resource, at any depth, whose name or
  alias is the name or an alias of a declared root verb, so a resource that
  the tool adds later cannot take the verb's word. A declared verb that
  names no visible root leaf fails as a stale opt-out does, and so does a
  declaration without a verb.
- `check_opt_outs` lists the declaration as a `root-verbs` customisation
  with its reason. ADR-0056 gave declarations no reason, because they keep
  crumbkit's behaviour. A root verb departs from the grammar, so the
  declaration keeps a reason, and it sits among the customisations because
  the choice is permanent and the opt-outs list departures that a tool may
  pay down.
- The root's `-h` budget still covers the root help, heading and all, so
  each root line still costs bytes that `check_help_sizes` counts.
- The root is the clap root, the command that `check_grammar` takes, and
  not the command that `Guide::root` names.

## Consequences

A tool with commands at its root on purpose states the choice once. Its
opt-out list then holds only departures that it may pay down, and an agent
that reads root help sees which commands run without a resource. A tool that
declares nothing keeps the root help it had, byte for byte. The reading
example declares `where` on its guide, so its root help shows the heading.

Considered: accepting the union of both declarations; declaring the verbs
in the verbs argument of `check_grammar`, which `check_opt_outs` does not
take; a heading that the tool names; listing the root verbs after the other
commands; an opt-out for the collision rule; a cost line for each root verb
in the report.
