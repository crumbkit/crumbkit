//! What a run reports to the one observer that the process installed, such
//! as the usage log (ADR-0057).
//!
//! crumbkit reports each fact from the place that knows it: `parse` reports
//! the command line, `finish` reports the exit code and the bytes it wrote,
//! a child hand-over reports that a child took the terminal, and the tool
//! and crumbkit-aliases report through [`aliased`], [`tolerated`] and
//! [`ended`]. Each report goes to the [`Observer`] that [`observe`]
//! installed. Without one, a report reads one atomic value and does
//! nothing else.
//!
//! A tool does not usually call [`observe`] itself. The usage log's `start`
//! calls it, and a tool that wants its own observer installs it the same
//! way, first in `main`:
//!
//! ```
//! use std::sync::atomic::{AtomicU8, Ordering};
//!
//! use crumbkit::events::{Observer, observe};
//!
//! /// Keeps the exit code that the tool reported last.
//! struct LastExit(AtomicU8);
//!
//! impl Observer for LastExit {
//!     fn ended(&self, exit: u8, _out_bytes: Option<u64>) {
//!         self.0.store(exit, Ordering::Relaxed);
//!     }
//! }
//!
//! static LAST_EXIT: LastExit = LastExit(AtomicU8::new(0));
//!
//! // First in `main`, before `parse`.
//! let _installed = observe(&LAST_EXIT);
//! ```

use std::ffi::OsString;
use std::process::ExitCode;
use std::sync::OnceLock;

use clap::{Arg, ArgMatches, Command};

use crate::{Code, Spelling};

#[cfg(test)]
pub(crate) mod testing;
#[cfg(test)]
mod tests;

/// The observer of this process, which the first successful [`observe`]
/// sets.
static OBSERVER: OnceLock<&'static dyn Observer> = OnceLock::new();

/// What a run reports to the observer that [`observe`] installed.
///
/// crumbkit calls each method from the place that knows the fact, on the
/// thread that reports it. Every method has a default body that does
/// nothing, so an observer implements only the methods it needs, and a
/// method that a later crumbkit adds never breaks it (ADR-0002). A method
/// must return quickly and must not panic, because it runs inside `parse`,
/// `finish` or a child hand-over, before the tool's own code goes on.
pub trait Observer: Sync {
    /// `parse` received `args`, the whole command line with the program
    /// name first, for the tool's root `command`, and has not parsed it
    /// yet. `spelling` is how the tool spells its failure codes.
    fn typed(&self, command: &Command, args: &[OsString], spelling: Spelling) {
        let _ = (command, args, spelling);
    }

    /// clap accepted the command line against `command` into `matches`. A
    /// picker can make this happen more than once in a run, once for each
    /// command line that parses.
    fn parsed(&self, command: &Command, matches: &ArgMatches) {
        let _ = (command, matches);
    }

    /// A picker filled in `count` missing arguments before the command line
    /// parsed.
    fn picked(&self, count: usize) {
        let _ = count;
    }

    /// `parse` printed help for the command at `path` below `command`, the
    /// tool's root, and ends the run with `exit`. `long` says that the help
    /// was the long page, which `--help` and the `help` topics print.
    fn helped(&self, command: &Command, path: &[String], long: bool, exit: u8) {
        let _ = (command, path, long, exit);
    }

    /// clap refused the command line, and `parse` ends the run, as
    /// [`Refused`] describes.
    fn refused(&self, refused: &Refused<'_>) {
        let _ = refused;
    }

    /// `parse` ended the run with `exit` after it printed the version, after
    /// a picker's source failed, or after it refused a view flag that the
    /// command does not offer.
    fn exited(&self, exit: u8) {
        let _ = exit;
    }

    /// crumbkit printed a failure or a refusal whose code, in the tool's
    /// `spelling`, is `code`. A batch reports each failure it prints, and a
    /// listing that the output budget cut short reports the budget's code.
    fn failed(&self, spelling: Spelling, code: &Code) {
        let _ = (spelling, code);
    }

    /// `finish` or a stream returned, as [`Finished`] describes.
    fn finished(&self, finished: &Finished) {
        let _ = finished;
    }

