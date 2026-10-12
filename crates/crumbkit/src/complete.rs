//! `complete`, which answers a shell's completion request through
//! `clap_complete`'s dynamic completion (ADR-0048), and the rule that tells a
//! completion request from an ordinary run, which the usage log shares.

use std::ffi::OsStr;
#[cfg(feature = "complete")]
use std::ffi::OsString;
#[cfg(feature = "complete")]
use std::io::{self, Write};
#[cfg(feature = "complete")]
use std::path::{Path, PathBuf};
#[cfg(feature = "complete")]
use std::process::ExitCode;

#[cfg(feature = "complete")]
use clap::Command;
#[cfg(feature = "complete")]
use clap_complete::env::Shells;

#[cfg(feature = "complete")]
use crate::Sink;

#[cfg(test)]
mod tests;

/// The environment variable that asks a tool to complete its command line,
/// with the shell as its value, as `clap_complete` names it.
pub(crate) const VAR: &str = "COMPLETE";

/// Reports whether `value`, the value of [`VAR`] in the environment, asks
/// for completion. An unset or empty variable and `0` ask for none, as
/// `clap_complete` reads them.
pub(crate) fn requested(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| !value.is_empty() && value != "0")
}

/// Answers a shell's completion request when the environment variable
/// `COMPLETE` holds the shell's name, and returns the exit code for `main`
/// to return. It returns `None` for an ordinary run, in which `COMPLETE` is
/// unset, empty or `0`, and then builds nothing.
///
/// A tool's `main` calls it first, before
/// [`usage::start`](crate::usage), and passes the function that builds the
/// command the tool parses against:
///
/// ```no_run
/// use std::process::ExitCode;
///
/// use clap::{CommandFactory, Parser};
///
/// /// List the books in a small catalogue.
/// #[derive(Parser)]
/// #[command(name = "shelf")]
/// struct Cli {}
///
/// fn main() -> ExitCode {
///     if let Some(code) = crumbkit::complete(Cli::command) {
///         return code;
///     }
///     // Start the usage log, parse and run the command.
///     ExitCode::SUCCESS
/// }
/// ```
///
/// `complete` calls `command` only for a completion request, so an ordinary
/// run builds its command once, for parsing. A tool that hides or removes
/// commands at run time does so inside the function it passes here and to
/// [`parse`](crate::parse()), so completion offers what parsing accepts.
///
/// A person registers the tool's completion once in their shell. Each line
/// below is for a tool named `shelf`:
///
/// - Bash: `echo "source <(COMPLETE=bash shelf)" >> ~/.bashrc`
/// - Elvish: `echo "eval (E:COMPLETE=elvish shelf | slurp)" >> ~/.elvish/rc.elv`
/// - Fish: `echo "COMPLETE=fish shelf | source" >> ~/.config/fish/completions/shelf.fish`
/// - PowerShell: `echo '$env:COMPLETE = "powershell"; shelf | Out-String | Invoke-Expression; Remove-Item Env:\COMPLETE' >> $PROFILE`
/// - Zsh: `echo "source <(COMPLETE=zsh shelf)" >> ~/.zshrc`
///
/// It behaves as [`complete_to`] does with a sink over the process's
/// standard output, the process's arguments and its current directory.
///
/// The `complete` feature follows `clap_complete`'s dynamic completion, which
/// clap marks as unstable, so a crumbkit minor release may change what it
/// offers and how a shell registers it (ADR-0002).
#[cfg(feature = "complete")]
#[must_use]
pub fn complete(command: impl FnOnce() -> Command) -> Option<ExitCode> {
    #[expect(
        clippy::disallowed_methods,
        reason = "completion owns its environment reads, and COMPLETE names the shell that asks"
    )]
    let shell = std::env::var_os(VAR)?;
    if !requested(Some(&shell)) {
        return None;
    }
    #[expect(
        clippy::disallowed_methods,
        reason = "completion owns its environment reads, and the registration names the tool's path from the current directory"
    )]
    let current_dir = std::env::current_dir().ok();
    let mut sink = Sink::standard();
    Some(complete_to(
        &mut sink,
        command(),
        &shell,
        std::env::args_os(),
        current_dir.as_deref(),
    ))
}

