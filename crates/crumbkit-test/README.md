# crumbkit-test

crumbkit-test checks a tool built with crumbkit from an ordinary `#[test]`.

crumbkit is in design and is not published yet. This crate has one check.

- `check_codes` takes the tool's `Spelling` and the `CODES` list of each of
  its error types, and spells every code as `finish` prints it. The test
  fails, naming every problem, when two different codes spell the same, or
  when a spelled code is empty or holds a character other than printable
  ASCII, `!` to `~`. A code that two lists share counts once.

```rust
use crumbkit::{Code, Spelling};

const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
const TIMED_OUT: Code = Code::new("shelf", "network", "timed_out");

const BOOK_CODES: &[&Code] = &[&NOT_FOUND];
const NETWORK_CODES: &[&Code] = &[&TIMED_OUT];

crumbkit_test::check_codes(Spelling::COLONS, &[BOOK_CODES, NETWORK_CODES]);
```

A later check will take the tool's clap command tree, as
`crumbkit_test::check(Cli::command())`, in the same way that clap's
`debug_assert` checks the tree's definitions.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
