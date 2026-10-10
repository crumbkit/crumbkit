//! The process's one recorder, which collects what `parse`, `finish`, an
//! alias and the tool report during a run, and writes the run's line once.

use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;
use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::{Instant, SystemTime};

use clap::{ArgMatches, Command};
use etcetera::BaseStrategy as _;
use serde::Serialize;
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use signal_hook::low_level::emulate_default_handler;

use super::caller::{self, Caller};
use super::file::{self, LIMITS, Limits};
use super::key;
use super::record::{self, Hash, Help, Name, Record, Run, Section};
use super::shape;
use crate::{Code, Document, Spelling};

#[cfg(test)]
mod tests;

/// The recorder of this process. A poisoned lock counts as no recorder.
static RECORDER: Mutex<State> = Mutex::new(State::Idle);

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
    /// The run's line is written, or was dropped.
    Written,
}

/// What the usage log knows about the run so far.
struct Recorder {
    /// The name of the tool's root command.
    tool: Name,
    /// The tool's version, when its command has one.
    version: Option<Name>,
    /// When `start` ran.
    started: Instant,
    /// The words after the tool's name, which the line holds only as a
    /// keyed hash.
    words: Vec<OsString>,
    /// What the run reported.
    run: Run,
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
            write(false);
        }
    }
}

