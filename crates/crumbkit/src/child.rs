//! Running another program that owns the terminal, such as an editor, a
//! pager or a wrapped tool, as a command's whole response (ADR-0042).
//!
//! A command that runs such a program calls [`hand_over`] or
//! [`hand_over_json`] last, in place of [`finish`](crate::finish), and
//! returns the exit code it gives. Without `--json`, the child gets the
//! terminal: it reads standard input and writes standard output and
//! standard error directly. The tool ignores SIGINT and SIGQUIT from the
//! moment the child starts until the tool ends, or SIGINT and SIGBREAK
//! while the child runs on Windows, so Ctrl-C reaches only the child, and
//! the tool returns the child's exit code.
//!
//! ```no_run
//! use std::process::{Command, ExitCode};
//!
//! use crumbkit::{Output, Spelling};
//!
//! fn edit(output: &mut Output, path: &str) -> ExitCode {
//!     let mut editor = Command::new("vi");
//!     editor.arg(path);
//!     crumbkit::child::hand_over(output, Spelling::COLONS, editor)
//! }
//! ```
//!
//! A child that prints a typed JSON document when asked declares how it is
//! asked and which document it prints with [`Json`]. Under `--json`,
//! [`hand_over_json`] adds the declared arguments, reads the child's standard
//! output as the document's type, and prints it as `finish` prints a result,
//! inside the usual envelope. A child without a JSON form refuses `--json`.
//!
//! Every failure prints as `finish` prints one, with one of these codes in
//! the tool's spelling:
//!
//! - `crumbkit::child::no_json`, exit code 2, when the caller passed
//!   `--json` to a command whose child has no JSON form.
//! - `crumbkit::child::not_found`, exit code 1, when the program does not
//!   exist.
//! - `crumbkit::child::not_started`, exit code 1, when the operating system
//!   refused to start the program or to wait for it.
//! - `crumbkit::child::bad_json`, exit code 1, when the child's output under
//!   `--json` is not the declared document.
//! - `crumbkit::child::failed`, with the child's exit code, when the child
//!   failed under `--json`.
//!
//! `--fields` without `--json` is refused with `crumbkit::cli::usage` and
//! exit code 2, because the child's own text has no columns to select, and
//! `--long` changes nothing.

mod error;
mod signals;

use std::ffi::OsStr;
use std::io::{self, Write};
use std::process::{self, Command, ExitCode, ExitStatus, Stdio};

use serde::de::DeserializeOwned;

use self::error::ChildError;
use crate::{Document, Output, Rows, Sink, Spelling};

#[cfg(test)]
mod tests;

#[cfg(feature = "usage-log")]
pub(crate) use self::signals::held_terminal;

/// How a child asks for JSON, and the document it then prints.
///
/// A tool declares one constant for each child with a JSON form and passes
/// it to [`hand_over_json`]:
///
/// ```
/// # #[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
/// # struct Commit { id: String }
/// # impl crumbkit::Row for Commit { const COLUMNS: &'static [&'static str] = &["id"]; }
/// use crumbkit::Document;
/// use crumbkit::child::Json;
///
/// const LOG_LIST: Document<Vec<Commit>> =
///     Document::new("vault", "log-list", 1, "The commits of the vault.");
/// const LOG_JSON: Json<Vec<Commit>> = Json::new(&["--format", "json"], LOG_LIST);
/// ```
#[derive(Debug)]
pub struct Json<T> {
    /// The arguments that make the child print JSON, which go after the
    /// arguments the tool gave the child.
    args: &'static [&'static str],
    /// The document whose type the child's output is read as, and under
    /// whose id it prints.
    document: Document<T>,
}

impl<T> Json<T> {
    /// Returns the JSON form of a child that prints a `T` when it receives
    /// `args` after its other arguments. `crumbkit::finish` prints the value
    /// under the id of `document`.
    #[must_use]
    pub const fn new(args: &'static [&'static str], document: Document<T>) -> Self {
        Self { args, document }
    }
}

// `Json<T>` is `Copy` for every `T`, as `Document<T>` is, because it holds
// no `T`. A derive would require `T: Copy`.
impl<T> Clone for Json<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Json<T> {}

/// Hands the terminal to `child`, waits for it, and returns its exit code,
/// for a child with no JSON form.
///
/// crumbkit sets the child's standard input, output and error to the
/// tool's own, and adds the variables of
/// [`trace::child_env`](crate::trace::child_env), so the child joins the
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
/// leaves it, returns 130 too. With the usage log, the run's line records
/// that number, and `"interrupted": true` when a signal ended the child.
///
/// It runs no child and fails with `crumbkit::child::no_json` and exit
/// code 2 when the caller passed `--json`, and with `crumbkit::cli::usage`
/// and exit code 2 when the caller passed `--fields`. The module
/// documentation lists the failures of a child that cannot start.
pub fn hand_over(output: &mut Output, spelling: Spelling, child: Command) -> ExitCode {
    hand_over_to(&mut Sink::standard(), output, spelling, child)
}

