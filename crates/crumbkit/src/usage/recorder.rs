//! The process's one recorder, which collects what `parse`, `finish`, an
//! alias and the tool report during a run, and writes the run's line once.

use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use clap::{ArgMatches, Command};
use serde::Serialize;
#[cfg(feature = "hand-over")]
use signal_hook::consts::SIGINT;

use super::caller::{self, Caller};
use super::context::{self, Context};
use super::file::{self, LIMITS, Limits};
use super::key;
use super::platform::{self, Interrupts};
use super::record::{self, Hash, Help, Name, Record, Run, Section};
use super::shape;
use crate::{Code, Document, Spelling, exit};

#[cfg(test)]
mod tests;

/// The recorder of this process. A poisoned lock counts as no recorder.
static RECORDER: Mutex<State> = Mutex::new(State::Idle);

/// Whether the signal thread has taken SIGINT, which it does with either
/// [`Signals`] value.
static LISTENS: AtomicBool = AtomicBool::new(false);

/// The most tolerated codes that one line holds. A run that reports more
/// distinct codes keeps the first eight, so the line stays within its
/// limit of 1 KiB.
const MAX_TOLERATED: usize = 8;

/// How long the main thread or the signal thread waits for a line that the
/// other thread is writing, before `main` returns or the signal takes
/// effect. A write waits at most 100 ms for the lock, and a trim reads at
/// most about 10 MB and writes 5 MB, which takes a few milliseconds from the
/// page cache, so 500 ms covers the lock wait and a trim on a disk that
/// reads and writes about 40 MB a second. A person who presses Ctrl-C waits
/// at most this long for the tool to end.
const LINE_WAIT: Duration = Duration::from_millis(500);

/// Where the process's recorder is in its life.
#[expect(
    clippy::large_enum_variant,
    reason = "the one `State` lives in a static and never moves, so its size costs nothing"
)]
enum State {
    /// No `start` has run.
    Idle,
    /// `start` ran, and the run's line is not written yet.
    Recording(Recorder),
    /// A thread took the recorder and is writing the run's line.
    Writing,
    /// The run's line is written, or was dropped.
    Written,
}

/// What the usage log knows about the run so far.
struct Recorder {
    /// The name of the tool's root command, which `parse` gives. A run
    /// without it writes no line.
    tool: Option<Name>,
    /// The tool's version, when its command has one.
    version: Option<Name>,
    /// When `start` ran.
    started: Instant,
    /// The run's trace context, which `start` reads.
    context: Context,
    /// The words after the tool's name, which the line holds only as a
    /// keyed hash.
    words: Vec<OsString>,
    /// How the tool spells its failure codes, which `parse` gives.
    spelling: Option<Spelling>,
    /// The distinct codes of the failures that the tool recovered from, in
    /// the order it reported them, at most [`MAX_TOLERATED`].
    tolerated: Vec<&'static Code>,
    /// What the run reported.
    run: Run,
    /// Whether SIGINT or SIGTERM arrived in a run whose tool owns its
    /// signals. The line then says that the run was interrupted.
    signalled: bool,
}

/// Who handles SIGINT and SIGTERM while the usage log runs, which
/// [`start_with`] takes. On Windows the log takes only SIGINT, which Ctrl-C
/// raises, so these values apply to SIGINT alone there.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Signals {
    /// crumbkit handles both signals. On either one, it writes the run's
    /// line with `"interrupted": true` and then lets the signal end the
    /// process as it would have without the log. [`start`] chooses this.
    #[default]
    Crumbkit,
    /// The tool handles both signals with its own handler. On either one,
    /// crumbkit only notes that the run was interrupted and leaves the
    /// process running. The tool ends the run itself and reports its exit
    /// with [`ended`](crate::usage::ended), which writes the line with
    /// `"interrupted": true`. A tool that returns from `main` without
    /// calling `ended` gets its line from the guard.
    ///
    /// crumbkit's listener replaces the default action of both signals for
    /// the whole process. A tool that chooses `Tool` and installs no handler
    /// of its own therefore keeps running on Ctrl-C and on SIGTERM, and only
    /// SIGKILL ends it. A handler that the tool installs later through
    /// signal-hook runs beside crumbkit's listener. A handler that replaces
    /// the signal's action directly, as the ctrlc crate does, removes
    /// crumbkit's listener, and the line then records the tool's exit
    /// without `interrupted`.
    Tool,
}

