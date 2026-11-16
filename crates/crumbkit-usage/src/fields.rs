//! The two core fields that every tool can report beside its own facts:
//! the time a run waited for locks and the place it took place in.

use std::time::Duration;

use super::facts::Label;
use super::recorder::report;

/// Adds `time` to the time that this run spent waiting to take locks, which
/// the usage log writes as `lock_ms`, and does nothing when no log was
/// started.
///
/// A tool passes the time that each lock reports, such as
/// `crumbkit_durable::PathLock::waited`, once for each lock it took.
/// Several calls add, so a call passes a stretch of time that no other call
/// covers. A time under 1 ms adds nothing, and a sum that would pass
/// `u64::MAX` milliseconds stays there. `self usage` counts the time
/// towards a run that mostly waits (ADR-0145).
pub fn lock_waited(time: Duration) {
    report(|recorder| recorder.run.add_lock_wait(time));
}

/// Records where this run took place, one [`Label`] from the tool's own
/// constants, such as `project` for a run inside a project and `elsewhere`
/// for a run outside one. The usage log writes it as `context`, and
/// `self runs list` and `self figures list` can keep only the runs with a
/// context. The last call of a run stays, and a call does nothing when no
/// log was started.
pub fn context(label: Label) {
    report(|recorder| recorder.run.context = Some(label));
}