    /// The tool, or crumbkit for a child process, ended the run with `exit`
    /// after writing `out_bytes` to standard output, as [`ended`]
    /// describes.
    fn ended(&self, exit: u8, out_bytes: Option<u64>) {
        let _ = (exit, out_bytes);
    }

    /// A personal alias expanded the command line, as [`aliased`]
    /// describes.
    fn aliased(&self) {}

    /// The run met the failure `code` and carried on past it, as
    /// [`tolerated`] describes.
    fn tolerated(&self, code: &'static Code) {
        let _ = code;
    }

    /// A child process is about to take the terminal (ADR-0042). From now
    /// on Ctrl-C belongs to the child, until [`Observer::taken_back`]. Only
    /// a build with the `hand-over` feature reports it.
    fn handed_over(&self) {}

    /// The child that held the terminal exited, and Ctrl-C belongs to the
    /// tool again. Only Windows reports it, because on Unix a Ctrl-C sent
    /// while the child ran can reach the tool after the child exits, so
    /// the terminal stays the child's until the tool ends (ADR-0042).
    fn taken_back(&self) {}

    /// A signal ended the run's child process. crumbkit then reports the
    /// child's exit with [`Observer::ended`].
    fn interrupted(&self) {}

    /// Reports whether the observer has already taken SIGINT, so a child
    /// hand-over installs no SIGINT handler of its own and leaves the
    /// signal to the observer. The default is false.
    fn handles_interrupt(&self) -> bool {
        false
    }

    /// Returns the environment variables that put a child process in this
    /// run's trace, which [`crate::trace::child_env`] returns. The default
    /// is none.
    fn child_env(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    /// Returns the run's trace id and its own id, which the spans of the
    /// `tracing` feature record (ADR-0030). The default is `None`, and the
    /// spans then record neither.
    fn trace_ids(&self) -> Option<TraceIds> {
        None
    }
}

/// A command line that clap refused, which [`Observer::refused`] receives.
#[non_exhaustive]
#[derive(Debug)]
pub struct Refused<'a> {
    /// The tool's root command after clap refused the line.
    command: &'a Command,
    /// The command path that `parse` walked.
    path: &'a [String],
    /// clap's error.
    error: &'a clap::Error,
    /// The exit code that `parse` returns.
    exit: u8,
}

impl<'a> Refused<'a> {
    /// Returns the refusal of `error` at `path` below `command`, with the
    /// exit code `exit`.
    pub(crate) fn new(
        command: &'a Command,
        path: &'a [String],
        error: &'a clap::Error,
        exit: u8,
    ) -> Self {
        Self {
            command,
            path,
            error,
            exit,
        }
    }

    /// Returns the tool's root command.
    #[must_use]
    pub fn command(&self) -> &'a Command {
        self.command
    }

    /// Returns the command path below the root that `parse` walked before
    /// clap refused the line, such as `["books", "list"]`. A word of the
    /// path can name no command, when the caller mistyped it.
    #[must_use]
    pub fn path(&self) -> &'a [String] {
        self.path
    }

    /// Returns clap's error.
    #[must_use]
    pub fn error(&self) -> &'a clap::Error {
        self.error
    }

    /// Returns the exit code that `parse` returns, 2 unless the tool's
    /// guide lists another for the usage code.
    #[must_use]
    pub fn exit(&self) -> u8 {
        self.exit
    }

    /// Returns the visible flag that clap suggested for a flag it refused,
    /// such as the argument of `--all` for `--al`, or `None` when clap
    /// suggested none.
    ///
    /// clap can suggest a hidden alias, such as `--max` of `--limit`, and
    /// this then returns the argument that the alias belongs to. A
    /// suggestion of a hidden flag, or of a hidden alias of one, returns
    /// `None`. It reads clap's suggestion only when called.
    #[must_use]
    pub fn nearest_flag(&self) -> Option<&'a Arg> {
        crate::parse::suggestion::nearest_flag(self.command, self.path, self.error)
    }
}