/// The guard that [`start`] returns. It writes the run's line when it
/// drops, at the end of `main`, after the tool's output is flushed.
///
/// The guard of a second `start` in one process does nothing.
#[derive(Debug)]
#[must_use = "the guard writes the run's line when it drops, so a guard dropped at once records nothing"]
pub struct Log {
    /// Whether this guard's `start` installed the recorder.
    active: bool,
}

impl Drop for Log {
    fn drop(&mut self) {
        if self.active {
            write(&RECORDER, false);
        }
    }
}

/// Starts the usage log of this run and returns the guard that writes its
/// line.
///
/// A tool's `main` calls it first and keeps the guard until `main` returns:
///
/// ```no_run
/// # use std::process::ExitCode;
/// # fn run() -> ExitCode { ExitCode::SUCCESS }
/// fn main() -> ExitCode {
///     let _log = crumbkit::usage::start();
///     run()
/// }
/// ```
///
/// `start` notes the time, reads the caller's trace context from
/// TRACEPARENT and TRACESTATE, and installs the process's recorder, which
/// `parse`, `finish`, [`aliased`](super::aliased) and [`section`] report to.
/// `parse` gives the recorder the tool's name and version from the command
/// it parses against, so a run that ends before it reaches `parse` writes no
/// line. `start` also starts the thread that writes the line when the run
/// ends on SIGINT or SIGTERM, and then lets the signal end the process as it
/// would have. On Windows the thread takes only SIGINT, and the process
/// then ends with the exit code that Ctrl-C gives, `STATUS_CONTROL_C_EXIT`.
///
/// The run joins the caller's trace when TRACEPARENT is valid, and starts a
/// new trace otherwise, as [`crate::trace`] describes. The random ids come
/// from the operating system, and a run whose ids cannot be read installs no
/// recorder, so it writes no line and gives its children no variables.
///
/// A run whose `COMPLETE` variable asks for shell completion, as
/// `crumbkit::complete` of the `complete` feature reads it, installs no
/// recorder either, so a Tab press writes no line.
///
/// The line goes to `usage.jsonl` in the tool's directory under the XDG
/// state directory, such as `~/.local/state/shelf/usage.jsonl`. On Windows
/// the directory is `XDG_STATE_HOME` when it is an absolute path, and
/// otherwise `%LOCALAPPDATA%`, such as
/// `C:\Users\me\AppData\Local\shelf\usage.jsonl`. Any failure
/// drops the line without a word, and recording never changes the tool's
/// output or exit code (ADR-0029).
pub fn start() -> Log {
    start_with(Signals::Crumbkit)
}

/// Starts the usage log of this run as [`start`] does, with `signals`
/// saying who handles SIGINT and SIGTERM.
///
/// A tool with its own handler for either signal calls
/// `start_with(Signals::Tool)`, so the log never ends the process, and
/// reports its exit with [`ended`](crate::usage::ended), as
/// [`Signals::Tool`] describes.
pub fn start_with(signals: Signals) -> Log {
    let started = Instant::now();
    let active = !context::completing()
        && context::read()
            .is_some_and(|context| install(&RECORDER, Recorder::new(started, context)));
    if active {
        watch_signals(signals);
    }
    Log { active }
}

/// Sets the tool's own section of this run's line to `value`, under the id
/// of `document`, such as `{"schema":"shelf/usage-ext/v1","data":{...}}`.
/// A later call replaces the section, and without a started log it does
/// nothing.
///
/// The tool owns the section, and crumbkit cannot check it, so the tool
/// keeps argument values, URLs, ids, paths and messages out of it, as
/// crumbkit keeps them out of its own fields. The section counts towards the
/// line's limit of 1 KiB, and a longer line is dropped.
pub fn section<T: Serialize>(document: Document<T>, value: &T) {
    update(&RECORDER, |recorder| recorder.section(document, value));
}

/// Returns the variables that put a child process in this run's trace, as
/// [`crate::trace::child_env`] describes.
pub(crate) fn child_env() -> Vec<(&'static str, String)> {
    child_env_in(&RECORDER)
}

/// Returns the trace id and the run's id as hex, or `None` without a
/// recorder.
#[cfg(feature = "tracing")]
pub(crate) fn ids() -> Option<(String, String)> {
    let state = RECORDER.lock().ok()?;
    let State::Recording(recorder) = &*state else {
        return None;
    };
    let ids = &recorder.context.ids;
    Some((ids.trace.as_str().to_owned(), ids.run.as_str().to_owned()))
}

