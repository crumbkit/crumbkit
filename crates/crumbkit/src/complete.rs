//! `complete`, which answers a shell's completion request through
//! `clap_complete`'s dynamic completion (ADR-0048), and the rule that tells a
//! completion request from an ordinary run, which keeps a completion request
//! from installing an observer.
//! [`complete_with`] declares the tool's own name for the completion
//! variable (ADR-0056), and [`complete_with_words`] builds the command from
//! the words that the caller typed.

use std::ffi::{OsStr, OsString};
#[cfg(feature = "complete")]
use std::io::{self, Write};
#[cfg(feature = "complete")]
use std::path::{Path, PathBuf};
#[cfg(feature = "complete")]
use std::process::ExitCode;
#[cfg(feature = "complete")]
use std::sync::OnceLock;

#[cfg(feature = "complete")]
use clap::Command;
#[cfg(feature = "complete")]
use clap_complete::env::Shells;

#[cfg(feature = "complete")]
use crate::Sink;
#[cfg(feature = "complete")]
use crate::help::Guide;
#[cfg(feature = "complete")]
use crate::parse::walk::own_names;

#[cfg(test)]
mod tests;

/// The environment variable that asks a tool to complete its command line,
/// with the shell as its value, as `clap_complete` names it.
pub(crate) const VAR: &str = "COMPLETE";

/// The completion variable that the tool declared with [`complete_with`],
/// which the first call of [`complete_with`] in the process sets.
#[cfg(feature = "complete")]
static DECLARED: OnceLock<&'static str> = OnceLock::new();

/// Returns the name of the completion variable of this process: the name
/// that the tool declared with [`complete_with`], or [`VAR`] when it
/// declared none.
fn variable() -> &'static str {
    #[cfg(feature = "complete")]
    if let Some(declared) = DECLARED.get() {
        return declared;
    }
    VAR
}

/// Reports whether the run answers a shell's completion request, so
/// [`crate::events::observe`] installs no observer (ADR-0048).
pub(crate) fn completing() -> bool {
    #[expect(
        clippy::disallowed_methods,
        reason = "the complete module owns the reads of the completion variable"
    )]
    completing_in(|name| std::env::var_os(name))
}

/// Reports whether the run answers a shell's completion request, with `env`
/// giving the value of an environment variable by its name. It reads the
/// completion variable that the tool declared with `complete_with`, or
/// `COMPLETE`.
fn completing_in(env: impl FnOnce(&str) -> Option<OsString>) -> bool {
    let value = env(variable());
    requested(value.as_deref())
}

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
/// A tool's `main` calls it first, before it installs a run observer with
/// [`events::observe`](crate::events::observe), and passes the function that builds the
/// command the tool parses against and the guide it parses with:
///
/// ```no_run
/// use std::process::ExitCode;
///
/// use clap::{CommandFactory, Parser};
/// use crumbkit::help::Guide;
///
/// /// List the books in a small catalogue.
/// #[derive(Parser)]
/// #[command(name = "shelf")]
/// struct Cli {}
///
/// const GUIDE: Guide = Guide::new();
///
/// fn main() -> ExitCode {
///     if let Some(code) = crumbkit::complete(Cli::command, &GUIDE) {
///         return code;
///     }
///     // Start the usage log, parse and run the command.
///     ExitCode::SUCCESS
/// }
/// ```
///
/// `complete` calls `command` only for a completion request, so an ordinary
/// run builds its command once, for parsing. Completion reads the guide's
/// [`Guide::dashed_forms`] declaration, so a dashed form completes when
/// `parse` accepts it. A tool that hides or removes
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
/// [`complete_with`] answers the same requests under a variable that the
/// tool names, and [`complete_with_words`] also passes the typed words to
/// the function that builds the command.
///
/// The `complete` feature follows `clap_complete`'s dynamic completion, which
/// clap marks as unstable, so a crumbkit minor release may change what it
/// offers and how a shell registers it (ADR-0002).
#[cfg(feature = "complete")]
#[must_use]
pub fn complete(command: impl FnOnce() -> Command, guide: &Guide) -> Option<ExitCode> {
    complete_with(VAR, command, guide)
}

