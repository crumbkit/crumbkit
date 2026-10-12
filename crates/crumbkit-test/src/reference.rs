//! The command reference check, which compares a tool's checked-in Markdown
//! reference with a fresh one written from its `commands list`.

use std::path::Path;

use snapbox::Assert;
use snapbox::assert::DEFAULT_ACTION_ENV;

use crate::goldens::compare;
use crate::runs::{self, Place, TIMEOUT, Xdg};
use crate::tree;

// The tests run shell scripts, which Windows cannot run. The two attributes
// stay apart, because clippy reads only a plain `cfg(test)` as test code.
#[cfg(unix)]
#[cfg(test)]
mod tests;

/// The arguments that ask a tool for the document of every command.
const ARGS: [&str; 5] = ["commands", "list", "--all", "--json", "--full"];

/// Checks that the Markdown reference in `file` is the one that
/// crumbkit-markdown writes from the help of the tool's built binary
/// `binary`, so a change to the tool's help that leaves the reference
/// stale fails the test.
///
/// The check runs `binary commands list --all --json --full` with a cleared
/// environment and fresh XDG directories, as [`check_examples`] runs an
/// example, and renders its output with `crumbkit_markdown::render`. With
/// `SNAPSHOTS=overwrite`, the check writes the reference to `file` instead,
/// as the other golden checks write their files (ADR-0004).
/// `cargo crumbkit docs --bin NAME` writes the same file.
///
/// [`check_examples`]: crate::check_examples
///
/// ```no_run
/// // In an integration test, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// crumbkit_test::check_command_reference("/path/to/shelf", "docs/commands.md");
/// ```
///
/// # Panics
///
/// Panics when the binary cannot start, when its `commands list` run fails,
/// when its output is not a document that crumbkit-markdown can render,
/// when `file` is missing or differs from the reference, or when
/// `SNAPSHOTS=overwrite` cannot write `file`. The message's first line is
/// `the command tree breaks crumbkit's command reference rules:`, and the
/// problem on the next line starts with the command line, such as
/// `shelf commands list --all --json --full`. A differing file's problem
/// continues with snapbox's report of both texts.
pub fn check_command_reference(binary: &str, file: impl AsRef<Path>) {
    let assert = Assert::new().action_env(DEFAULT_ACTION_ENV);
    check(binary, file.as_ref(), &assert);
}

/// Runs [`check_command_reference`] with `assert`, whose action decides
/// whether the check compares or overwrites, so a test chooses the action
/// without setting `SNAPSHOTS`.
fn check(binary: &str, file: &Path, assert: &Assert) {
    let tool = Path::new(binary)
        .file_stem()
        .map_or_else(|| binary.into(), |stem| stem.to_string_lossy());
    let line = format!("{tool} {}", ARGS.join(" "));
    let mut problems = Vec::new();
    match Xdg::new("reference")
        .and_then(|xdg| runs::run(binary, &[], &xdg, &ARGS, Place::default(), TIMEOUT))
    {
        Err(problem) => problems.push(format!("{line}: {problem}")),
        Ok(output) if !output.status.success() => {
            let mut problem = format!(
                "{line}: the run must exit with code 0, and it ends with {}",
                output.status
            );
            if !output.stderr.is_empty() {
                problem.push_str(" after writing this to standard error:\n");
                problem.push_str(&String::from_utf8_lossy(&output.stderr));
            }
            problems.push(problem);
        }
        Ok(output) => match crumbkit_markdown::render(&output.stdout) {
            Ok(reference) => compare(
                assert,
                &line,
                "reference",
                reference.text().to_owned(),
                file,
                &mut problems,
            ),
            Err(error) => {
                problems.push(format!("{line}: the reference cannot be written: {error}"));
            }
        },
    }
    tree::report("command reference rules", &problems);
}