/// What `finish` or a stream reports when it returns, which
/// [`Observer::finished`] receives.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct Finished {
    /// The exit code that it returned.
    exit: u8,
    /// The bytes that it wrote to standard output.
    out_bytes: u64,
    /// Whether a listing left rows out, or the output budget cut the result
    /// or the stream short.
    truncated: bool,
    /// Whether a listing or a stream had no rows.
    empty: bool,
}

impl Finished {
    /// Returns the report of a run that returned `exit` after writing
    /// `out_bytes`.
    pub(crate) fn new(exit: u8, out_bytes: u64, truncated: bool, empty: bool) -> Self {
        Self {
            exit,
            out_bytes,
            truncated,
            empty,
        }
    }

    /// Returns the exit code that `finish` or the stream returned.
    #[must_use]
    pub fn exit(&self) -> u8 {
        self.exit
    }

    /// Returns how many bytes it wrote to standard output.
    #[must_use]
    pub fn out_bytes(&self) -> u64 {
        self.out_bytes
    }

    /// Reports whether a listing left rows out, as a `Listing` with a total
    /// above its rows does, or the output budget cut the result or the
    /// stream short.
    #[must_use]
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// Reports whether a listing or a stream had no rows. A result that is
    /// neither reports false.
    #[must_use]
    pub fn empty(&self) -> bool {
        self.empty
    }
}

/// The run's trace id and its own id, as lowercase hex, which an observer
/// gives the `tracing` spans through [`Observer::trace_ids`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceIds {
    /// The trace id, 32 hex digits.
    trace: String,
    /// The run's own id, 16 hex digits.
    run: String,
}

impl TraceIds {
    /// Returns the ids of a run with the trace id `trace` and its own id
    /// `run`. The spans record both as they are given.
    #[must_use]
    pub fn new(trace: String, run: String) -> Self {
        Self { trace, run }
    }

    /// Returns the trace id.
    #[must_use]
    pub fn trace(&self) -> &str {
        &self.trace
    }

    /// Returns the run's own id.
    #[must_use]
    pub fn run(&self) -> &str {
        &self.run
    }
}

/// Installs `observer` for the rest of the process, and returns whether it
/// did.
///
/// It installs nothing and returns false when an observer is already
/// installed, so the first call wins, as with `complete_with`. It also
/// installs nothing when the run answers a shell's completion request,
/// because a Tab press is no run of the tool (ADR-0048). The completion
/// variable is `COMPLETE`, or the name that the tool passed to
/// `complete_with` before it called `observe`.
pub fn observe(observer: &'static dyn Observer) -> bool {
    observe_in(&OBSERVER, observer, crate::complete::completing())
}

/// Installs `observer` in `slot` unless the run is `completing`, as
/// [`observe`] describes.
fn observe_in(
    slot: &OnceLock<&'static dyn Observer>,
    observer: &'static dyn Observer,
    completing: bool,
) -> bool {
    !completing && slot.set(observer).is_ok()
}

/// Returns the observer that [`observe`] installed, or `None`.
#[must_use]
pub fn observer() -> Option<&'static dyn Observer> {
    OBSERVER.get().copied()
}

/// Reports that a personal alias expanded the run's command line.
/// crumbkit-aliases' `expand` calls it, so the usage log's line holds
/// `"alias": true`. Without an observer it does nothing.
pub fn aliased() {
    if let Some(observer) = observer() {
        observer.aliased();
    }
}

/// Reports that the run met the failure `code` and recovered from it, such
/// as a lock that the tool waited out or a download that it retried.
///
/// ```
/// use crumbkit::Code;
///
/// static LOCKED: Code = Code::new("shelf", "catalogue", "locked");
///
/// // The tool waited for the lock and then read the catalogue.
/// crumbkit::events::tolerated(&LOCKED);
/// ```
///
/// The usage log lists the code under `tolerated` in the tool's spelling,
/// once, in the order the tool reported its codes, up to eight codes, and
/// `self usage` counts the runs that tolerated each code. A code that the
/// run then fails with belongs to [`finish`](crate::finish), which reports
/// it as the run's failure. Without an observer it does nothing.
pub fn tolerated(code: &'static Code) {
    if let Some(observer) = observer() {
        observer.tolerated(code);
    }
}