/// Answers the completion request of `shell` for `command`, writes the
/// answer to the sink's output writer, and returns the exit code. `args` are
/// the process's arguments, the program name first, as in
/// [`parse_from`](crate::parse_from), and `current_dir` is the directory
/// that a relative program path starts from. Tests call it with a sink over
/// in-memory writers.
///
/// `shell` is the value of `COMPLETE`, such as `fish` or `/bin/zsh`, whose
/// file stem names one of `bash`, `elvish`, `fish`, `powershell` and `zsh`.
/// Arguments with no words after `--` ask for the shell's registration
/// script, which runs the program at its path for each completion. Words
/// after `--` are the shell's command line, and the answer is the shell's
/// list of candidates for the word at the cursor.
///
/// The candidates are those of `clap_complete`. They leave out every
/// subcommand, flag and alias that the tool hid, except that a word that
/// matches only hidden names completes to them, so `shelf books ls` still
/// completes when `ls` is a hidden alias of `list`. clap's `help`
/// subcommand is turned off, as [`parse_to`](crate::parse_to) turns it off.
///
/// A completion never prints a failure into the shell. An unknown shell, a
/// command line that `clap_complete` cannot complete and a failed write each
/// write nothing to the error writer and return exit code 1, and the output
/// writer receives the answer whole or not at all. A reader that closed the
/// pipe early gives exit code 0, as with help.
#[cfg(feature = "complete")]
pub fn complete_to<O, E, R, I, T>(
    sink: &mut Sink<O, E, R>,
    command: Command,
    shell: &OsStr,
    args: I,
    current_dir: Option<&Path>,
) -> ExitCode
where
    O: Write,
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut answer = Vec::new();
    let args = args.into_iter().map(Into::into).collect();
    if answer_into(&mut answer, command, shell, args, current_dir).is_err() {
        return ExitCode::FAILURE;
    }
    match sink.out.write_all(&answer).and_then(|()| sink.out.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

/// Writes the answer to the completion request of `shell` into `answer`, as
/// [`complete_to`] describes.
//
// RUST-10: this repeats clap_complete's `CompleteEnv::try_complete`, which
// writes to the process's standard output itself, so that the answer goes
// through crumbkit's sink. Unlike `try_complete`, it leaves `COMPLETE` set,
// because removing an environment variable needs unsafe code. A custom
// completer of the tool that runs another clap_complete tool as a child
// would make that child answer a completion request too, so such a
// completer removes the variable from its child's environment with
// `std::process::Command::env_remove`.
#[cfg(feature = "complete")]
fn answer_into(
    answer: &mut Vec<u8>,
    command: Command,
    shell: &OsStr,
    mut args: Vec<OsString>,
    current_dir: Option<&Path>,
) -> io::Result<()> {
    let shells = Shells::builtins();
    let name = Path::new(shell).file_stem().unwrap_or(shell);
    let shell = shells
        .completer(&name.to_string_lossy())
        .ok_or_else(|| io::Error::other("unknown shell"))?;
    let mut command = command.disable_help_subcommand(true);
    if args.is_empty() {
        return Err(io::Error::other("no program name"));
    }
    let program = args.remove(0);
    let words = args
        .iter()
        .position(|word| word == "--")
        .map_or(args.len(), |position| position + 1);
    args.drain(..words);
    if !args.is_empty() {
        return shell.write_complete(&mut command, args, current_dir, answer);
    }
    let mut program = PathBuf::from(program);
    if let Some(current_dir) = current_dir
        && program.components().count() > 1
    {
        program = current_dir.join(program);
    }
    let name = command.get_name();
    let bin = command.get_bin_name().unwrap_or(name);
    shell.write_registration(VAR, name, bin, &program.to_string_lossy(), answer)
}