/// Starts the usage log of this run and returns the guard that writes its
/// line.
///
/// A tool's `main` calls it first, with the tool's command, and keeps the
/// guard until `main` returns:
///
/// ```no_run
/// # use std::process::ExitCode;
/// # use clap::{CommandFactory, Parser};
/// # #[derive(Parser)]
/// # struct Cli {}
/// # fn run() -> ExitCode { ExitCode::SUCCESS }
/// fn main() -> ExitCode {
///     let _log = crumbkit::usage::start(&Cli::command());
///     run()
/// }
/// ```
///
/// `start` reads the tool's name and version from `command`, notes the
/// time, and installs the process's recorder, which `parse`, `finish`,
/// [`aliased`](super::aliased) and [`section`] report to. It also starts the
/// thread that writes the line when the run ends on SIGINT, SIGTERM or
/// SIGHUP, and then lets the signal end the process as it would have.
///
/// The line goes to `usage.jsonl` in the tool's directory under the XDG
/// state directory, such as `~/.local/state/shelf/usage.jsonl`. Any failure
/// drops the line without a word, and recording never changes the tool's
/// output or exit code (ADR-0029).
pub fn start(command: &Command) -> Log {
    let active = install(&RECORDER, command, Instant::now());
    if active {
        watch_signals();
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

/// Records that a personal alias expanded.
pub(super) fn aliased() {
    update(&RECORDER, |recorder| recorder.run.alias = true);
}

/// Records `args`, the command line that `parse` received, whose words
/// after the program name the line holds as a keyed hash.
pub(crate) fn typed(args: &[OsString]) {
    update(&RECORDER, |recorder| {
        recorder.words = args.get(1..).unwrap_or_default().to_vec();
    });
}

/// Records the command path, the flags typed and the ids of a command line
/// that clap parsed against `command` into `matches`.
pub(crate) fn parsed(command: &Command, matches: &ArgMatches) {
    update(&RECORDER, |recorder| {
        shape::parsed(&mut recorder.run, command, matches);
    });
}

/// Records that `parse` printed help for the command at `path`, the long
/// page when `long`, and returned `exit`.
pub(crate) fn helped(path: &[String], long: bool, exit: ExitCode) {
    update(&RECORDER, |recorder| recorder.helped(path, long, exit));
}

/// Records the exit code that `parse` returned after it printed the
/// version.
pub(crate) fn exited(exit: ExitCode) {
    update(&RECORDER, |recorder| recorder.run.exit = Some(number(exit)));
}

/// Records a command line that clap refused with `error`, after `parse`
/// walked it to the command at `path` of `command`. `parse` returns 2.
pub(crate) fn refused(command: &Command, path: &[String], error: &clap::Error) {
    update(&RECORDER, |recorder| {
        shape::refused(&mut recorder.run, command, path, &recorder.words, error);
        recorder.run.exit = Some(2);
    });
}

/// Records the code of a failure or refusal that crumbkit printed, in
/// `spelling`. The first code of a run stays, so a batch records its first
/// failure.
pub(crate) fn failed(spelling: Spelling, code: &Code) {
    update(&RECORDER, |recorder| recorder.failed(spelling, code));
}

/// Records what `finish` did: the `exit` code it returned, the `out_bytes`
/// it wrote to standard output, and whether a listing was `truncated`.
pub(crate) fn finished(exit: ExitCode, out_bytes: u64, truncated: bool) {
    update(&RECORDER, |recorder| {
        recorder.run.exit = Some(number(exit));
        recorder.run.out_bytes = Some(out_bytes);
        recorder.run.truncated = truncated;
    });
}

impl Recorder {
    /// Returns the recorder of a run of `command` that started at `started`.
    fn new(command: &Command, started: Instant) -> Self {
        Self {
            tool: Name::new(command.get_name()),
            version: command.get_version().map(Name::new),
            started,
            words: Vec::new(),
            run: Run::default(),
        }
    }

    /// Records help for the command at `path`, as [`helped`] describes.
    fn helped(&mut self, path: &[String], long: bool, exit: ExitCode) {
        self.run.command = (!path.is_empty()).then(|| Name::new(path.join(" ")));
        self.run.help = Some(if long { Help::Long } else { Help::Short });
        self.run.exit = Some(number(exit));
    }

    /// Records a failure's code, as [`failed`] describes.
    fn failed(&mut self, spelling: Spelling, code: &Code) {
        if self.run.code.is_none() {
            self.run.code = Some(Name::new(spelling.spell(code)));
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
    /// with `key`.
    fn finish(self, key: &[u8; 16], now: SystemTime, interrupted: bool, caller: &Caller) -> Record {
        Record {
            schema: record::SCHEMA,
            tool: self.tool,
            version: self.version,
            at: humantime::format_rfc3339_millis(now).to_string(),
            ms: u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX),
            caller: caller.name,
            session: Hash::new(key, [caller.session.as_bytes()]),
            args: Hash::new(key, self.words.iter().map(|word| word.as_encoded_bytes())),
            interrupted,
            run: self.run,
        }
    }
}

/// Installs a recorder for `command` in `slot`, and returns whether it did.
/// It installs none when `slot` already had one, or its lock is poisoned.
fn install(slot: &Mutex<State>, command: &Command, started: Instant) -> bool {
    let Ok(mut state) = slot.lock() else {
        return false;
    };
    if !matches!(*state, State::Idle) {
        return false;
    }
    *state = State::Recording(Recorder::new(command, started));
    true
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
/// run, or returns `None` when there is none.
fn take(slot: &Mutex<State>) -> Option<Recorder> {
    let mut state = slot.lock().ok()?;
    match std::mem::replace(&mut *state, State::Written) {
        State::Recording(recorder) => Some(recorder),
        other => {
            *state = other;
            None
        }
    }
}

/// Writes the run's line, once per run. `interrupted` says that a signal
/// ended the run.
fn write(interrupted: bool) {
    let Some(recorder) = take(&RECORDER) else {
        return;
    };
    let state = etcetera::choose_base_strategy()
        .ok()
        .and_then(|strategy| strategy.state_dir());
    if let Some(state) = state {
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
}

/// Writes the line of `recorder` to `usage.jsonl` in the tool's directory
/// under the `state` directory, or returns `None` when any step fails.
fn save(
    state: &Path,
    recorder: Recorder,
    now: SystemTime,
    interrupted: bool,
    caller: &Caller,
    limits: &Limits,
) -> Option<()> {
    let dir = state.join(recorder.tool.as_str());
    file::create_dir(&dir).ok()?;
    let key = key::key(&dir)?;
    let line = record::line(&recorder.finish(&key, now, interrupted, caller))?;
    file::append(&dir, &line, limits).ok()
}

/// Starts the thread that writes the line when the run ends on SIGINT,
/// SIGTERM or SIGHUP, and returns once the thread listens. After the line,
/// the thread lets the signal take its default effect, so the process ends
/// as it would have without the log.
//
// RUST-10: the thread takes the three signals from the start of the run.
// A process that started with one of them ignored, as `nohup` ignores
// SIGHUP, now ends on it, and a tool that handles one of them itself ends
// on it too. Reading each signal's disposition before taking it replaces
// this, which needs the `sigaction` call that crumbkit's ban on unsafe code
// rules out today.
fn watch_signals() {
    let (listening, listens) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name("crumbkit-usage".to_owned())
        .spawn(move || {
            let signals = Signals::new([SIGINT, SIGTERM, SIGHUP]);
            let _ = listening.send(());
            let Ok(mut signals) = signals else {
                return;
            };
            if let Some(signal) = signals.forever().next() {
                write(true);
                let _ = emulate_default_handler(signal);
            }
        });
    if spawned.is_ok() {
        // The thread sends once it listens, and a thread that ended early
        // drops the sender, which ends the wait too.
        let _ = listens.recv();
    }
}

/// Returns `exit` as a number. crumbkit's `parse` and `finish` return only
/// 0, 1 and 2 (ADR-0002).
fn number(exit: ExitCode) -> u8 {
    if exit == ExitCode::SUCCESS {
        0
    } else if exit == ExitCode::from(2) {
        2
    } else {
        1
    }
}
