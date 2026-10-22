# Schema commands

This page covers `<tool> schemas list`, which lists the JSON documents that a
tool prints, and `<tool> schemas show <ID>...`, which prints the JSON Schema
of each, so an agent learns the shape of a document from the tool it runs. It
also covers crumbkit's fixed command paths and the checks that pin each
schema. [`crumbkit::schemas`](../../../crates/crumbkit/src/schemas.rs)
describes the wiring, and shelf shows it.

## Mounting the commands

The tool adds a `schemas` group whose `list` and `show` subcommands take
`SchemasList` and `SchemasShow`, and its handlers pass `schemas::list` and
`schemas::show` to `finish` with `SCHEMAS_LIST` and `SCHEMAS_SHOW`.

crumbkit's own commands sit at fixed paths below the tool: `commands list`,
`schemas list`, `schemas show`, `self usage`, `self figures list` and
`aliases list`. An agent that knows one crumbkit tool then runs them in every
other by the same words. crumbkit-test's
[`check_mounts`](../../../crates/crumbkit-test/src/documents.rs) fails on a
leaf that declares one of their documents at another path and names the path
it belongs at. A command tree does not record the Rust type of a command's
arguments, so the check knows crumbkit's commands by the document their leaf
declares, and review checks a command that takes crumbkit's arguments type
without such a leaf. The id check skips these documents, so each failure
names one fix.

Why: ADR-0035, ADR-0015.

## `schemas list`

The tool's `Guide` is the source of the documents. Each leaf that declares
its document with `Leaf::document` or `Leaf::stream` adds that document, and
crumbkit adds `<tool>/failure/v1` first, because every command prints it
when it fails with `--json`.

- A leaf whose path is hidden in the command tree that the run's `Output`
  holds adds nothing, so the commands list only what help
  shows.
- Two leaves that declare one id give one row that names both command lines.
- A leaf that declares `Leaf::prints_no_result` has no document and adds
  nothing.
- A tool declares the files that users write for it, such as a settings
  file, as `FileFormat`s with `Guide::documents`. `schemas list` lists them
  after the leaves' documents with no commands.
- Each row's `kind` is `output`, `unwrapped` or `file`, so an agent sees
  which documents a command prints. Each row's compact `form` column is
  `document` or `stream` ([streams](../output/streams.md)).

Why: ADR-0035, ADR-0041, ADR-0081, ADR-0143.

## `schemas show`

`schemas show` serves the schema that `schemas::document_schema` builds from
a document's id and its result type: the `schema` constant, `data`, and
`total` and `truncated`, `summary` or `failures` by the result's shape. The
`data` of a prose result is the schema of its value. `schemas show` and the
tests of the checked-in schema files call the same function, so the served
schema and the checked-in file cannot drift. A batch's `failures` entries use
the failure document's schema without its `schema` property. For a file
format, `schemas show` serves the schema of the file's type with the id as
its title.

`schemas show` takes 1 to 100 ids, as every `show` command does, and returns
a `Batch`. An id that names no document adds a failure with the code
`<tool>::schemas::not_found`, whose recovery runs `schemas list`. Text
output prints each schema as compact JSON in one table cell, so a caller who
wants it pretty-printed uses `--json`.

crumbkit fixes the code `<tool>::schemas::not_found`, as it fixes the
codes of [`commands list`](../help/commands-list.md).

Why: ADR-0035, ADR-0016, ADR-0007, ADR-0076.

## Schema golden files

crumbkit-test's
[`check_schema_goldens`](../../../crates/crumbkit-test/src/schemas.rs)
compares the schema of each document that `schemas show` serves with a golden
file named after its id, such as `shelf-books-list-v1.json`, so a change to a
document's shape shows in review, where the [version rule](README.md#versions)
decides whether it needs a new id.

- It skips the documents of the component `crumbkit`, whose schemas
  crumbkit's own tests pin.
- It fails on two leaves that declare one id with different schemas, because
  `schemas show` serves only the first.
- It fails on two ids whose golden files have one name, such as
  `shelf/books-list/v1` and `shelf-books/list/v1`, which a component with
  dashes allows, so the tool renames one of the ids.
- It pins file formats too and fails on a file format with the id of another
  document.

`check_opt_outs` lists each document whose name is not its command's path as
a `document-name` customisation, and leaves out crumbkit's documents at fixed
paths, which `check_mounts` checks
([declarations](../adoption/declarations.md#the-customisations-section)).

Why: ADR-0035, ADR-0056.
