# Deriving Failure

`#[derive(Failure)]` writes a type's `Failure` impl and its `CODES` list
from a `#[failure(...)]` attribute on each variant. The rustdoc of
[`crumbkit-derive`](../../../crates/crumbkit-derive/src/lib.rs) is the
reference for the keys and shows the code the derive writes for an example
type. shelf derives the impl of its books failures.

## Turning it on

The derive sits behind the `derive` feature. crumbkit's `derive` feature
re-exports the macro from crumbkit-derive beside the trait of the same
name, so `crumbkit::Failure` names both. A procedural macro must live in
its own crate. The derive writes paths under `::crumbkit` unless its
`crate` key names another path to crumbkit, such as a re-export. The
feature is off by default, so a tool that depends on crumbkit alone and
writes its `Failure` impls by hand does not build crumbkit-derive. It
builds syn either way, because clap_derive needs it. crumbkit-child and
crumbkit-aliases turn the feature on for their own failures, so a tool that
uses either builds crumbkit-derive.

Why: ADR-0001, ADR-0097, ADR-0003.

## What it writes

A hand-written impl repeats one `match` per method, and a hand-written
`CODES` list drifts from the variants, so the derive writes both.

- `code` takes a path to a constant, so `Code::from_spelling` still checks the
  segments at compile time. The derive checks each field name by the same
  rule.
- `code = EXPR` takes any expression of type `Code<'_>` that reads the
  variant's fields, and `codes = [PATH, ...]` names the codes it returns, so
  `CODES` lists them. `codes = []` says the variant's codes are in no
  declared list. An expression `code` without `codes`, and `codes` beside a
  path `code` or `forward`, are compile errors that name the variant. The
  derive does not check that `codes` matches what the expression returns;
  `check_recoveries` and `check_twins` report an undeclared code when a test
  provokes it.
- `hint` text with braces goes through `format!`.
- `no_next_step` gives the reason of a `Recovery::None`, and its text with
  braces goes through `format!` too.
- `forward` returns every answer of a wrapped failure, so the wrapper's
  `CODES` leaves the wrapped codes to the wrapped type's own list.
- `causes = EXPR` gives a `Vec<String>` that `causes()` returns, one reason
  for each side-by-side cause. The expression reads the variant's fields.
  `forward` returns the wrapped failure's causes, and `causes` beside
  `forward` is a compile error that names the variant. The derive writes
  `causes()` only when a variant gives the key or forwards.
- `sentence = "..."` gives the fixed sentence of a variant. The text is one
  line with no `{`, and the derive refuses an empty text, a line break and a
  brace, because a placeholder would let a run-time value into reviewed text.
  `#[failure(reviewed)]` on the type makes a variant without a `sentence` a
  compile error that names the variant, except a variant that uses
  `forward`, which returns the wrapped failure's sentence. The derive does
  not check that the wrapped type has one, so a forwarded type is checked
  only when it is marked `reviewed` itself. A failure written
  by hand and a code from `Code::runtime` escape the check, and a tool moves
  a hand-written impl to the derive to have it checked.
- The derive writes no `Display`, so the message stays with the tool or
  another derive.

## Forwarding a foreign error

`forward` needs the wrapped type to implement `Failure`, and an adopter
cannot implement it for a library's error. The variant holds a newtype that
implements `Failure`, and `forward` returns its answers. The newtype owns
the mapping from the library's error to the failure's answers, takes
`Failure`'s defaults for the rest and follows every method `Failure` gains
later. A test can call it on its own.

```rust,ignore
// The newtype owns the mapping from the library's error to crumbkit's answers.
#[derive(Debug)]
struct LibraryFailure(LibError);

impl fmt::Display for LibraryFailure { /* delegate to self.0 */ }
impl std::error::Error for LibraryFailure {}
impl From<LibError> for LibraryFailure { /* wrap it */ }

impl LibraryFailure {
    const CODES: &'static [Code<'static>] = &[LIB_TIMEOUT, LIB_REFUSED];
}

impl Failure for LibraryFailure {
    fn code(&self) -> Code<'_> { /* by kind */ }
    fn recovery(&self) -> Recovery { /* by kind */ }
}

#[derive(Debug, Failure)]
enum AppError {
    #[failure(forward)]
    Library(LibraryFailure),
}
```

`AppError::CODES` leaves out the newtype's codes, so a test of the twin and
recovery checks passes `LibraryFailure::CODES` as well.

The test `crates/crumbkit/tests/derive_forward.rs` holds this pattern in
full, with a stand-in library error.

Why: ADR-0255.

The keys and the code that the derive writes for them are public API.
Removing or renaming a key, or changing what a key returns, is a breaking
change. A new key is a minor change, and so is a new trait method that the
derive starts to write.

Why: ADR-0001, ADR-0002, ADR-0226, ADR-0244, ADR-0255.