/// Records that a signal ended the run's child process, so the line says
/// that the run was interrupted.
#[cfg(feature = "hand-over")]
pub(crate) fn interrupted() {
    update(&RECORDER, |recorder| recorder.signalled = true);
}

/// Reports whether the signal thread has taken SIGINT.
#[cfg(feature = "hand-over")]
pub(crate) fn listens() -> bool {
    LISTENS.load(Ordering::SeqCst)
}

/// Records that a personal alias expanded.
pub(super) fn aliased() {
    update(&RECORDER, |recorder| recorder.run.alias = true);
}

/// Records the name and version of `command`, the tool's root command,
/// `args`, the command line that `parse` received, whose words after the
/// program name the line holds as a keyed hash, and `spelling`, in which
/// the line spells tolerated codes.
pub(crate) fn typed(command: &Command, args: &[OsString], spelling: Spelling) {
    update(&RECORDER, |recorder| {
        recorder.typed(command, args, spelling);
    });
}

/// Records that the run met the failure `code` and recovered from it, as
/// [`crate::usage::tolerated`] describes.
pub(super) fn tolerated(code: &'static Code) {
    update(&RECORDER, |recorder| recorder.tolerated(code));
}

/// Records the command path, the flags typed and the ids of a command line
/// that clap parsed against `command` into `matches`.
pub(crate) fn parsed(command: &Command, matches: &ArgMatches) {
    update(&RECORDER, |recorder| {
        shape::parsed(&mut recorder.run, command, matches);
    });
}

/// Records that `parse` printed help for the command at `path` below
/// `command`, the long page when `long`, and returned `exit`.
pub(crate) fn helped(command: &Command, path: &[String], long: bool, exit: ExitCode) {
    update(&RECORDER, |recorder| {
        recorder.helped(command, path, long, exit);
    });
}

/// Records that a picker filled in `count` missing arguments before the
/// command line parsed.
pub(crate) fn picked(count: usize) {
    update(&RECORDER, |recorder| recorder.run.picked = Some(count));
}

/// Records the exit code that `parse` returned after it printed the version,
/// after a picker's source failed, or after it refused an unoffered view flag.
pub(crate) fn exited(exit: u8) {
    update(&RECORDER, |recorder| {
        recorder.run.exit = Some(exit);
    });
}

/// Records a command line that clap refused with `error`, after `parse`
/// walked it to the command at `path` of `command`, and the `exit` code
/// that `parse` returns for it.
pub(crate) fn refused(command: &Command, path: &[String], error: &clap::Error, exit: u8) {
    update(&RECORDER, |recorder| {
        shape::refused(&mut recorder.run, command, path, &recorder.words, error);
        recorder.run.exit = Some(exit);
    });
}

/// Records the code of a failure or refusal that crumbkit printed, in
/// `spelling`. The first code of a run stays, so a batch records its first
/// failure.
pub(crate) fn failed(spelling: Spelling, code: &Code) {
    update(&RECORDER, |recorder| recorder.failed(spelling, code));
}

/// Records that the tool ended the run with `exit` after writing
/// `out_bytes` to standard output, and writes the run's line now, as
/// [`crate::usage::ended`] describes.
pub(crate) fn ended(exit: u8, out_bytes: Option<u64>) {
    update(&RECORDER, |recorder| {
        recorder.run.exit = Some(exit);
        recorder.run.out_bytes = out_bytes;
    });
    write(&RECORDER, false);
}

/// Records what `finish` did: the `exit` code it returned, the `out_bytes`
/// it wrote to standard output, whether a listing was `truncated` or the
/// output budget cut the result short, and whether a listing or a stream
/// was `empty`.
pub(crate) fn finished(exit: u8, out_bytes: u64, truncated: bool, empty: bool) {
    update(&RECORDER, |recorder| {
        recorder.run.exit = Some(exit);
        recorder.run.out_bytes = Some(out_bytes);
        recorder.run.truncated = truncated;
        recorder.run.empty = empty;
    });
}

impl Recorder {
    /// Returns the recorder of a run that started at `started` in the trace
    /// `context`.
    fn new(started: Instant, context: Context) -> Self {
        Self {
            tool: None,
            version: None,
            started,
            context,
            words: Vec::new(),
            spelling: None,
            tolerated: Vec::new(),
            run: Run::default(),
            signalled: false,
        }
    }