/// Runs `child` as [`hand_over`] describes, with its failures printed to
/// `sink`.
fn hand_over_to<O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    output: &mut Output,
    spelling: Spelling,
    child: Command,
) -> ExitCode {
    if output.is_json() {
        let failure = ChildError::NoJson(program(&child));
        return fail(sink, output, spelling, &failure, 2);
    }
    terminal(sink, output, spelling, child)
}

/// Runs `child` as [`hand_over`] does without `--json`, and as `finish` does
/// with `--json`: crumbkit adds the arguments of `json`, reads the child's
/// standard output as a `T` and prints it in the format that `output`
/// chooses, under the id of `json`'s document.
///
/// Under `--json`, the child's standard output is a pipe that crumbkit reads
/// to its end and holds in memory. Its standard input and standard error
/// stay the tool's, so the child can still ask a question or report a
/// problem. `--fields`, `--long` and `--full` shape the value as they shape
/// any result, and a `--fields` name that `T` lacks is refused before the
/// child runs.
///
/// Under `--json`, a child that exits with a code other than 0 fails with
/// `crumbkit::child::failed` and returns the child's code, and its standard
/// output is not read as the document. A child whose standard output is not
/// a `T` fails with `crumbkit::child::bad_json` and exit code 1.
pub fn hand_over_json<T: Rows + DeserializeOwned>(
    output: &mut Output,
    spelling: Spelling,
    child: Command,
    json: Json<T>,
) -> ExitCode {
    hand_over_json_to(&mut Sink::standard(), output, spelling, child, json)
}

/// Runs `child` as [`hand_over_json`] describes, with its result and
/// failures printed to `sink`.
fn hand_over_json_to<T: Rows + DeserializeOwned, O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    output: &mut Output,
    spelling: Spelling,
    mut child: Command,
    json: Json<T>,
) -> ExitCode {
    if !output.is_json() {
        return terminal(sink, output, spelling, child);
    }
    if let Some(code) = output.refuse_fields::<T, _, _>(sink, spelling) {
        return code;
    }
    child
        .args(json.args)
        .envs(crate::trace::child_env())
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let program = program(&child);
    let captured = signals::while_running(|| child.output());
    respond_json(sink, output, spelling, json.document, program, captured)
}

/// Hands the terminal to `child` and returns its exit code, after refusing
/// `--fields`.
fn terminal<O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    output: &mut Output,
    spelling: Spelling,
    mut child: Command,
) -> ExitCode {
    if let Some(code) = output.refuse_fields_for_child(sink, spelling) {
        return code;
    }
    child
        .envs(crate::trace::child_env())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let program = program(&child);
    let status = signals::while_running(|| child.status());
    respond_text(sink, output, spelling, program, status)
}

/// Returns the exit code of a child that held the terminal and ended with
/// `status`, and prints the failure of a child that could not run.
fn respond_text<O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    output: &mut Output,
    spelling: Spelling,
    program: String,
    status: io::Result<ExitStatus>,
) -> ExitCode {
    match status {
        Ok(status) => {
            if signals::ended_by(status).is_some() {
                crate::usage::interrupted();
            }
            crate::usage::ended(exit_number(status), None)
        }
        Err(error) => fail(
            sink,
            output,
            spelling,
            &ChildError::start(program, error),
            1,
        ),
    }
}

/// Prints the document that a child printed under `--json`, or the failure
/// of a child that could not run, failed or printed something else, and
/// returns the exit code.
fn respond_json<T: Rows + DeserializeOwned, O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    output: &mut Output,
    spelling: Spelling,
    document: Document<T>,
    program: String,
    captured: io::Result<process::Output>,
) -> ExitCode {
    let captured = match captured {
        Ok(captured) => captured,
        Err(error) => {
            return fail(
                sink,
                output,
                spelling,
                &ChildError::start(program, error),
                1,
            );
        }
    };
    if !captured.status.success() {
        if signals::ended_by(captured.status).is_some() {
            crate::usage::interrupted();
        }
        let exit = exit_number(captured.status);
        return fail(
            sink,
            output,
            spelling,
            &ChildError::Failed { program, exit },
            exit,
        );
    }
    match serde_json::from_slice::<T>(&captured.stdout) {
        Ok(value) => {
            let result: Result<T, ChildError> = Ok(value);
            crate::finish_to(sink, output, spelling, document, result)
        }
        Err(error) => {
            let failure = ChildError::BadJson {
                program,
                document: document.to_string(),
                error,
            };
            fail(sink, output, spelling, &failure, 1)
        }
    }
}

/// Prints `failure` as `finish` prints a failure, records `exit` in the
/// usage log, and returns it.
fn fail<O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    output: &mut Output,
    spelling: Spelling,
    failure: &ChildError,
    exit: u8,
) -> ExitCode {
    output.print_failure(sink, spelling, failure);
    crate::usage::ended(exit, None)
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
