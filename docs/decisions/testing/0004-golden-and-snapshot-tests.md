# ADR-0004. Golden files

- snapbox checks golden files, which hold the exact bytes a tool printed, at paths crumbkit chooses under `tests/golden/`.
- crumbkit-test's `check_help_goldens` keeps each `-h` and `--help` page in its own file, and a tool such as shelf keeps those files in `tests/golden/help/` (ADR-0025). Its `check_examples` keeps the standard output of each example in a leaf's help in its own file (ADR-0005).
- Contributors update them with `SNAPSHOTS=overwrite` and review the result with `git diff`. CI never writes them.

Considered: insta, expect-test, plain `assert_eq!` files.
