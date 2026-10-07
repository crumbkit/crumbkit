# 0001. Errors

- `Failure: std::error::Error` has `code() -> &'static Code` and `recovery() -> Recovery`, plus a provided `url()` that returns `None` by default. The cause line is the error's `Display`.
- `Recovery` has the variants `Run(Call)`, `Retry`, `Hint(text)` and `None { reason }`. Text output labels the hint line `help:`.
- A `const fn` checks each code's `<tool>::<area>::<reason>` grammar. Each error type exposes its codes in a `CODES` list, which crumbkit-test's codes check receives, and nothing registers codes at runtime.
- The optional `miette` feature, off by default, adds a provided `Failure::diagnostic()` that returns `None`. A failure that returns `Some(self)` gets miette's source snippet, labels and cause chain in its text output, and its code, recovery and url still come from `Failure`. Without the feature, an error that points into a file prints no snippet. CI builds and tests with and without the feature.
- The `miette` feature also renders a `miette::Report`. It checks the report's code at render time, maps its help to `Recovery::Hint` and its labels to `labels`, and an optional adopter-supplied finder recovers the adopter's own `Failure` from the report's chain. A malformed code prints as `<tool>::cli::unchecked`, with the original string in the message.
- `crumbkit::Error` wraps a `Box<dyn Failure + Send + Sync>` and converts from every `Failure`, so `?` works across a tool's error types. It has no context methods.
- A later `crumbkit-derive` uses its own `#[failure(code = ..., recovery = ...)]` attribute. It waits until hand-written `impl Failure` blocks prove a burden.

Considered: a required miette dependency, a blanket impl over `miette::Diagnostic`, converting failures into `miette::Report`, reading miette's `#[diagnostic]` attribute, error-stack, a separate `help()` method beside `Hint`, registering codes on an `App` value, context wrapping on `crumbkit::Error`.
