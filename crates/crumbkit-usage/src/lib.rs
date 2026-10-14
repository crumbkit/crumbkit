//! The usage log: one JSON line per run in a file on the user's machine,
//! which records the shape of the run and never its values (ADR-0029).
//!
//! The log exists only with the `usage-log` feature, which is off by
//! default. A tool's `main` calls `start` first and keeps the guard it
//! returns, as the example of `start` shows. `start` installs the log as the
//! process's observer (ADR-0057), so [`parse`](crate::parse),
//! [`finish`](crate::finish), a child hand-over and crumbkit-aliases'
//! `expand` report what the run did to it through [`crate::events`], and
//! the guard writes the line when `main` returns. A run that never reaches
//! `finish` reports its exit with [`crate::events::ended`], which writes the
//! line at once. `parse` gives the line the tool's name and version, so a
//! run that ends before it reaches `parse` writes no line. Without a
//! started log, every report does nothing.

mod caller;
mod context;
mod file;
mod key;
mod observer;
mod platform;
mod record;
mod recorder;
mod report;
mod self_figures;
mod self_usage;
mod shape;
#[cfg(test)]
mod testing;

pub use self::recorder::{Log, Signals, section, start, start_with};
pub use self::self_figures::{FiguresRow, SELF_FIGURES_LIST, SelfFigures, figures};
pub use self::self_usage::{FindingRow, SELF_USAGE, SelfUsage, UsageSummary, findings};

// The log's platform module has a Unix half and a Windows half, so the
// feature builds on those two (ADR-0003).
#[cfg(not(any(unix, windows)))]
compile_error!("crumbkit's `usage-log` feature builds only on Unix and Windows");
