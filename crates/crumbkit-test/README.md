# crumbkit-test

crumbkit-test checks a tool built with crumbkit from an ordinary `#[test]`.
Each check collects every problem it finds and fails the test with all of
them, and each problem starts on a new line. crumbkit is in design and is not published yet.

## Adding it

A tool adds crumbkit-test as a dev-dependency, from this repository until
the first release:

```toml
[dev-dependencies]
crumbkit-test = { git = "https://github.com/crumbkit/crumbkit" }
```

## Example

This test checks that no two of a tool's failure codes spell the same:

```rust
use crumbkit::{Code, Spelling};

const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
const TIMED_OUT: Code = Code::new("shelf", "network", "timed_out");

const BOOK_CODES: &[&Code] = &[&NOT_FOUND];
const NETWORK_CODES: &[&Code] = &[&TIMED_OUT];

crumbkit_test::check_codes(Spelling::COLONS, &[BOOK_CODES, NETWORK_CODES]);
```

## Where to read more

The rustdoc of each check says what it fails on and shows an example:

- Failure codes: [`check_codes`].
- Command and flag names: [`check_names`], [`check_grammar`],
  [`check_listings`] and [`check_filters`].
- The help guide: [`check_guide`].
- Help pages: [`check_help_sizes`] with [`Budgets`], and
  [`check_help_goldens`].
- Example output: [`check_examples`] with [`Examples`].

[ADR-0025] records why these checks exist, and [ADR-0004] records how golden
files work.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`check_codes`]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/src/codes.rs
[`check_names`]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/src/names.rs
[`check_grammar`]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/src/grammar.rs
[`check_listings`]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/src/listings.rs
[`check_filters`]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/src/filters.rs
[`check_guide`]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/src/guide.rs
[`check_help_sizes`]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/src/sizes.rs
[`Budgets`]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/src/sizes.rs
[`check_help_goldens`]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/src/goldens.rs
[`check_examples`]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/src/examples.rs
[`Examples`]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/src/examples.rs
[ADR-0025]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0025-help-checks.md
[ADR-0004]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/testing/0004-golden-and-snapshot-tests.md
