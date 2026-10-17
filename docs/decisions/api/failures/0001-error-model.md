# ADR-0001. Errors

- Date: 2026-10-13
- Status: amended by ADR-0061, ADR-0078, ADR-0079 and ADR-0085

## Context

An agent that meets a failure needs to know which failure it is, what to
run next, and which values were at fault, without parsing a message
written for people. Rust error types carry a message and a cause chain but
no stable code or next step, and tools use several error libraries.

## Decision

- A tool's error types implemented `Failure` beside `std::error::Error`,
  which adds a stable code, a recovery and an optional url. The message
  and causes come from `Display` and `source()`, and Debug formatting
  never reaches the output.
- Every failure stated a `Recovery`, so an agent always learns what to do
  next. `Recovery::None` with a reason is the exception for a failure no
  step can repair, and a declared failure with no step needs an opt-out.
  A `Run` call that the tool's tree cannot render prints as `None`, so
  crumbkit never advertises a command the tool hid.
- A failure could offer alternative repairs, because some failures have
  more than one reasonable repair. When the recovery offers no step, the
  first alternative with one takes its place, so the output never says
  there is no next step and then names one.
- A `Run` call carried concrete values with `Call::value`, so a failure
  can suggest the exact command that repairs it, and rendering refuses a
  value that the tool would refuse.
- A failure named the values at fault through `Failure::fields`, so a
  program reads them without parsing the message. `finish` drops a field
  with a bad or repeated name, because a wrong name must not fail a run
  that has already failed.
- A code got three checked segments: component, area and reason. The area
  is a noun from the tool's domain, so moving code does not change a code.
  `Code` has no `Display`, so every printed code goes through the tool's
  one `Spelling`, and `check_codes` refuses characters that make two codes
  look the same.
- `Code::runtime` accepted codes learnt at run time, such as a plugin's,
  and a process keeps at most 256 of them, so a plugin that puts an id in
  its reason cannot grow memory without bound. A new code past that limit
  is refused, and the tool reports the failure under a code of its own.
- crumbkit's own codes became public constants, so a tool's tests can
  name them, and a tool may print its own code in place of any of them
  (ADR-0056).
- Refusals by clap, an unknown `--fields` name and conflicting view flags
  failed with fixed crumbkit codes whose fields name what was refused, and
  a refusal's recovery runs the `-h` of the deepest typed command. A leaf
  or the guide could add refusal alternatives, because a tool may know a
  better next step. Alternatives that do not render or parse are left out
  silently, and a trace never holds the caller's values.
- A debug build warned on standard error about dropped fields and calls
  that do not render, so a tool's tests catch them. A release build
  writes no warning.
- The optional `miette` feature showed a diagnostic's labels, which
  crumbkit draws itself, and converted a `miette::Report` into `Error`
  with the fixed code `crumbkit::report::unchecked`. `Error::unclassified`
  did the same for any `std::error::Error`, so a tool using another error
  library reaches `finish` without crumbkit depending on that library.
- `Error` held any failure, so `?` works across a tool's error types. It
  cannot implement `Failure`, because its blanket `From` would overlap
  with `From<T> for T`, so `finish` takes either through the sealed
  `AsFailure` trait.
- `#[derive(Failure)]` wrote the `Failure` impl and the `CODES` list,
  because a hand-written impl repeats one `match` per method and a
  `CODES` list drifts from the variants. `code` takes a path to a
  constant, so `Code::new` still checks it at compile time. The derive
  writes no `Display`.

## Consequences

`docs/behaviour/failures/README.md` and `docs/behaviour/failures/derive.md`
describe the current failure model.

Considered: private constants for crumbkit's own codes, a `Recovery::Choice` variant holding several recoveries, which needs a new failure document id, alternatives printed as `no next step` when they do not resolve, literal words appended to a call without a check against the tree, the `Display` as the cause line, a required miette dependency, a blanket impl over `miette::Diagnostic`, converting failures into `miette::Report`, reading miette's `#[diagnostic]` attribute, error-stack, a separate `help()` method beside `Hint`, registering codes on an `App` value, context wrapping on `crumbkit::Error`, one fixed `::` spelling for every tool, a global spelling set once at startup, accepting only declared codes from a `miette::Report`, miette's graphical renderer, miette's narratable renderer, printing a report's own code through a second failure path, an adopter-supplied finder, a debug-build warning for a refusal alternative that does not render or parse, a usage code passed to `parse` in place of a guide's rename, a usage code built from the command's name, a `--fields` recovery that runs the root's `-h`, structured fields under `labels`, a field value that holds a float, keys inside a field's objects under the field name rule, an unbounded run-time code table, run-time codes from a list the tool registers at startup, a derive that also writes `Display`, a `code = "a::b::c"` string checked by the derive, a `From<anyhow::Error>` behind an `anyhow` feature, a new code for unclassified errors beside `crumbkit::report::unchecked`.