    /// Records the tool's command and command line, as [`typed`] describes.
    fn typed(&mut self, command: &Command, args: &[OsString], spelling: Spelling) {
        self.tool = Some(Name::command(command));
        self.version = Name::version(command);
        self.words = args.get(1..).unwrap_or_default().to_vec();
        self.spelling = Some(spelling);
    }

    /// Records a tolerated code, as [`tolerated`] describes. A code that the
    /// run already reported, and every code after the first
    /// [`MAX_TOLERATED`], is left out.
    fn tolerated(&mut self, code: &'static Code) {
        if self.tolerated.len() < MAX_TOLERATED && !self.tolerated.contains(&code) {
            self.tolerated.push(code);
        }
    }

    /// Records help for the command at `path`, as [`helped`] describes. A
    /// word of `path` that names no subcommand of `command` is left out.
    fn helped(&mut self, command: &Command, path: &[String], long: bool, exit: ExitCode) {
        self.run.command = Name::path(&shape::subcommands(command, path));
        self.run.help = Some(if long { Help::Long } else { Help::Short });
        self.run.exit = Some(exit::number(exit));
    }

    /// Records a failure's code, as [`failed`] describes.
    fn failed(&mut self, spelling: Spelling, code: &Code) {
        if self.run.code.is_none() {
            self.run.code = Some(Name::code(spelling, code));
        }
    }

    /// Sets the tool's section, as [`section`] describes.
    fn section<T: Serialize>(&mut self, document: Document<T>, value: &T) {
        if let Ok(data) = serde_json::to_value(value) {
            self.run.ext = Some(Section {
                schema: document.to_string(),
                data,
            });
        }
    }

    /// Returns the run's record, as it ended at `now`, with the hashes keyed
    /// with `key`, or `None` when the run never reached `parse`.
    fn finish(
        self,
        key: &[u8; 16],
        now: SystemTime,
        interrupted: bool,
        caller: &Caller,
    ) -> Option<Record> {
        let mut run = self.run;
        if let Some(spelling) = self.spelling {
            run.tolerated = self
                .tolerated
                .iter()
                .map(|code| Name::code(spelling, code))
                .collect();
        }
        Some(Record {
            schema: record::SCHEMA,
            tool: self.tool?,
            version: self.version,
            at: humantime::format_rfc3339_millis(now).to_string(),
            ms: u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX),
            caller: caller.name,
            session: Hash::new(key, [caller.session.as_bytes()]),
            args: Hash::new(key, self.words.iter().map(|word| word.as_encoded_bytes())),
            trace: self.context.ids,
            interrupted: interrupted || self.signalled,
            run,
        })
    }
}

/// Installs `recorder` in `slot`, and returns whether it did. It installs
/// none when `slot` already had one, or its lock is poisoned.
fn install(slot: &Mutex<State>, recorder: Recorder) -> bool {
    let Ok(mut state) = slot.lock() else {
        return false;
    };
    if !matches!(*state, State::Idle) {
        return false;
    }
    *state = State::Recording(recorder);
    true
}

/// Returns the variables that put a child process in the trace of the run
/// in `slot`. It returns none when `slot` has no recorder, or the run has
/// not reached `parse`, because such a run writes no line, and a child must
/// never name a parent that nothing records.
fn child_env_in(slot: &Mutex<State>) -> Vec<(&'static str, String)> {
    match slot.lock().as_deref() {
        Ok(State::Recording(recorder)) if recorder.tool.is_some() => recorder.context.child_env(),
        _ => Vec::new(),
    }
}

/// Applies `change` to the recorder in `slot`, and does nothing when there
/// is none.
fn update(slot: &Mutex<State>, change: impl FnOnce(&mut Recorder)) {
    if let Ok(mut state) = slot.lock()
        && let State::Recording(recorder) = &mut *state
    {
        change(recorder);
    }
}

/// Takes the recorder out of `slot`, so no other line is written for the
/// run, or returns `None` when there is none. The slot then says that the
/// line is being written, until [`written`] marks it written.
fn take(slot: &Mutex<State>) -> Option<Recorder> {
    let mut state = slot.lock().ok()?;
    match std::mem::replace(&mut *state, State::Writing) {
        State::Recording(recorder) => Some(recorder),
        other => {
            *state = other;
            None
        }
    }
}

