//! The process's one recorder, which collects what `parse`, `finish`, an
//! alias and the tool report during a run, and writes the run's line once.

use std::ffi::OsString;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use clap::Command;
use signal_hook::consts::SIGINT;

use super::caller::{self, Caller};
use super::context::{self, Context};
use super::file::{self, LIMITS, Limits};
use super::key;
use super::observer::{self, UsageLog};
use super::platform::{self, Interrupts};
use super::record::{self, Hash, Help, Name, Record, Run};
use super::shape;
use crumbkit::events::TraceIds;
use crumbkit::{Code, Spelling, ToolName};

#[cfg(test)]
mod tests;

/// The recorder of this process. A poisoned lock counts as no recorder.
static RECORDER: Mutex<State> = Mutex::new(State::Idle);

/// Whether a `start` in this process has already tried to install the usage
/// log's observer, so a later `start` installs it no second time and the
/// recorder hears each report once.
static OBSERVING: AtomicBool = AtomicBool::new(false);

/// Whether the signal thread has taken SIGINT.
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
pub(super) struct Recorder {
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
    pub(super) words: Vec<OsString>,
    /// How the tool spells its failure codes, which `parse` gives.
    spelling: Option<Spelling>,
    /// The distinct codes of the failures that the tool recovered from, in
    /// the order it reported them, at most [`MAX_TOLERATED`].
    tolerated: Vec<&'static Code>,
    /// What the run reported.
    pub(super) run: Run,
    /// Whether a signal ended a child that held the terminal, which the
    /// observer reports. The line then says that the run was interrupted.
    pub(super) signalled: bool,
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
///     let _log = crumbkit_usage::start();
///     run()
/// }
/// ```
///
/// `start` notes the time, reads the caller's trace context from
/// TRACEPARENT and TRACESTATE, and installs the process's recorder, which
/// `parse`, `finish` and [`crumbkit::events::aliased`] report to. `parse`
/// gives the recorder the tool's name and version from the command it
/// parses against, so a run that ends before it reaches `parse` writes no
/// line. `start` also starts the thread that writes the line with
/// `"interrupted": true` when the run ends on SIGINT or SIGTERM, and then
/// lets the signal end the process as it would have. On Windows the thread
/// takes only SIGINT, and the process then ends with the exit code that
/// Ctrl-C gives, `STATUS_CONTROL_C_EXIT`.
///
/// The thread replaces the default action of both signals for the whole
/// process. A handler that the tool installs later through signal-hook runs
/// beside the thread, which still ends the process. A handler that replaces
/// the signal's action directly, as the ctrlc crate does, removes the
/// thread's listener, and the line then records the tool's exit without
/// `interrupted`. A tool that handles either signal itself keeps its own
/// handler and runs without the log.
///
/// The run joins the caller's trace when TRACEPARENT is valid, and starts a
/// new trace otherwise, as [`crumbkit::trace`] describes. The random ids come
/// from the operating system, and a run whose ids cannot be read installs no
/// recorder, so it writes no line and gives its children no variables.
///
/// `start` installs the usage log as one of the process's observers with
/// [`crumbkit::events::observe`], beside any observer that the tool installed
/// before it. `observe` installs nothing in a run that answers a shell's
/// completion request, so a Tab press writes no line (ADR-0048), and
/// nothing when the process already holds four observers. A run that then
/// installs no observer installs no recorder either, and its guard writes
/// nothing. Only the first `start` in a process installs the observer, so
/// a second `start` installs nothing and its guard writes nothing.
///
/// The line goes to `usage.jsonl` in the tool's directory under the XDG
/// state directory, such as `~/.local/state/shelf/usage.jsonl`. On Windows
/// the directory is `XDG_STATE_HOME` when it is an absolute path, and
/// otherwise `%LOCALAPPDATA%`, such as
/// `C:\Users\me\AppData\Local\shelf\usage.jsonl`. Any failure
/// drops the line without a word, and recording never changes the tool's
/// output or exit code (ADR-0029).
pub fn start() -> Log {
    let started = Instant::now();
    let first = !OBSERVING.swap(true, Ordering::SeqCst);
    let active = first
        && crumbkit::events::observe(&UsageLog)
        && context::read()
            .is_some_and(|context| install(&RECORDER, Recorder::new(started, context)));
    if active {
        watch_signals();
    }
    Log { active }
}

/// Applies `change` to the process's recorder, and does nothing without
/// one. The usage log's observer reports through it.
pub(super) fn report(change: impl FnOnce(&mut Recorder)) {
    update(&RECORDER, change);
}

/// Records that the tool ended the run with `exit` after writing
/// `out_bytes` to standard output, and writes the run's line now, as
/// [`crumbkit::events::ended`] describes.
pub(super) fn ended(exit: u8, out_bytes: Option<u64>) {
    report(|recorder| {
        recorder.run.exit = Some(exit);
        recorder.run.out_bytes = out_bytes;
    });
    write(&RECORDER, false);
}

/// Returns the variables that put a child process in this run's trace, as
/// [`crumbkit::trace::child_env`] describes.
pub(super) fn child_env() -> Vec<(&'static str, String)> {
    child_env_in(&RECORDER)
}

/// Returns the trace id and the run's id, or `None` without a recorder.
pub(super) fn ids() -> Option<TraceIds> {
    let state = RECORDER.lock().ok()?;
    let State::Recording(recorder) = &*state else {
        return None;
    };
    let ids = &recorder.context.ids;
    Some(TraceIds::new(
        ids.trace.as_str().to_owned(),
        ids.run.as_str().to_owned(),
    ))
}

/// Reports whether the signal thread has taken SIGINT.
pub(super) fn listens() -> bool {
    LISTENS.load(Ordering::SeqCst)
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

    /// Records the name and version of `command`, the tool's root command,
    /// `args`, the command line that `parse` received, whose words after the
    /// program name the line holds as a keyed hash, and `spelling`, in which
    /// the line spells tolerated codes.
    pub(super) fn typed(&mut self, command: &Command, args: &[OsString], spelling: Spelling) {
        self.tool = Some(Name::command(command));
        self.version = Name::version(command);
        self.words = args.get(1..).unwrap_or_default().to_vec();
        self.spelling = Some(spelling);
    }

    /// Records a code that the run recovered from, as
    /// [`crumbkit::events::tolerated`] describes. A code that the run already
    /// reported, and every code after the first [`MAX_TOLERATED`], is left
    /// out.
    pub(super) fn tolerated(&mut self, code: &'static Code) {
        if self.tolerated.len() < MAX_TOLERATED && !self.tolerated.contains(&code) {
            self.tolerated.push(code);
        }
    }

    /// Records that `parse` printed help for the command at `path` below
    /// `command`, the long page when `long`, and returned `exit`. A word of
    /// `path` that names no subcommand of `command` is left out.
    pub(super) fn helped(&mut self, command: &Command, path: &[String], long: bool, exit: u8) {
        self.run.command = Name::path(&shape::subcommands(command, path));
        self.run.help = Some(if long { Help::Long } else { Help::Short });
        self.run.exit = Some(exit);
    }

    /// Records the code of a failure or refusal that crumbkit printed, in
    /// `spelling`. The first code of a run stays, so a batch records its
    /// first failure.
    pub(super) fn failed(&mut self, spelling: Spelling, code: &Code) {
        if self.run.code.is_none() {
            self.run.code = Some(Name::code(spelling, code));
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
            schema: self
                .spelling
                .map_or(ToolName::CRUMBKIT, Spelling::tool)
                .schema_id(record::SCHEMA),
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
/// The thread writes the line on the first signal. When the main thread is
/// already writing the line, the signal thread waits for it, for at most
/// [`LINE_WAIT`], as [`write`] describes. After the line, the thread ends
/// the process as the signal would have without the log. The thread leaves
/// the process running on a SIGINT that arrives once crumbkit reported that
/// a child took the terminal, and on Windows before it reported that the
/// child gave the terminal back, because Ctrl-C then belongs to the child
/// (ADR-0042).
//
// RUST-10: the thread takes both signals from the start of the run. A
// process that started with one of them ignored now ends on it, such as a
// background job of a non-interactive shell, which starts with SIGINT
// ignored. A tool that handles one of them itself ends on it too, so such a
// tool runs without the log (ADR-0095). Reading each signal's disposition
// before taking it replaces this, which needs the `sigaction` call that
// crumbkit's ban on unsafe code rules out today.
fn watch_signals() {
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
                if signal == SIGINT && observer::held_terminal() {
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
