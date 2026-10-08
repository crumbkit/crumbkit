# ADR-0007. JSON schemas

- Every JSON document type derives a 2020-12 `JsonSchema`. A test compares each schema with its checked-in file, which the binary serves.
- Removing, renaming or retyping a field, making it optional, or adding an enum value needs a new `/vN` id. Readers ignore unknown fields.
- A `Recovery` is tagged by `kind`: `run` with `call`, `retry`, `hint` with `text`, or `none` with `reason`.
- The failure document `crumbkit/failure/v1` has `schema`, `code`, `message`, `cause`, `recovery`, and `url` when present. With the `miette` feature, it also has an optional `labels` array, where each label gives the source name, line, column, length and text.

Considered: hand-written schemas, boon.
