# ADR-0045. Failure forms

- Date: 2026-10-12
- Status: accepted

## Context

Some tools already publish a failure document that their callers read. To
adopt crumbkit they needed to keep that document while crumbkit still
decides when each failure happens, its recovery and its exit, and
crumbkit-test's checks needed to read failures in that shape.

## Decision

- A tool set a `FailureForm` on its guide, with methods that return the
  form's document id, its schema and the JSON of one failure. The text
  methods default to crumbkit's lines, so a form that changes only the
  JSON keeps them.
- crumbkit prepared each failure before the form sees it: the code in the
  tool's spelling, the recovery and alternatives with each call rendered,
  and the fields. A form therefore cannot get the rendering or the
  spelling wrong.
- crumbkit wrote the form's JSON on one line in a stream and flushed it,
  so each failure line stays one line and a closed pipe keeps the exit
  code.
- The form wrote every failure that prints alone, and with `entry` and
  `entry_schema` each entry of a batch's or prose result's `failures`, so
  a tool can have one failure shape in all of its JSON. The defaults keep
  crumbkit's entry, so an older form prints what it printed before.
- `schemas list` and `schemas show` served the form's document in place
  of crumbkit's failure document.
- A form became a declaration that needs no opt-out (ADR-0056), and
  `check_document_ids` fails on a form whose document claims the
  `crumbkit` component, because crumbkit owns it.
- `FailureForm::read` let a form read its own failures back, so the form
  gets the same checks as crumbkit's document. crumbkit-test's checks read
  a failed run through the form, then crumbkit's document, then its
  `code:` line. A tool with its own shape and no form sets a
  `FailureReader` instead, and setting both fails, because a tool reads
  its failures through one of them.
- `check_recoveries` stopped at the first run whose failure it cannot
  read, because every later run would fail the same way.

## Consequences

crumbkit re-exports schemars, so a form builds its schema without its own
schemars dependency. `docs/behaviour/failures/forms.md` describes the
current form API.

Considered: a re-export of `Schema` and `json_schema!` alone in `crumbkit::form`, a form that receives the failure alone and prepares nothing, a form registered on `Output` in place of the guide, a form that returns a typed `Serialize` value, batch entries that always keep crumbkit's shape, an `entry` whose default returns the form's `json`, a `Related` getter for miette's related diagnostics, a form document kept beside `crumbkit/failure/v1` in `schemas list`, a separate check for failure forms, a form example in shelf or reading with golden files, a required `read` method, a `read` that returns command lines as text, `check_recoveries` failing before it runs anything on a guide with a form, one problem for each run that a form cannot read, a form setting on `Examples` beside the guide's, which `check_examples` compared with the guide's form, removing `FailureForm::read` in favour of `FailureReader`, a reader that wins over the guide's form without a problem, an opt-out from `Rule::FailureForm` with a reason and three costs.
