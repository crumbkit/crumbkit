# ADR-0149. A view flag that adds nothing to the JSON is accepted with a note

- Date: 2026-10-24
- Status: accepted
- Supersedes: ADR-0016, ADR-0062, ADR-0065 and ADR-0138, which it amends
- Missed: ADR-0065 and ADR-0138 refused `--full` and `--long` with `--json` for a row type that sets `Row::WHOLE`, although the JSON that `--json` alone prints already satisfies the request, so the refusal only made the caller run the command again.

## Context

A leaf offers only the view flags that its document can use, and crumbkit
refused the others with exit 2, the code `<tool>::cli::usage` and a hint
to drop the flag (ADR-0016). Some of those refusals guard a real conflict,
such as `--long` on a command that has no table. Others refused `--long`
or `--full` with `--json` where the JSON that `--json` alone prints
already holds the whole result:

- `--full` on a document or stream without crumbkit's envelope (ADR-0016);
- `--full` and `--long` with `--json` on a headed document (ADR-0062);
- `--full` and `--long` with `--json` on prose and bytes (ADR-0016);
- `--full` on rows whose type sets `Row::WHOLE` (ADR-0065), and `--long`
  with `--json` on them (ADR-0138).

A caller that types `--json --full` or `--json --long` asks for the whole
result, and crumbkit knows that `--json` alone prints it. The refusal gave
the caller nothing it lacked. It cost a second run, and the caller is often
an agent that pays for each run in time and tokens. When crumbkit already
knows what the caller wants and the output is the same, the request should
not cost a second call.

## Decision

- A view flag that adds nothing to the JSON is accepted with a note and is
  never refused. `--long` and `--full` with `--json` add nothing when the
  JSON already holds the whole result: for prose and bytes, for a document
  or stream without the envelope, for a headed document and for rows whose
  type sets `Row::WHOLE`. `parse`, `finish` and `stream` accept the flag,
  print what `--json` alone prints and exit as `--json` alone exits.
- The run writes one note to standard error after the result, as a note
  that describes a listing does (ADR-0050, ADR-0128): `note: --long adds
  nothing to --json here; the JSON already holds the whole result.`, or
  the same line for `--full`. With both flags the view holds `--full`
  alone, because `--full` already takes in `--long`, so the note names
  `--full` once. The note does not start with `warning: <tool> left out`,
  the prefix that crumbkit-test's example checks fail on.
- `finish` writes the note after the result and its cut line, before the
  failures of a batch or prose result and before the dry-run line. It
  writes no note for a command that fails as a whole, because that run
  prints only the failure. A stream writes the note after its last line,
  also when the output budget stopped it or `end` received an `Err`,
  because the stream has printed result lines by then.
- The refusals that guard a real conflict stay: `--fields` where the
  document has no columns, `--full` without `--json`, `--long` without
  `--json` where there is no table or no long column, `--long` with
  `--json` on enveloped rows that do not print whole and have no long
  columns, because their JSON holds only the columns, and every view flag
  on a leaf that prints no result (ADR-0143).
- Help still leaves out a flag that the leaf does not offer, so it does
  not suggest a flag that adds nothing. crumbkit-test's `check_guide`
  fails an example that types a flag that adds nothing, offered or not,
  and its message says that crumbkit prints a note.

## Consequences

A caller that adds `--long` or `--full` to `--json` where they add nothing
gets its answer in one run and learns from the note that the flag can go.
A script that relied on the refusal's exit 2 now gets the exit of `--json`
alone. `--json --long` on a document without the envelope whose rows have
long columns was accepted silently before and now also prints the note.
`docs/behaviour/output/shapes/README.md` describes the current behaviour.

Considered: keeping the refusals, which lost because they refuse a request
that the JSON already satisfies and only force a retry; accepting the
flags without a note, which leaves the caller unaware that the flag can
go; a note for each flag when both are given, which repeats one reason;
offering the flags in help, which suggests flags that change nothing;
accepting only whole rows, which kept the same cost on the other
documents whose JSON holds the whole result.