/// Marks the line of the run in `slot` written, after the thread that took
/// the recorder wrote the line or dropped it.
fn written(slot: &Mutex<State>) {
    if let Ok(mut state) = slot.lock() {
        *state = State::Written;
    }
}

/// Waits while another thread writes the line of the run in `slot`, for at
/// most `bound`, and returns whether no write is in progress any more. It
/// checks every 5 ms, and a poisoned lock ends the wait.
fn wait_for_line(slot: &Mutex<State>, bound: Duration) -> bool {
    let started = Instant::now();
    loop {
        match slot.lock().as_deref() {
            Ok(State::Writing) => {}
            _ => return true,
        }
        if started.elapsed() >= bound {
            return false;
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// Writes the line of the run in `slot`, once per run. `interrupted` says
/// that a signal ended the run. When another thread is already writing the
/// line, it waits for that write, for at most [`LINE_WAIT`], so neither the
/// end of `main` nor a signal ends the process in the middle of the line.
fn write(slot: &Mutex<State>, interrupted: bool) {
    let Some(recorder) = take(slot) else {
        let _ = wait_for_line(slot, LINE_WAIT);
        return;
    };
    if let Some(state) = platform::state_home() {
        let now = SystemTime::now();
        // Every failure drops the line without a word (ADR-0029).
        let _ = save(
            &state,
            recorder,
            now,
            interrupted,
            &caller::detect(),
            &LIMITS,
        );
    }
    written(slot);
}

/// Writes the line of `recorder` to `usage.jsonl` in the tool's directory
/// under the `state` directory, or returns `None` when the run never
/// reached `parse` or any step fails.
fn save(
    state: &Path,
    recorder: Recorder,
    now: SystemTime,
    interrupted: bool,
    caller: &Caller,
    limits: &Limits,
) -> Option<()> {
    let dir = state.join(recorder.tool.as_ref()?.as_str());
    file::create_dir(&dir).ok()?;
    let key = key::key(&dir)?;
    let line = record::line(&recorder.finish(&key, now, interrupted, caller)?)?;
    file::append(&dir, &line, limits).ok()
}

/// Starts the thread that listens for SIGINT and SIGTERM, or for SIGINT
/// alone on Windows, and returns once the thread listens.
///
/// With [`Signals::Crumbkit`], the thread writes the line on the first
/// signal. When the main thread is already writing the line, the signal
/// thread waits for it, for at most [`LINE_WAIT`], as [`write`] describes.
/// After the line, the thread ends the process as the signal would have
/// without the log. With
/// [`Signals::Tool`], the thread notes each signal in the recorder and
/// leaves the process running. With the `hand-over` feature, the thread
/// leaves the process running on a SIGINT that arrives once a child has
/// held the terminal on Unix, or while a child holds it on Windows, because
/// Ctrl-C then belongs to the child (ADR-0042).
//
// RUST-10: the thread takes both signals from the start of the run. A
// process that started with one of them ignored now ends on it, such as a
// background job of a non-interactive shell, which starts with SIGINT
// ignored. A tool that handles one of them itself and starts the log with
// `start` ends on it too, and `Signals::Tool` exists for it. Reading
// each signal's disposition before taking it replaces this, which needs the
// `sigaction` call that crumbkit's ban on unsafe code rules out today.
fn watch_signals(owner: Signals) {
    let (listening, listens) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name("crumbkit-usage".to_owned())
        .spawn(move || {
            let interrupts = Interrupts::listen();
            // The flag is set before the main thread goes on, so a child
            // that `main` starts next already sees it.
            if interrupts.is_some() {
                LISTENS.store(true, Ordering::SeqCst);
            }
            let _ = listening.send(());
            let Some(mut interrupts) = interrupts else {
                return;
            };
            while let Some(signal) = interrupts.wait() {
                if owner == Signals::Tool {
                    update(&RECORDER, |recorder| recorder.signalled = true);
                    continue;
                }
                #[cfg(feature = "hand-over")]
                if signal == SIGINT && crate::child::held_terminal() {
                    continue;
                }
                write(&RECORDER, true);
                Interrupts::end_process(signal);
                return;
            }
        });
    if spawned.is_ok() {
        // The thread sends once it listens, and a thread that ended early
        // drops the sender, which ends the wait too.
        let _ = listens.recv();
    }
}
