# ADR-0078. A refusal recovers with the tool's call

- Date: 2026-10-17
- Status: amended by ADR-0123
- Supersedes: ADR-0001 and ADR-0056, which it amends
- Missed: ADR-0001 and ADR-0056 let a leaf or the guide declare a function that builds calls from a refusal, but always offered those calls after the `-h` recovery, so an agent that follows only the recovery reran help when the tool had returned the corrected line.

## Context

A refused command line recovered with the `-h` of the deepest typed
command. A leaf or the guide could declare `refusal_alternatives`, a
function that builds calls from the refusal, such as the refused line with
the refused value corrected. crumbkit kept each call that renders and whose
line parses, and offered it after its own alternatives. The tool's
correction therefore printed on the second `help:` line in text and under
`alternatives` in JSON. An agent that reads only the recovery ran `-h` and
had to work out the correction again from the help page, although the tool
had already returned it.

## Decision

- When a refusal's recovery would be the `-h` call and the tool's function
  returns at least one call that renders and whose line parses, the first
  such call became the recovery. The `-h` call became the first
  alternative, followed by crumbkit's own alternatives and then the tool's
  remaining calls, in the order that the function returned them.
- When the function returns no such call, or the command has no function,
  the `-h` call stayed the recovery.
- A refused `--limit` above its bound kept its recovery, the caller's line
  with `--all` in place of `--limit`, and the tool's calls stayed
  alternatives after crumbkit's. That recovery is built from the caller's
  own words and repairs the refused value, so it is already the specific
  step that a tool's function would offer.
- crumbkit added no option for the order. The tool's function exists to say
  what the tool thinks the next step is, and a tool that wants `-h` as the
  recovery returns no call.

## Consequences

`docs/behaviour/failures/README.md` describes the order. A tool whose
function returns one suggestion for every refusal sees that suggestion
become the recovery also for refusals that it does not repair, and can
return no call for those refusals to keep `-h`. shelf's `books show` does
this: its refusals recover with `books list` and offer `books show -h` as
the first alternative, except a line that types ids beside `--from`, whose
recovery stays `books show -h`, because listing the books does not repair
that conflict.

Considered: a leaf or guide setting that chooses whether the tool's call
replaces the `-h` recovery, a separate function for the recovery beside the
function for alternatives, promoting the tool's call also over the `--all`
recovery of a `--limit` above its bound, keeping the order and documenting
that an agent reads the alternatives.

## Children

- [ADR-0123. A refusal recovers with the tool's hint](0123-a-refusal-recovers-with-the-tools-hint.md)
