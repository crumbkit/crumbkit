# ADR-0005. End-to-end tests

- End-to-end tests run the built binary through snapbox's `cmd` module with a cleared environment and temporary XDG directories.
- crumbkit-test's [`check_examples`](../../../crates/crumbkit-test/src/examples.rs) runs every example of every leaf in the tool's guide against the tool's built binary, so each command line that help shows is a run that works.
- cargo gives the binary's path only to the integration tests of the binary's own package, so the check runs from one. An integration test imports only from a library, so that package has a library target that exports the clap command and the guide, and its binary calls the library, as shelf's does.
- The check runs each example with no shell, a cleared environment and fresh XDG directories, so a run depends only on what the example and the check give it. It runs the examples of hidden commands, because their own help lists them.
- The golden file holds the run's standard output only. A run that exits with a code other than 0 or writes to standard error fails the check, because an example shows a run that works.
- The check fails on a stale golden file, so the directory holds only example golden files, and on an exemption that names no command, as the other tree checks do (ADR-0015).

Considered: trycmd, assert_cmd, running the examples through a shell, golden files that also hold standard error and the exit code, the inherited environment for example runs, finding the binary from a unit test, including the tool's source files in the test with `#[path]`.
