// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`start`]: crate::start()
//! [`Log`]: crate::Log
//! [`Facts`]: crate::Facts
//! [`Key`]: crate::Key
//! [`Label`]: crate::Label
//! [`SELF_USAGE`]: crate::SELF_USAGE
//! [`SELF_FIGURES_LIST`]: crate::SELF_FIGURES_LIST
//! [`FindingRow`]: crate::FindingRow
//! [`FiguresRow`]: crate::FiguresRow
//! [`UsageSummary`]: crate::UsageSummary
//! [`Extension`]: crate::Extension
//! [`extend`]: crate::extend()
//! [`SelfUsage`]: crate::SelfUsage
//! [`findings`]: crate::findings()
//! [`SelfFigures`]: crate::SelfFigures
//! [`figures`]: crate::figures()
//! [`cli.rs`]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/src/cli.rs
//! [`lib.rs`]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/src/lib.rs
//! [schemas]: https://github.com/crumbkit/crumbkit/tree/main/crates/crumbkit-usage/schemas
//! [ADR-0029]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/usage/0029-usage-log.md
//! [ADR-0030]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/usage/0030-trace-context.md
//! [ADR-0034]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/usage/0034-usage-report.md
//! [ADR-0095]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/usage/0095-text-hand-over-own-signals-and-menu.md
//! [ADR-0144]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/usage/0144-a-tool-adds-value-free-facts-to-the-usage-record.md
//! [ADR-0145]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/usage/0145-self-usage-finds-slow-commands-that-mostly-wait.md
//! [usage log docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/usage-log/README.md
//! [trace context docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/usage-log/trace-context.md
//! [usage report docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/usage-log/report.md
//! [run observer docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/run-observer/README.md
#![doc = include_str!("../README.md")]

mod caller;
mod context;
mod facts;
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

pub use self::facts::{Extension, Facts, Key, Label, extend};
// `log_path` exists for cargo-crumbkit's `weigh --usage`, which reads the
// log where this crate writes it. No tool calls it, so it stays out of the
// documentation.
#[doc(hidden)]
pub use self::file::log_path;
pub use self::recorder::{Log, start};
pub use self::self_figures::{FiguresRow, SELF_FIGURES_LIST, SelfFigures, figures};
pub use self::self_usage::{FindingRow, SELF_USAGE, SelfUsage, UsageSummary, findings};

// The log's platform module has a Unix half and a Windows half, so the
// crate builds on those two (ADR-0003).
#[cfg(not(any(unix, windows)))]
compile_error!("crumbkit-usage builds only on Unix and Windows");