/// Reports that the tool ended the run with `exit` after writing
/// `out_bytes` to standard output, and returns `exit` as an `ExitCode` for
/// `main` to return.
///
/// A run that never reaches [`finish`](crate::finish), such as a stream
/// that the tool ends itself, a child process or the tool's own exit path,
/// calls it last, so the observer receives the real exit number, such as
/// 124, and the byte count. `out_bytes` is `None` when the tool did not
/// count them. Because it returns the code, the number that the tool
/// returns and the number that the observer receives are the same:
///
/// ```no_run
/// # use std::process::ExitCode;
/// fn main() -> ExitCode {
///     // The tool ran out of time after it streamed 2048 bytes.
///     crumbkit::events::ended(124, Some(2048))
/// }
/// ```
///
/// crumbkit gives 0, 1 and 2 their meanings unless the tool's guide moves a
/// failure to another code (ADR-0002), and the observer receives the number
/// as given. The usage log writes the run's line at once, so a tool that
/// then exits through its own path, such as `std::process::exit` on another
/// thread, still has its line. Without an observer it only returns the
/// code.
#[must_use]
pub fn ended(exit: u8, out_bytes: Option<u64>) -> ExitCode {
    if let Some(observer) = observer() {
        observer.ended(exit, out_bytes);
    }
    ExitCode::from(exit)
}

/// Reports [`Observer::typed`].
pub(crate) fn typed(command: &Command, args: &[OsString], spelling: Spelling) {
    if let Some(observer) = observer() {
        observer.typed(command, args, spelling);
    }
}

/// Reports [`Observer::parsed`].
pub(crate) fn parsed(command: &Command, matches: &ArgMatches) {
    if let Some(observer) = observer() {
        observer.parsed(command, matches);
    }
}

/// Reports [`Observer::picked`].
pub(crate) fn picked(count: usize) {
    if let Some(observer) = observer() {
        observer.picked(count);
    }
}

/// Reports [`Observer::helped`].
pub(crate) fn helped(command: &Command, path: &[String], long: bool, exit: u8) {
    if let Some(observer) = observer() {
        observer.helped(command, path, long, exit);
    }
}

/// Reports [`Observer::refused`] for `error` at `path` below `command`, with
/// the exit code `exit`.
pub(crate) fn refused(command: &Command, path: &[String], error: &clap::Error, exit: u8) {
    if let Some(observer) = observer() {
        observer.refused(&Refused::new(command, path, error, exit));
    }
}

/// Reports [`Observer::exited`].
pub(crate) fn exited(exit: u8) {
    if let Some(observer) = observer() {
        observer.exited(exit);
    }
}

/// Reports [`Observer::failed`].
pub(crate) fn failed(spelling: Spelling, code: &Code) {
    if let Some(observer) = observer() {
        observer.failed(spelling, code);
    }
}

/// Reports [`Observer::finished`] with the fields of [`Finished`].
pub(crate) fn finished(exit: u8, out_bytes: u64, truncated: bool, empty: bool) {
    if let Some(observer) = observer() {
        observer.finished(&Finished::new(exit, out_bytes, truncated, empty));
    }
}

/// Reports [`Observer::handed_over`].
#[cfg(feature = "hand-over")]
pub(crate) fn handed_over() {
    if let Some(observer) = observer() {
        observer.handed_over();
    }
}

/// Reports [`Observer::taken_back`].
#[cfg(feature = "hand-over")]
pub(crate) fn taken_back() {
    if let Some(observer) = observer() {
        observer.taken_back();
    }
}

/// Reports [`Observer::interrupted`].
#[cfg(feature = "hand-over")]
pub(crate) fn interrupted() {
    if let Some(observer) = observer() {
        observer.interrupted();
    }
}

/// Reports whether the observer has taken SIGINT, as
/// [`Observer::handles_interrupt`] describes. Without an observer it is
/// false.
#[cfg(feature = "hand-over")]
pub(crate) fn handles_interrupt() -> bool {
    observer().is_some_and(Observer::handles_interrupt)
}

/// Returns the observer's trace ids, or `None` without an observer.
#[cfg(feature = "tracing")]
pub(crate) fn trace_ids() -> Option<TraceIds> {
    observer().and_then(Observer::trace_ids)
}
