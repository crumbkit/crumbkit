//! The usage log: one JSON line per run in a file on the user's machine,
//! which records the shape of the run and never its values (ADR-0029).
//!
//! The log exists only with the `usage-log` feature, which is off by
//! default. A tool's `main` calls `start` first and keeps the guard it
//! returns, as the example of `start` shows.
//! [`parse`](crate::parse), [`finish`](crate::finish) and
//! crumbkit-aliases' `expand` then report what the run did, and the guard
//! writes the line when `main` returns. `parse` gives the line the tool's
//! name and version, so a run that ends before it reaches `parse` writes no
//! line. Without the feature, or without a started log, every report does
//! nothing.

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
mod random;
#[cfg(feature = "usage-log")]
mod record;
#[cfg(feature = "usage-log")]
mod recorder;
#[cfg(feature = "usage-log")]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "only the tests read the report until the `self usage` command exists"
    )
)]
mod report;
#[cfg(feature = "usage-log")]
mod shape;
#[cfg(all(feature = "usage-log", test))]
mod testing;

#[cfg(feature = "usage-log")]
pub(crate) use self::counted::Counted;
#[cfg(all(feature = "usage-log", feature = "tracing"))]
pub(crate) use self::recorder::ids;
#[cfg(feature = "usage-log")]
pub use self::recorder::{Log, section, start};
#[cfg(feature = "usage-log")]
pub(crate) use self::recorder::{
    child_env, exited, failed, finished, helped, parsed, refused, typed,
};

// RUST-10: the log reads its key from `/dev/urandom`, finds the session in
// the parent process id, and sets Unix file modes, so the feature builds
// only on Unix. Windows equivalents of the three replace this when a tool
// needs the log there.
#[cfg(all(feature = "usage-log", not(unix)))]
compile_error!("crumbkit's `usage-log` feature builds only on Unix");

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
