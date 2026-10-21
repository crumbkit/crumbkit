# ADR-0131. A refusal is styled at a terminal

- Date: 2026-10-21
- Status: accepted
- Supersedes: ADR-0049, which it amends

## Context

Help kept clap's colours at a terminal (ADR-0102), and a refusal printed
plain on standard error wherever it went. A person who typed a wrong
command line saw a plain error under a coloured help page, while clap's own
error rendering colours the `error:` label, the refused value and the flag
it was given to. Since ADR-0128, every piece of output carries its role to
the sink, and the sink alone decides the styles of each role, so a sink
could style a refusal without styling any other message. An adopting tool
asked for styled refusals, and the owner approved the request.

## Decision

- The text form of a refusal, and the group page printed in its place,
  carry clap's styles when the sink's error writer wants them. Every other
  message on the error writer, such as a failure, a note or a dry run,
  stays plain. The JSON refusal is a result and carries no styles, and a
  refusal that `try_parse` returns prints nothing.
- The `error:` label takes the command's error style. A refused value, a
  flag, a suggestion and the usage take the style that clap's own rendered
  error gives them, where crumbkit's text holds them. Removing the escape
  codes gives exactly the plain bytes.
- `Sink::styled_errors` is a public setter, and `Sink::is_styled_errors`
  reads it. A sink from `Sink::new` or `Sink::with_input` wants no error
  styles. The setter checks no terminal, so a tool that prints a refusal
  into its own sink with `Refused::print_to` decides colour itself, as it
  does for help with `Sink::styled`.
- The standard sink that `parse`, `parse_from` and `Refused::print` build
  decides once, in the sink's module. A non-empty `NO_COLOR` turns the
  styles off, and it beats `CLICOLOR_FORCE`. Otherwise a `CLICOLOR_FORCE`
  that is neither empty nor `0` turns them on. Otherwise they show only
  when standard error is a terminal. The command's `ColorChoice::Never`
  and a guide that turns terminal detection off keep the refusal plain.
- The styles need crumbkit's `color` feature, as help's do. Without it, a
  refusal prints plain on every sink.
- OUTPUT-1 gains an exception for the styles of a refusal and of the group
  page printed for it on standard error. The plain bytes stay the same at
  a terminal and at a pipe.
- The API-1 ground is one adopter's ordinary request, which the owner
  approved (ADR-0122).

## Consequences

A person at a terminal sees a refusal in the colours of clap's own errors,
and a golden file of a piped run still pins its bytes once the escape codes
are removed. shelf's pseudo-terminal test checks a refusal at a terminal
that way.

The rule differs from help's. Help reads standard output and clap's own
reading of the environment, so help ignores `CLICOLOR_FORCE` at a pipe,
ignores `NO_COLOR` under `ColorChoice::Always`, and reads `CLICOLOR` and
`TERM`, which a refusal ignores. clap also treats `CLICOLOR_FORCE=0` as a
request for colour, and a refusal does not.

A refusal that `finish` or `Output::new` prints through a standard sink
stays plain, because those sinks want no error styles.

Considered: styling every message on standard error, which would change the
bytes of failures and notes that OUTPUT-1 keeps the same at a terminal;
following standard output's terminal state, which styles a refusal sent to
a pipe when only standard output is a terminal; and reusing `Sink::styled`
for refusals, which would tie standard error's styling to standard output's
decision.
