# Behaviour

These pages state what crumbkit does now: what a tool's callers see and
which conventions crumbkit enforces on a tool. The contract of a single
item lives in its rustdoc, and each page links to the items it uses. Each
section ends with a "Why:" line that names the decision records holding
its reasons.

- [Parsing](parsing/README.md): `parse`, its stops and refused command
  lines, [aliases](parsing/aliases.md) and
  [shell completion](parsing/completion.md).
- [Output](output/README.md): the result formats and shapes,
  [listings](output/listings/README.md), [streams](output/streams.md), the
  [output budget](output/budget/README.md),
  [child processes](output/child-processes.md),
  [terminal detection](output/terminal.md) and the
  [JSON layout](output/json-layout.md).
- [Failures](failures/README.md): the failure model, codes and recoveries,
  [failure forms](failures/forms.md) and
  [exit codes](failures/exit-codes.md).
- [Help](help/README.md): help pages, the `help` command and topics,
  [leaf examples and columns](help/examples.md),
  [`commands list`](help/commands-list.md), the
  [command reference](help/reference.md), [help weight](help/weight.md)
  and the [help checks](help/checks.md).
- [Effects](effects/README.md): effects, dry runs and confirmation,
  [pickers](effects/pickers.md) and [write guards](effects/write-guards.md).
- [Schemas](schemas/README.md): JSON documents, their ids and schemas, and
  the [schema commands](schemas/commands.md).
- [Adoption](adoption/README.md): the guide, the core crate, the public
  API, [opt-outs](adoption/opt-outs.md) and
  [declarations](adoption/declarations.md).
- [Usage log](usage-log/README.md): the local usage log,
  [trace context](usage-log/trace-context.md) and the
  [usage report](usage-log/report.md).
- [Checks](checks/README.md): what crumbkit-test's checks enforce on a
  tool.