/// Answers a shell's completion request as [`complete`] does, when the
/// environment variable named `variable` holds the shell's name. A tool
/// whose users already register its completion under a variable of its own,
/// such as `SHELF_COMPLETE`, declares that name here and keeps it
/// (ADR-0056):
///
/// ```no_run
/// use std::process::ExitCode;
///
/// use clap::{CommandFactory, Parser};
/// use crumbkit::help::Guide;
///
/// /// List the books in a small catalogue.
/// #[derive(Parser)]
/// #[command(name = "shelf")]
/// struct Cli {}
///
/// const GUIDE: Guide = Guide::new();
///
/// fn main() -> ExitCode {
///     if let Some(code) = crumbkit::complete_with("SHELF_COMPLETE", Cli::command, &GUIDE) {
///         return code;
///     }
///     // Start the usage log, parse and run the command.
///     ExitCode::SUCCESS
/// }
/// ```
///
/// The registration script that it writes sets `variable` in place of
/// `COMPLETE`, so the registration lines of [`complete`] work with the
/// variable's name in place of `COMPLETE`, such as
/// `echo "SHELF_COMPLETE=fish shelf | source" >> ~/.config/fish/completions/shelf.fish`.
/// The name is one that every shell accepts in an assignment: ASCII letters,
/// digits and underscores, starting with a letter or an underscore.
///
/// The first call in the process also declares `variable` as the process's
/// completion variable, and a later call with another name changes nothing.
/// [`events::observe`](crate::events::observe) reads that declaration and
/// installs no observer in a run whose declared variable asks for
/// completion, so `main` calls `complete_with` before it installs one. A
/// tool that never calls it keeps `COMPLETE`.
#[cfg(feature = "complete")]
#[must_use]
pub fn complete_with(
    variable: &'static str,
    command: impl FnOnce() -> Command,
    guide: &Guide,
) -> Option<ExitCode> {
    complete_with_words(variable, |_| command(), guide)
}

/// Answers a shell's completion request as [`complete_with`] does, and
/// passes `command` the words of the shell's command line after the program
/// name, so the tool builds its command for what the caller typed.
///
/// A tool whose completers depend on an earlier word, such as path
/// completers under the directory that a global `-C <DIR>` chooses, reads
/// that word here and builds its completers for it:
///
/// ```no_run
/// use std::ffi::OsString;
/// use std::process::ExitCode;
///
/// use clap::{Arg, Command};
/// use crumbkit::help::Guide;
///
/// /// Returns the directory after the first `-C` in `words`, if any.
/// fn directory(words: &[OsString]) -> Option<&OsString> {
///     let position = words.iter().position(|word| word == "-C")?;
///     words.get(position + 1)
/// }
///
/// fn shelf(words: &[OsString]) -> Command {
///     let base = directory(words).map(std::path::PathBuf::from);
///     // Build the path completers relative to `base` here.
///     Command::new("shelf").arg(Arg::new("dir").short('C').global(true))
/// }
///
/// const GUIDE: Guide = Guide::new();
///
/// fn main() -> ExitCode {
///     if let Some(code) = crumbkit::complete_with_words("COMPLETE", shelf, &GUIDE) {
///         return code;
///     }
///     // Start the usage log, parse and run the command.
///     ExitCode::SUCCESS
/// }
/// ```
///
/// The words are the shell's words in the order it sends them, including
/// the word at the cursor, which may be partial or empty. A shell may send
/// words after the cursor too. A request for the registration script passes
/// no words. A tool that passes `COMPLETE` as `variable` keeps the default
/// name, and [`complete`] and [`complete_with`] serve a tool whose command
/// does not depend on the typed words.
#[cfg(feature = "complete")]
#[must_use]
pub fn complete_with_words(
    variable: &'static str,
    command: impl FnOnce(&[OsString]) -> Command,
    guide: &Guide,
) -> Option<ExitCode> {
    // A second declaration keeps the first, so `events::observe` and the
    // registration agree within one run.
    let _ = DECLARED.set(variable);
    #[expect(
        clippy::disallowed_methods,
        reason = "completion owns its environment reads, and the completion variable names the shell that asks"
    )]
    let shell = std::env::var_os(variable)?;
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
        command,
        guide,
        variable,
        &shell,
        std::env::args_os(),
        current_dir.as_deref(),
    ))
}

