# ADR-0040. Schemas

- Date: 2026-10-12
- Status: accepted

## Decision

These records decided the shape of the JSON documents that a tool prints
and how a caller reads their schemas: the envelope, document ids and their
versions, and the `schemas list` and `schemas show` commands with the
checks that pin each schema.

## Children

- [ADR-0007. JSON schemas](0007-json-schemas.md)
- [ADR-0035. Schema commands](0035-schema-commands.md)
- [ADR-0080. Documents of runtime leaves](0080-runtime-leaf-documents.md)
- [ADR-0110. Schemas describe what the tool prints](0110-schemas-describe-what-prints.md)
- [ADR-0113. A skipped column admits null](0113-a-skipped-column-admits-null.md)
- [ADR-0132. A variant without a column admits null](0132-a-variant-without-a-column-admits-null.md)
