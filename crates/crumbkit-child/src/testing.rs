//! The observer of the crate's unit tests, which keeps each hand-over
//! report as a line of text in a list of the thread that reported it, so
//! each test reads only its own reports. Only the tests that run a script
//! watch the reports, so the observer builds on Unix alone.

#[cfg(unix)]
use std::cell::RefCell;

#[cfg(unix)]
use crumbkit::events::{Observer, observe};

#[cfg(unix)]
thread_local! {
    /// The reports that this thread's test watches, or `None` when it
    /// watches none.
    static SEEN: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// The test observer. It handles no interrupt.
#[cfg(unix)]
struct Recording;

/// The one test observer of the process.
#[cfg(unix)]
static RECORDING: Recording = Recording;

#[cfg(unix)]
impl Observer for Recording {
    fn ended(&self, exit: u8, out_bytes: Option<u64>) {
        see(format!("ended {exit} {out_bytes:?}"));
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
}

/// Adds `line` to the reports of this thread's test, if it watches them.
#[cfg(unix)]
fn see(line: String) {
    SEEN.with_borrow_mut(|seen| {
        if let Some(seen) = seen {
            seen.push(line);
        }
    });
}

/// Runs `run` and returns the reports that it made on this thread. The
/// first call installs [`RECORDING`] as the observer of the test process.
#[cfg(unix)]
pub(crate) fn watch(run: impl FnOnce()) -> Vec<String> {
    let _ = observe(&RECORDING);
    SEEN.set(Some(Vec::new()));
    run();
    SEEN.take().unwrap_or_default()
}

/// Checks that `refused` and `admitted` differ by one word added, removed
/// or replaced, as the twin of a refused command line must (TEST-1).
#[track_caller]
pub(crate) fn assert_one_word_apart(refused: &[&str], admitted: &[&str]) {
    let (long, short) = if refused.len() >= admitted.len() {
        (refused, admitted)
    } else {
        (admitted, refused)
    };
    let first = long
        .iter()
        .zip(short)
        .position(|(a, b)| a != b)
        .unwrap_or(short.len());
    let apart = match long.len() - short.len() {
        0 => first < long.len() && long[first + 1..] == short[first + 1..],
        1 => long[first + 1..] == short[first..],
        _ => false,
    };
    assert!(
        apart,
        "the refused words {refused:?} and the admitted words {admitted:?} must differ by one word added, removed or replaced"
    );
}
