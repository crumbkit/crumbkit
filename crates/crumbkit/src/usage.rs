//! The usage log: one JSON line per run in a file on the user's machine,
//! which records the shape of the run and never its values (ADR-0029).
//!
//! The log exists only with the `usage-log` feature, which is off by
//! default. A tool's `main` calls `start` first and keeps the guard it
//! returns, as the example of `start` shows.
//! [`parse`](crate::parse), [`finish`](crate::finish) and
//! crumbkit-aliases' `expand` then report what the run did, and the guard
//! writes the line when `main` returns. A run that never reaches `finish`
//! reports its exit with [`ended`], which writes the line at once. `parse`
//! gives the line the tool's name and version, so a run that ends before it
//! reaches `parse` writes no line. Without the feature, or without a started
//! log, every report does nothing.

use std::process::ExitCode;

#[cfg(feature = "usage-log")]
mod caller;
#[cfg(feature = "usage-log")]
mod context;
#[cfg(feature = "usage-log")]
mod counted;
#[cfg(feature = "usage-log")]
mod file;
#[cfg(feature = "usage-log")]
mod key;
#[cfg(feature = "usage-log")]
mod platform;
#[cfg(feature = "usage-log")]
mod record;
#[cfg(feature = "usage-log")]
mod recorder;
#[cfg(feature = "usage-log")]
mod report;
#[cfg(feature = "usage-log")]
mod self_figures;
#[cfg(feature = "usage-log")]
mod self_usage;
#[cfg(feature = "usage-log")]
mod shape;
// The two attributes stay apart, because clippy reads only a plain
// `cfg(test)` as test code, where its helpers may unwrap.
#[cfg(feature = "usage-log")]
#[cfg(test)]
mod testing;

#[cfg(feature = "usage-log")]
pub(crate) use self::counted::Counted;
#[cfg(all(feature = "usage-log", feature = "tracing"))]
pub(crate) use self::recorder::ids;
#[cfg(feature = "usage-log")]
pub use self::recorder::{Log, Signals, section, start, start_with};
#[cfg(feature = "usage-log")]
pub(crate) use self::recorder::{
    child_env, exited, failed, finished, helped, parsed, picked, refused, typed,
};
#[cfg(feature = "usage-log")]
pub use self::self_figures::{FiguresRow, SELF_FIGURES_LIST, SelfFigures, figures};
#[cfg(feature = "usage-log")]
pub use self::self_usage::{FindingRow, SELF_USAGE, SelfUsage, UsageSummary, findings};

// The log's platform module has a Unix half and a Windows half, so the
// feature builds on those two (ADR-0003).
#[cfg(all(feature = "usage-log", not(any(unix, windows))))]
compile_error!("crumbkit's `usage-log` feature builds only on Unix and Windows");

/// Records that a personal alias expanded the run's command line.
/// crumbkit-aliases' `expand` calls it, so the run's line holds
/// `"alias": true`.
///
/// It exists in every build, so a crate that calls it needs no feature of
/// its own. Without the `usage-log` feature, or without a started log, it
/// does nothing.
pub fn aliased() {
    #[cfg(feature = "usage-log")]
    recorder::aliased();
}

/// Records that the run met the failure `code` and recovered from it, such
/// as a lock that the tool waited out or a download that it retried, so
/// the run's line lists the code under `tolerated`.
///
/// ```
/// use crumbkit::Code;
///
/// static LOCKED: Code = Code::new("shelf", "catalogue", "locked");
///
/// // The tool waited for the lock and then read the catalogue.
/// crumbkit::usage::tolerated(&LOCKED);
/// ```
///
/// The line spells each code as the tool's failures spell it, with the
/// `Spelling` that the tool passed to [`parse`](crate::parse), and holds
/// each code once, in the order the tool reported them, up to eight codes.
/// `self usage` counts the runs that tolerated each code. A code that the
/// run then fails with belongs to [`finish`](crate::finish), which records
/// it as the run's failure.
///
/// It exists in every build, so a tool calls it without a feature check.
/// Without the `usage-log` feature, or without a started log, it does
/// nothing.
pub fn tolerated(code: &'static crate::Code) {
    #[cfg(feature = "usage-log")]
    recorder::tolerated(code);
    #[cfg(not(feature = "usage-log"))]
    let _ = code;
}

/// Records that the tool ended the run with `exit` after writing
/// `out_bytes` to standard output, writes the run's line now, and returns
/// `exit` as an `ExitCode` for `main` to return.
///
/// A run that never reaches [`finish`](crate::finish), such as a stream, a
/// child process or the tool's own exit path, calls it last, so its line
/// records the real exit number, such as 124, and the byte count. The line
/// leaves `out_bytes` out when it is `None`. Because it returns the code, the
/// number that the tool returns and the number that the line records are the
/// same:
///
/// ```no_run
/// # use std::process::ExitCode;
/// fn main() -> ExitCode {
///     // The tool ran out of time after it streamed 2048 bytes.
///     crumbkit::usage::ended(124, Some(2048))
/// }
/// ```
///
/// crumbkit owns the exit codes 0, 1 and 2, and the tool owns the rest
/// (ADR-0002). The line records the number as given.
///
/// The line is written at once, so a tool that then exits through its own
/// path, such as `std::process::exit` on another thread, still has its line,
/// and the guard that `start` returns writes nothing more. A report after `ended`
/// does nothing. It exists in every build, so a tool calls it without a
/// feature check. Without the `usage-log` feature, or without a started log,
/// it only returns the code.
#[must_use]
pub fn ended(exit: u8, out_bytes: Option<u64>) -> ExitCode {
    #[cfg(feature = "usage-log")]
    recorder::ended(exit, out_bytes);
    #[cfg(not(feature = "usage-log"))]
    let _ = out_bytes;
    ExitCode::from(exit)
}

/// Records that a signal ended the run's child process, so the run's line
/// holds `"interrupted": true`. Without the `usage-log` feature, or without
/// a started log, it does nothing.
#[cfg(feature = "hand-over")]
pub(crate) fn interrupted() {
    #[cfg(feature = "usage-log")]
    recorder::interrupted();
}

/// Reports whether the usage log's signal thread has taken SIGINT, as a log
/// started with either `Signals` value has. Without the `usage-log` feature
/// it is false.
#[cfg(feature = "hand-over")]
pub(crate) fn listens() -> bool {
    #[cfg(feature = "usage-log")]
    {
        recorder::listens()
    }
    #[cfg(not(feature = "usage-log"))]
    {
        false
    }
}
