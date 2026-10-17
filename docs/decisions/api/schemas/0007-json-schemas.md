# ADR-0007. JSON schemas

- Date: 2026-10-07
- Status: amended by ADR-0079

## Context

Programs and agents read a tool's JSON. They need to know which document
they hold, whether it is complete, and which changes may break them,
without guessing from the content. The documents also need schemas that
cannot drift from the types that print them.

## Decision

- Every JSON document became an object with a `schema` id and the result
  under `data`, so a reader always knows which document it holds. A
  document without the envelope exists for a tool that keeps an existing
  shape.
- A listing always held `truncated`, and `total` when the tool counted,
  so a program can tell whether it has every row. A batch and a prose
  result always held `failures`, written by the same code as the failure
  document, so the two shapes cannot drift. A bytes result holds text when
  the bytes are UTF-8 and base64 otherwise, so a program reads text
  without decoding it and still receives any byte.
- A document id became a typed `Document` constant tied to its result
  type, with a component that names the owner as dotted labels, so a tool
  can use a domain name. The name is the command's path joined with `-`,
  so an agent can predict a document's id from the command and the
  command from an id.
- Every document type got a 2020-12 `JsonSchema` derived from its types,
  and checked-in schema files pin crumbkit's and each tool's schemas.
- A failure's `fields` held scalars, null, arrays and objects but no
  float, because JSON has no form for NaN. Its `cause` is one string that
  joins the source chain, because library errors have no stable wording
  or depth, so a program matches on `code`.
- With the `miette` feature, failures held labels with source excerpts
  and related diagnostics down to 8 levels, so a program sees what
  miette's report shows. A ninth level is left out, which stops a
  diagnostic that lists itself as related from repeating forever.
- Removing, renaming or retyping a field, making it optional, or adding
  an enum value needs a new `/vN` id, and readers ignore unknown fields,
  so an optional new field keeps the `v1` id. Until crumbkit's first
  release, schemas change in place.

## Consequences

Several later optional fields, such as `dry_run`, `alternatives`, `all`
and `cut`, kept the `v1` ids under this rule.
`docs/behaviour/schemas/README.md` describes the current documents.

Considered: hand-written schemas, a hand-written envelope type per document, `cause` as an array of strings, boon, an untyped string id, a `total` only when rows were left out, a `more` field beside an absent `total`, a `next` field holding the same command with `--all`, `failures` only when an id failed, a `schema` in each entry of `failures`, the source line kept out of the JSON, the whole source text in the document, the field names of miette's JSON report handler, one excerpt merged across the labels that share lines, a component of underscores alone, `fields` as an array of name and value pairs, field values limited to strings, integers and booleans, a field value that holds a `serde_json::Value`, a field value built from any `Serialize` type, a limit on how deeply a field value nests.
