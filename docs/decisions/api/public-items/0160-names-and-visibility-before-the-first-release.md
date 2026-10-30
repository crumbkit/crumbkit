# ADR-0160. Names and visibility before the first release

- Date: 2026-10-30
- Status: accepted
- Supersedes: ADR-0057, ADR-0071, ADR-0085, ADR-0088, ADR-0089, ADR-0100, ADR-0107 and ADR-0148, which it amends
- Missed: ADR-0148 made `Duration` a tuple struct with a public field, unlike `Age` and `Bytes`, so a tool built and read it through the field and the wrapped type became part of the API.

## Context

An architecture review before crumbkit's first release found public names
that disagreed with each other and items that were public by accident.
Each was cheap to change before 0.1.0 and a breaking release after it.

- The exit status of a run had three names. `Stop::code` returned an
  `ExitCode`, `Stop::number` and `Refused::number` returned the same status
  as a `u8`, and `events::Finished::exit`, `events::Refused::exit` and
  `Observer::ended` called that `u8` the exit. `Refused::code` returns the
  refusal's failure `Code`, so `code` meant two things.
- `crumbkit::Refused` and `crumbkit::events::Refused` were two public types
  of one name. `events::Refused` and `events::Finished` were
  `#[non_exhaustive]`, which changes nothing for a struct whose fields are
  all private.
- `Duration` had a public field, so its representation was API.
- `Rows` carried about 26 `#[doc(hidden)]` members that any tool could
  call on a result it held. The example tool's tests called them to read a
  handler's rows, failures, notes and prose, and an outside tool reads the
  rows, the total and the failures of any result in generic code, so a
  tool does need to read a result.
- The hidden getter `Document::always_prints_first` was one letter from
  the builder `always_print_first`.
- `RuntimeCodeError::Segment` had public fields without
  `#[non_exhaustive]`, so a new field would break a tool that matched it.
- crumbkit re-exported schemars publicly and serde as the hidden
  `crumbkit::shown::__serde`. A tool names schemars through crumbkit in an
  override of `Shown::schema`, and the docs show it, so the hidden form of
  the serde re-export was the odd one out.
- crumbkit-usage's `Facts` derived `Default`, so a tool could build one,
  although its documentation says that a tool cannot.

## Decision

- `Stop::exit_code` returns the `ExitCode`, and `Stop::exit` and
  `Refused::exit` return the status as a `u8`. Across crumbkit, `exit`
  names the status as a number and `code` names a failure code.
  `From<Stop> for ExitCode` stays.
- The observer's event is `events::RefusalEvent`. It and `events::Finished`
  are plain structs with private fields and getters, which lets them gain
  a field without `#[non_exhaustive]`.
- `Duration`'s field is private. `Duration::new` and `Duration::get` are
  `const fn`, and `From<std::time::Duration>` stays.
- `Rows` keeps three documented members, `rows`, `total` and `failures`,
  because an outside tool reads them in generic code over `T: Rows`, such
  as a function that counts a result's rows or reads its first failure's
  code. The other members that `finish` reads move to the supertrait
  `Sealed`, which sits in a private module, so no tool can name it or call
  its members on a concrete result.
- A tool reads a concrete result through public methods of that result:
  `Listing::rows`, `Listing::total`, `Listing::get_summary` and
  `Listing::get_note`, `Batch::rows`, `Batch::failures`, `Batch::left_out`
  and `Batch::get_note`, and `Prose::value`, `Prose::failures` and
  `Prose::get_note`. The inherent `failures` returns the tool's own failure
  type.
- A getter whose plain name a builder already takes has the `get_`
  prefix, as the getters of `Guide`, `Leaf` and `Exit` do and as
  `std::process::Command::get_args` does. `Document::always_prints_first`
  becomes `Document::get_always_print_first`, and `Document::get_envelope`
  keeps its name.
- `RuntimeCodeError::Segment` is `#[non_exhaustive]`.
- crumbkit re-exports serde as `crumbkit::serde`, public and documented
  as `crumbkit::schemars` is, and `shown!` names serde through it. Both
  re-exports exist because crumbkit's bounds already make their major
  versions part of crumbkit's API.
- `Facts` has no `Default`. crumbkit-usage builds it with a crate-private
  constructor.

## Consequences

A tool that called an old name fails to compile, and the compiler names
the item. No guard keeps an old name from returning, because ADR-0162
deleted the compile_fail guards for items changed before the first
release. In a tool's own function with a
`T: Rows` bound, `Rows::rows`, `Rows::total` and `Rows::failures` are the
supported reads. Such a function can still call `Sealed`'s members on `T`,
because Rust lets a generic call the members of every supertrait of its
bounds, but those calls are unsupported and may break in any release. The
members are absent from the documentation and from every concrete result,
and a comment at `Sealed` says so.

Considered: keeping `Stop::code` and naming the `u8` `exit_status`;
hiding `crumbkit::schemars` as `__serde` was hidden, which would hide a
path the docs teach; a private token argument on each `Sealed` member,
which seals it fully and adds an argument to every call in crumbkit;
renaming `Document::get_envelope`, which would leave `Document` with the
one getter that differs from `Guide` and `Leaf`.
