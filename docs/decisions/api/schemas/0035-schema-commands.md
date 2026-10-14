# ADR-0035. Schema commands

- Date: 2026-10-11
- Status: accepted

## Context

An agent that reads a tool's JSON needs the shape of each document, and
the place it can always reach is the tool it runs. A schema served by the
tool must not drift from the checked-in schema, and a change to a
document's shape must show in review.

## Decision

- Each tool got `schemas list` and `schemas show <ID>...`, wired from
  crumbkit's argument types and handlers, so an agent learns a document's
  shape from the tool itself.
- The guide was the source of the documents. A leaf hidden in the
  run-time tree adds nothing, so the commands list only what help shows.
  crumbkit's failure document comes first, because every command prints
  it on failure.
- `Document::new` required a one-line description, so the compiler finds
  a document without one.
- `schemas show` and the schema tests both called
  `schemas::document_schema`, so the served schema and the checked-in
  file cannot drift. It takes 1 to 100 ids and returns a `Batch`, as
  every `show` command does.
- A document's name defaulted to its command's path joined with `-`, so
  an agent can predict one from the other. A different name is a
  declaration.
- crumbkit's own commands sat at fixed paths, and `check_mounts` fails
  on one at another path, so an agent runs them by the same words in
  every tool. The check knows them by the document their leaf declares,
  because a command tree does not record an argument's Rust type.
- `check_schema_goldens` pinned each served schema in a golden file named
  after its id, so a shape change shows in review, where ADR-0007 decides
  whether it needs a new version.
- A tool declared the files that users write for it as `FileFormat`s, so
  an agent can also read their schemas.

## Consequences

`docs/behaviour/schemas/commands.md` describes the current commands and
checks.

Considered: a `Schemas` trait that each result type implements, a registry of documents separate from the guide, an optional description, the schema pretty-printed in text output, a single-id `schemas show`, schema goldens for crumbkit's documents in every tool, the id check and the mount check as one check, finding crumbkit's commands by their arguments type, golden file names that escape the dashes of a component, a file format as a leaf without a path, `kind` among the `--long` columns, an opt-out with a reason for a document named other than its command's path.
