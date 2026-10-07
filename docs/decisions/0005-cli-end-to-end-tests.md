# 0005. End-to-end tests

- End-to-end tests run the built binary through snapbox's `cmd` module with a cleared environment and temporary XDG directories.
- crumbkit-test runs every example in a leaf's help as a test, and its output is the golden file the documentation shows.

Considered: trycmd, assert_cmd.
