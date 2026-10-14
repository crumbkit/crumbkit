//! The usage log as the process's observer: each report that crumbkit
//! sends through `crate::events` updates the process's recorder (ADR-0057).

use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::{ArgMatches, Command};

use super::{recorder, shape};
use crate::events::{Finished, Observer, Refused, TraceIds};
use crate::{Code, Spelling};

/// Whether crumbkit reported that a child took the terminal, and on
/// Windows has not reported that the child gave it back.
static HELD: AtomicBool = AtomicBool::new(false);

/// The usage log's observer, which `start` installs.
pub(super) struct UsageLog;

/// Reports whether a child holds the terminal, as [`HELD`] says, which the
/// signal thread asks before it lets SIGINT end the process.
pub(super) fn held_terminal() -> bool {
    HELD.load(Ordering::SeqCst)
}

impl Observer for UsageLog {
    fn typed(&self, command: &Command, args: &[OsString], spelling: Spelling) {
        recorder::report(|recorder| recorder.typed(command, args, spelling));
    }

    fn parsed(&self, command: &Command, matches: &ArgMatches) {
        recorder::report(|recorder| shape::parsed(&mut recorder.run, command, matches));
    }

    fn picked(&self, count: usize) {
        recorder::report(|recorder| recorder.run.picked = Some(count));
    }

    fn helped(&self, command: &Command, path: &[String], long: bool, exit: u8) {
        recorder::report(|recorder| recorder.helped(command, path, long, exit));
    }

    fn refused(&self, refused: &Refused<'_>) {
        recorder::report(|recorder| {
            shape::refused(&mut recorder.run, refused, &recorder.words);
            recorder.run.exit = Some(refused.exit());
        });
    }

    fn exited(&self, exit: u8) {
        recorder::report(|recorder| recorder.run.exit = Some(exit));
    }

    fn failed(&self, spelling: Spelling, code: &Code) {
        recorder::report(|recorder| recorder.failed(spelling, code));
    }

    fn finished(&self, finished: &Finished) {
        recorder::report(|recorder| {
            recorder.run.exit = Some(finished.exit());
            recorder.run.out_bytes = Some(finished.out_bytes());
            recorder.run.truncated = finished.truncated();
            recorder.run.empty = finished.empty();
        });
    }

    fn ended(&self, exit: u8, out_bytes: Option<u64>) {
        recorder::ended(exit, out_bytes);
    }

    fn aliased(&self) {
        recorder::report(|recorder| recorder.run.alias = true);
    }

    fn tolerated(&self, code: &'static Code) {
        recorder::report(|recorder| recorder.tolerated(code));
    }

    fn handed_over(&self) {
        HELD.store(true, Ordering::SeqCst);
    }

    fn taken_back(&self) {
        HELD.store(false, Ordering::SeqCst);
    }

    fn interrupted(&self) {
        recorder::report(|recorder| recorder.signalled = true);
    }

    fn handles_interrupt(&self) -> bool {
        recorder::listens()
    }

    fn child_env(&self) -> Vec<(&'static str, String)> {
        recorder::child_env()
    }

    fn trace_ids(&self) -> Option<TraceIds> {
        recorder::ids()
    }
}
