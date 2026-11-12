# ADR-0207. A listing names its next steps only with `Listing::step`

- Date: 2026-11-12
- Status: accepted
- Supersedes: ADR-0050, ADR-0155 and ADR-0167, which it amends

## Context

A listing had two builders for its next steps. `Listing::next` took one
call of the tool and a phrase, and the text printed them on one line as
`Next: {call} {phrase}`. `Listing::step` took a `Step`, a sentence and the
commands that carry it out, and the text printed the sentence on a `Next:`
line and each command on its own line below it. ADR-0167 put both in the
JSON document under `next`, with the step of `Listing::next` first.

A `Step` with one call holds everything that `Listing::next` held, so
`Listing::next` is a special case of `Listing::step`. A reader of the API
had to learn both builders, how their text differs, and that the step of
`Listing::next` always comes first whatever order the tool called them in.
A tool that wanted its one call after a step of another program could not
say so with `Listing::next`.

## Decision

- `Listing::next` is removed. A tool names every next step with
  `Listing::step`, and a step that ran one call through `Listing::next`
  becomes `Step::new(sentence).call(call)`.
- The text prints every step as ADR-0155 describes: the line
  `Next: {sentence}`, then each command on its own line, indented by two
  spaces. A step that `Listing::next` printed on one line now prints on two.
- The JSON document holds the steps under `next` in the order the tool
  added them, with each step's sentence as `text`, as ADR-0167 describes.
  The shape of `next` and every listing schema are unchanged.
- crumbkit's own listings move to `Listing::step`: `schemas list`, the
  `self usage` report of crumbkit-usage and the shelf example. Each phrase
  that followed a call becomes a sentence that starts with a capital letter
  and says what to do, such as `Show books by id, up to 100 in one call.`

## Consequences

A listing has one way to name a next step, and its steps print and
serialize in the order the tool added them. Every tool that called
`Listing::next` fails to build on the release that removes it, and the
changelog gives the one-line change. The `Next:` lines of `schemas list`,
`self usage` and shelf's `books list` each take two lines, so their golden
files change, and the `text` of their JSON steps holds the new sentence.
`docs/behaviour/output/listings/README.md` describes the current behaviour.

Considered: keeping both builders, which keeps two ways to say one thing and
the fixed order of the step of `Listing::next`; and keeping `Listing::next`
as a shorthand that builds a `Step` from its call and phrase, which keeps a
second name to learn and would have to turn a phrase that follows a command
into a sentence that stands alone.
