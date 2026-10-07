# 0001. Errors

- `Failure: std::error::Error` has `code() -> &'static Code` and `recovery() -> Recovery`, plus a provided `url()` that returns `None` by default. The cause line is the error's `Display`.
- `Recovery` has the variants `Run(Call)`, `Retry`, `Hint(text)` and `None { reason }`. Text output labels the hint line `help:`.
- A `const fn` checks each code's `<tool>::<area>::<reason>` grammar, and the adopter registers every module's `CODES` list on `App`.
- crumbkit does not depend on miette. A later `crumbkit-derive` crate provides `#[derive(Failure)]`, which reads miette's `#[diagnostic(code, help, url)]` annotation and an optional `#[recovery]`, and rejects a variant that has neither `help` nor `#[recovery]`.

Considered: a required miette dependency, an optional miette adapter, error-stack, a separate `help()` method beside `Hint`.
