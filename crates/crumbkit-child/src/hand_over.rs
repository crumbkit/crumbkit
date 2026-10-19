//! Running another program that owns the terminal, such as an editor, a
//! pager or a wrapped tool, as a command's whole response (ADR-0042).
//!
//! A command that runs such a program calls [`hand_over`] last, in place of
//! [`finish`](crumbkit::finish), and returns the exit code it gives. The
//! child gets the terminal: it reads standard input and writes standard
//! output and standard error directly. The tool ignores SIGINT and SIGQUIT
//! from the moment the child starts until the tool ends, or SIGINT and
//! SIGBREAK while the child runs on Windows, so Ctrl-C reaches only the
//! child, and the tool returns the child's exit code.
//!
//! ```no_run
//! use std::process::{Command, ExitCode};
//!
//! use crumbkit::Output;
//!
//! fn edit(output: Output, path: &str) -> ExitCode {
//!     let mut editor = Command::new("vi");
//!     editor.arg(path);
//!     crumbkit_child::hand_over(output, editor)
//! }
//! ```
//!
//! The child's output is text that the terminal shows, so `--json` and
//! `--fields` are refused with `<tool>::cli::usage` and the exit code of a
//! refused command line, which is 2 unless the guide lists another, before
//! the program starts. `--long` changes nothing. A tool that wants a
//! child's JSON runs the child itself and reads its output.
//!
//! A child that cannot run fails as `finish` prints a failure, with one of
//! these codes in the spelling that the tool's guide sets, or the tool's
//! own code when its guide renames it. Each exits with 1 unless an
//! [`Exit`](crumbkit::help::Exit) of the guide lists the code:
//!
//! - `<tool>::child::not_found`, when the program does not exist.
//! - `<tool>::child::not_started`, when the operating system refused to
//!   start the program or to wait for it.

use std::ffi::OsStr;
use std::io::{self, Write};
use std::process::{Command, ExitCode, ExitStatus, Stdio};

use crumbkit::{Failure as _, Output, Sink};

use crate::error::ChildError;
use crate::signals;

#[cfg(test)]
mod tests;

/// Hands the terminal to `child`, waits for it, and returns its exit code.
///
/// crumbkit sets the child's standard input, output and error to the
/// tool's own, and adds the variables of
/// [`trace::child_env`](crumbkit::trace::child_env), so the child joins the
/// run's trace. On Unix, the tool ignores SIGINT and SIGQUIT from the moment
/// the child starts until the tool ends, so Ctrl-C reaches only the child,
/// even when the signal reaches the tool after the child has exited. On
/// Windows, the tool ignores SIGINT and SIGBREAK, which Ctrl-C and
/// Ctrl-Break raise, until the child exits, and both then end the tool as
/// before. The child itself receives both signals with their default
/// action.
///
/// The exit code is the child's, so on this command 1 and 2 carry the
/// child's meanings. A child that a signal ended returns 128 plus the
/// signal's number, such as 130 for SIGINT, as a shell reports it. On
/// Windows, a child that exited with `STATUS_CONTROL_C_EXIT`, as Ctrl-C
/// leaves it, returns 130 too. crumbkit reports that number to the
/// observer with [`crumbkit::events::ended`], after
/// [`Observer::interrupted`](crumbkit::events::Observer::interrupted) when a
/// signal ended the child, so the usage log's line records both.
///
/// It runs no child and refuses with `<tool>::cli::usage` when the caller
/// passed `--json` or `--fields`, because the child's output is text that
/// the terminal shows. The refusal exits with the code of a refused command
/// line, which is 2 unless the guide lists another. The module
/// documentation lists the failures of a child that cannot start.
///
/// It takes `output` by value, as [`finish`](crumbkit::finish) does,
/// because the child's run is the last thing the tool prints (ADR-0094).
#[must_use]
pub fn hand_over(output: Output, child: Command) -> ExitCode {
    hand_over_to(&mut Sink::standard(), output, child)
}

/// Why `--json` and `--fields` are refused, which ends each refusal's
/// message.
const TEXT_ONLY: &str = "this command hands the terminal to another program";

/// Runs `child` as [`hand_over`] describes, with its refusals and failures
/// printed to `sink`.
fn hand_over_to<O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    mut output: Output,
    mut child: Command,
) -> ExitCode {
    if output.is_json() {
        let refused = ChildError::JsonRefused {
            program: program(&child),
        };
        return fail(sink, &mut output, &refused);
    }
    if let Some(code) = output.refuse_fields_for_text(sink, TEXT_ONLY) {
        return code;
    }
    child
        .envs(crumbkit::trace::child_env())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let program = program(&child);
    let status = signals::while_running(|| child.status());
    respond_text(sink, &mut output, program, status)
}

/// Returns the exit code of a child that held the terminal and ended with
/// `status`, and prints the failure of a child that could not run.
fn respond_text<O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    output: &mut Output,
    program: String,
    status: io::Result<ExitStatus>,
) -> ExitCode {
    match status {
        Ok(status) => {
            if signals::ended_by(status).is_some() {
                crumbkit::events::interrupted();
            }
            crumbkit::events::ended(exit_number(status), None)
        }
        Err(error) => fail(sink, output, &ChildError::start(program, error)),
    }
}

/// Prints `failure` as `finish` prints a failure, reports its exit code to
/// the observer, and returns it. The exit code is the one that the guide
/// gives the failure's code: the exit that lists it, 2 for a refused
/// command line, and otherwise 1 (ADR-0042).
fn fail<O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    output: &mut Output,
    failure: &ChildError,
) -> ExitCode {
    output.print_failure(sink, failure);
    crumbkit::events::ended(output.failure_exit(failure.code()), None)
}

/// Returns the name of the program that `child` runs, for messages.
fn program(child: &Command) -> String {
    display(child.get_program())
}

/// Returns `name` as text, with each byte that is not UTF-8 replaced.
fn display(name: &OsStr) -> String {
    name.to_string_lossy().into_owned()
}

/// Returns the exit number of a child that ended with `status`: 128 plus
/// the number of the signal that ended it, and otherwise its exit code. On
/// Windows a child that Ctrl-C ended counts as ended by SIGINT, so it gives
/// 130. Any other exit code outside 0 to 255, which only Windows gives,
/// counts as 1.
fn exit_number(status: ExitStatus) -> u8 {
    if let Some(signal) = signals::ended_by(status) {
        return u8::try_from(128 + signal).unwrap_or(1);
    }
    status
        .code()
        .map_or(1, |code| u8::try_from(code).unwrap_or(1))
}