/// Answers the completion request of `shell` for the command that
/// `command` builds from the typed words, as [`complete_with_words`]
/// describes, and which the tool parses with `guide`, writes the answer to
/// the sink's output writer, and returns the exit code.
/// `variable` is the name of the completion variable, `COMPLETE` or the
/// name that the tool passes to [`complete_with`], which a registration
/// script sets for each completion. `args` are
/// the process's arguments, the program name first, as in
/// [`parse_from`](crate::parse_from), and `current_dir` is the directory
/// that a relative program path starts from. Tests call it with a sink over
/// in-memory writers.
///
/// `shell` is the value of the completion variable, such as `fish` or `/bin/zsh`, whose
/// file stem names one of `bash`, `elvish`, `fish`, `powershell` and `zsh`.
/// Arguments with no words after `--` ask for the shell's registration
/// script, which runs the program at its path for each completion. Words
/// after `--` are the shell's command line, and the answer is the shell's
/// list of candidates for the word at the cursor.
///
/// The candidates are those of `clap_complete`. They leave out every
/// subcommand, flag and alias that the tool hid, except that a word that
/// matches only hidden names completes to them, so `shelf books ls` still
/// completes when `ls` is a hidden alias of `list`. When `guide` declares
/// [`Guide::dashed_forms`], a word before the cursor that is a command's
/// [`dashed_form`](crate::dashed_form) reaches that command, as it does in
/// [`parse_to`](crate::parse_to), and the candidates name each command by
/// the tool's own name. clap's `help`
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
    command: impl FnOnce(&[OsString]) -> Command,
    guide: &Guide,
    variable: &str,
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
    let dashed_forms = guide.get_dashed_forms();
    if answer_into(
        &mut answer,
        command,
        dashed_forms,
        variable,
        shell,
        args,
        current_dir,
    )
    .is_err()
    {
        return ExitCode::FAILURE;
    }
    match sink.out.write_all(&answer).and_then(|()| sink.out.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

/// Writes the answer to the completion request of `shell` into `answer`, as
/// [`complete_to`] describes. `dashed_forms` is the guide's
/// [`Guide::dashed_forms`] declaration.
//
// RUST-10: this repeats clap_complete's `CompleteEnv::try_complete`, which
// writes to the process's standard output itself, so that the answer goes
// through crumbkit's sink. Unlike `try_complete`, it leaves the completion
// variable set, because removing an environment variable needs unsafe code.
// A custom
// completer of the tool that runs another clap_complete tool as a child
// would make that child answer a completion request too, so such a
// completer removes the variable from its child's environment with
// `std::process::Command::env_remove`.
#[cfg(feature = "complete")]
fn answer_into(
    answer: &mut Vec<u8>,
    command: impl FnOnce(&[OsString]) -> Command,
    dashed_forms: bool,
    variable: &str,
    shell: &OsStr,
    mut args: Vec<OsString>,
    current_dir: Option<&Path>,
) -> io::Result<()> {
    let shells = Shells::builtins();
    let name = Path::new(shell).file_stem().unwrap_or(shell);
    let shell = shells
        .completer(&name.to_string_lossy())
        .ok_or_else(|| io::Error::other("unknown shell"))?;
    if args.is_empty() {
        return Err(io::Error::other("no program name"));
    }
    let program = args.remove(0);
    let words = args
        .iter()
        .position(|word| word == "--")
        .map_or(args.len(), |position| position + 1);
    args.drain(..words);
    // The shell's first word is the program as the caller typed it, and the
    // tool reads the words after it.
    let typed = args.get(1..).unwrap_or_default();
    let mut command = command(typed).disable_help_subcommand(true);
    if let Some(last) = args.len().checked_sub(1) {
        // A dashed form reaches its command as it does in `parse`, so the
        // words after it complete. The word at the cursor keeps its
        // spelling, because the shell matches candidates against it.
        //
        // RUST-10: the cursor is taken to be on the last word, as fish,
        // zsh, elvish and PowerShell send the line. Bash can complete a
        // word in the middle of the line, and a dashed form at its cursor
        // then reads as the tool's own name. Reading clap_complete's index
        // variable replaces this if that case matters.
        own_names(&command, dashed_forms, &mut args[..last]);
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
    shell.write_registration(variable, name, bin, &program.to_string_lossy(), answer)
}
