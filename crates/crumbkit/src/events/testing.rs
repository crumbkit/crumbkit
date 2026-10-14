//! The observer of crumbkit's unit tests, which keeps each event as a line
//! of text in a list of the thread that reported it, so each test reads
//! only its own events under `cargo test` and under nextest.

use std::cell::RefCell;
use std::ffi::OsString;

use clap::{ArgMatches, Command};

use super::{Finished, OBSERVER, Observer, Refused, TraceIds};
use crate::{Code, Spelling};

thread_local! {
    /// The events that this thread's test watches, or `None` when it
    /// watches none.
    static SEEN: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
    /// What [`Observer::child_env`] returns on this thread.
    static CHILD_ENV: RefCell<Vec<(&'static str, String)>> = const { RefCell::new(Vec::new()) };
    /// What [`Observer::trace_ids`] returns on this thread.
    static TRACE_IDS: RefCell<Option<TraceIds>> = const { RefCell::new(None) };
}

/// The code that shows a spelling: a `typed` line ends with this code in
/// the run's spelling, such as `a::b::c`.
const SAMPLE: Code = Code::new("a", "b", "c");

/// The test observer. It handles no interrupt, and its other queries answer
/// what the thread's test set, by default no variables and no ids.
struct Recording;

/// The one test observer of the process.
static RECORDING: Recording = Recording;

/// Installs [`RECORDING`] as the observer of the test process, once.
pub(crate) fn install() {
    let _ = OBSERVER.get_or_init(|| &RECORDING);
}

/// Runs `run` and returns the events that it reported on this thread.
pub(crate) fn watch(run: impl FnOnce()) -> Vec<String> {
    install();
    SEEN.with_borrow_mut(|seen| *seen = Some(Vec::new()));
    run();
    SEEN.with_borrow_mut(Option::take).unwrap_or_default()
}

/// Makes the observer give `env` as the child variables on this thread.
pub(crate) fn give_child_env(env: Vec<(&'static str, String)>) {
    install();
    CHILD_ENV.set(env);
}

/// Makes the observer give `ids` as the trace ids on this thread.
#[cfg(feature = "tracing")]
pub(crate) fn give_trace_ids(ids: Option<TraceIds>) {
    install();
    TRACE_IDS.set(ids);
}

/// Adds `event` to this thread's events when its test watches.
fn see(event: String) {
    SEEN.with_borrow_mut(|seen| {
        if let Some(seen) = seen {
            seen.push(event);
        }
    });
}

/// Returns `words` as text, separated by spaces.
fn joined(words: &[impl AsRef<std::ffi::OsStr>]) -> String {
    words
        .iter()
        .map(|word| word.as_ref().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

impl Observer for Recording {
    fn typed(&self, command: &Command, args: &[OsString], spelling: Spelling) {
        see(format!(
            "typed {}: {} in {}",
            command.get_name(),
            joined(args),
            spelling.spell(&SAMPLE)
        ));
    }

    fn parsed(&self, command: &Command, matches: &ArgMatches) {
        let mut path = vec![command.get_name()];
        let mut level = matches;
        while let Some((name, next)) = level.subcommand() {
            path.push(name);
            level = next;
        }
        see(format!("parsed {}", path.join(" ")));
    }

    fn picked(&self, count: usize) {
        see(format!("picked {count}"));
    }

    fn helped(&self, command: &Command, path: &[String], long: bool, exit: u8) {
        let page = if long { "long" } else { "short" };
        see(format!(
            "helped {}: {} {page} {exit}",
            command.get_name(),
            joined(path)
        ));
    }

    fn refused(&self, refused: &Refused<'_>) {
        let nearest = refused
            .nearest_flag()
            .map_or_else(|| "none".to_owned(), |arg| arg.get_id().to_string());
        see(format!(
            "refused {}: {} {} nearest {nearest}",
            refused.command().get_name(),
            joined(refused.path()),
            refused.exit(),
        ));
    }

    fn exited(&self, exit: u8) {
        see(format!("exited {exit}"));
    }

    fn failed(&self, spelling: Spelling, code: &Code) {
        see(format!("failed {}", spelling.spell(code)));
    }

    fn finished(&self, finished: &Finished) {
        see(format!(
            "finished {} {} bytes truncated {} empty {}",
            finished.exit(),
            finished.out_bytes(),
            finished.truncated(),
            finished.empty()
        ));
    }

    fn ended(&self, exit: u8, out_bytes: Option<u64>) {
        see(format!("ended {exit} {out_bytes:?}"));
    }

    fn aliased(&self) {
        see("aliased".to_owned());
    }

    fn tolerated(&self, code: &'static Code) {
        see(format!("tolerated {}", Spelling::DOTS.spell(code)));
    }

    fn handed_over(&self) {
        see("handed over".to_owned());
    }

    fn taken_back(&self) {
        see("taken back".to_owned());
    }

    fn interrupted(&self) {
        see("interrupted".to_owned());
    }

    fn child_env(&self) -> Vec<(&'static str, String)> {
        CHILD_ENV.with_borrow(Clone::clone)
    }

    fn trace_ids(&self) -> Option<TraceIds> {
        TRACE_IDS.with_borrow(Clone::clone)
    }
}
