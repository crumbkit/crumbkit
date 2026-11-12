# ADR-0205. A code borrows its segments

- Date: 2026-11-12
- Status: accepted
- Supersedes: ADR-0001 and ADR-0088, which it amends

## Context

`Failure::code` returned `&'static Code`, and a `Code` held three
`&'static str` segments. A tool that learns a code at run time, such as the
code a plugin answers with, called `Code::runtime`, which had to return a
code that lives until the process exits. It copied the three segments onto
the heap, never freed them, and kept each distinct code in a process-wide
table behind a mutex (ADR-0088 restored it in that form).

ADR-0001 capped the table at 256 distinct codes, because the memory was never
freed and a plugin that puts an id in its reason would otherwise grow the
process without bound. At the 257th distinct code, `Code::runtime` returned
`RuntimeCodeError::Full`, and the tool reported the failure under a fallback
code of its own, so the caller lost the plugin's code. A long-running tool, or
a test process that makes many codes, reached the cap through normal use, and
no setting raised it.

## Decision

- `Code` is `Code<'a>` and borrows its three segments as `&'a str`. It stays
  `Copy`, and `Code::new` stays a `const fn`, so a declared code is still a
  constant, written `Code<'static>`.
- `Failure::code(&self)` returns `Code<'_>` by value. A declared failure
  returns its constant, and a failure with a run-time code returns a code that
  borrows from the failure.
- `Code::runtime` checks the segments with the rule of `Code::new` and
  returns a `Code<'a>` that borrows them. It keeps no table, so a tool makes
  any number of run-time codes. `RuntimeCodeError::Full` is removed.
- `CodeBuf` is an owned code that holds three `Box<str>`. `Code::to_buf`
  makes one and `CodeBuf::as_code` lends a `Code<'_>` from it. A failure that
  learns its code at run time stores a `CodeBuf`.
- An observer receives `tolerated` codes as `Code<'_>`, which may borrow text
  that is gone after the call. The usage recorder, which keeps tolerated codes
  until the run ends, stores each as a `CodeBuf`. Its own limit of eight
  distinct tolerated codes for each run still bounds what one run holds.
- `Exit::failures` and `CODES` lists stay `&'static [&'static Code<'static>]`,
  so the lists that an author writes do not change.
- Exit lookup compares codes by their segments, so a run-time code with the
  segments of a listed code exits with that code's number.
- crumbkit writes `Code<'static>` and `Code<'_>` in every constant, signature,
  doc and example, so text an author copies is clean under the
  `rust_2018_idioms` lints as well as rustc's defaults.

## Consequences

Nothing that crumbkit makes for a run-time code outlives the value that holds
it, and no limit on run-time codes remains. Every hand-written `fn code`
changes its return type and drops the `&`, and a tool that matched
`RuntimeCodeError::Full` removes that arm. `#[derive(Failure)]` emits the new
signature, so a derived failure changes nothing. A code can no longer be read
from a temporary failure in one expression: `make_failure().code()` becomes two
statements or ends with `.to_buf()`. The process-wide table of `ToolName`
names is unchanged and is a separate decision.

Considered: keeping the table with a higher or configurable cap; a
`Cow<'static, str>` segment, which rustc refuses to promote in `&CONST`
because the type has drop glue; a `pub type StaticCode = Code<'static>`
alias; keeping `Code` as an alias of a borrowed `CodeRef<'a>`, under which
`Code::runtime` called through the alias demands `'static` segments; a
workspace-wide `allow` of `elided_lifetimes_in_paths`.
