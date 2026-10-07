# 0004. Golden files

- snapbox checks golden files, which hold the exact bytes a tool printed, at paths crumbkit chooses under `tests/golden/`.
- Contributors update them with `SNAPSHOTS=overwrite` and review the result with `git diff`. CI never writes them.

Considered: insta, expect-test, plain `assert_eq!` files.
