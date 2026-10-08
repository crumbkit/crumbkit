# ADR-0007. JSON schemas

- Every JSON document is an object with a `schema` id and the result under `data`, such as `{"schema":"shelf/books-list/v1","data":[...]}`.
- A document id is `component/name/vN`. A tool declares it as a typed constant, such as `const BOOKS_LIST: Document<Vec<Book>> = Document::new("shelf", "books-list", 1)`, so the compiler ties the id to its result type. `Document::new` is a `const fn` that fails compilation unless the component is non-empty lowercase ASCII letters, digits and underscores, the name is non-empty lowercase ASCII letters, digits and dashes, and the version is at least 1.
- Every JSON document type derives a 2020-12 `JsonSchema`. A test compares each schema with its checked-in file, which the binary serves.
- Removing, renaming or retyping a field, making it optional, or adding an enum value needs a new `/vN` id. Readers ignore unknown fields.
- A `Recovery` is tagged by `kind`: `run` with `call`, `retry`, `hint` with `text`, or `none` with `reason`.
- The failure document `crumbkit/failure/v1` has `schema`, `code`, `message`, `cause`, `recovery`, and `url` when present. With the `miette` feature, it also has an optional `labels` array, where each label gives the source name, line, column, length and text. The schema allows any non-empty ASCII `code` without whitespace, because each tool chooses its own spelling.

Considered: hand-written schemas, boon, an untyped string id.
