// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`start`]: crate::start()
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
//! [ADR-0095]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/usage/0095-text-hand-over-and-own-signals.md
//! [usage log docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/usage-log/README.md
//! [trace context docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/usage-log/trace-context.md
//! [usage report docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/usage-log/report.md
//! [run observer docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/run-observer/README.md
#![doc = include_str!("../README.md")]

mod caller;
mod context;
mod file;
mod key;
mod observer;
mod platform;
mod record;
mod recorder;
#[cfg(doctest)]
mod removed;
mod report;
mod self_figures;
mod self_usage;
mod shape;
#[cfg(test)]
mod testing;

pub use self::recorder::{Log, start};
pub use self::self_figures::{FiguresRow, SELF_FIGURES_LIST, SelfFigures, figures};
pub use self::self_usage::{FindingRow, SELF_USAGE, SelfUsage, UsageSummary, findings};

// The log's platform module has a Unix half and a Windows half, so the
// crate builds on those two (ADR-0003).
#[cfg(not(any(unix, windows)))]
compile_error!("crumbkit-usage builds only on Unix and Windows");
