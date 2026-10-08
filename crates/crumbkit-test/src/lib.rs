//! crumbkit-test will check a tool's clap command tree against the crumbkit
//! conventions. A tool calls `crumbkit_test::check(Cli::command())` in a test,
//! in the same way it calls clap's `debug_assert`, and the check fails the test
//! when the tree breaks a convention.
