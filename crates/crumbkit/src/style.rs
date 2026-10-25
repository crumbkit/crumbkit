//! The semantic styles that a row type gives the cells of a text table,
//! which show only when the sink's result wants styles (ADR-0150).

use clap::builder::styling::{AnsiColor, Style as Ansi};

#[cfg(test)]
mod tests;

/// The style of one cell of a text table, which a row type gives a cell
/// with [`Row::cell_style`](crate::Row::cell_style).
///
/// A style names what the cell means, and crumbkit chooses how a terminal
/// shows it. A cell carries its style only in a text table whose sink wants
/// styles for the result, which [`Sink::styled`](crate::Sink::styled) sets
/// and [`finish`](crate::finish) sets when standard output is a terminal
/// that allows colour. JSON, a pipe and a sink that wants no styles print
/// the cell's text unchanged, and the styled table without its escape codes
/// is that same text (ADR-0150).
///
/// ```
/// use crumbkit::{Row, Style};
/// use schemars::JsonSchema;
/// use serde::Serialize;
///
/// /// A service and its state.
/// #[derive(Serialize, JsonSchema)]
/// struct Service {
///     /// The service's name.
///     name: String,
///     /// The service's state: `running`, `failed` or `stopped`.
///     state: String,
/// }
///
/// impl Row for Service {
///     const COLUMNS: &'static [&'static str] = &["name", "state"];
///
///     fn cell_style(&self, column: &str) -> Option<Style> {
///         match (column, self.state.as_str()) {
///             ("state", "running") => Some(Style::Success),
///             ("state", "failed") => Some(Style::Error),
///             ("state", "stopped") => Some(Style::Dim),
///             _ => None,
///         }
///     }
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Style {
    /// A cell that a person should notice first, such as the name of the
    /// thing that a row describes. A terminal shows it in bold.
    Emphasis,
    /// A cell of less interest, such as the state of a stopped service. A
    /// terminal shows it dim.
    Dim,
    /// A cell that reports that something works or finished, such as
    /// `running`. A terminal shows it in green.
    Success,
    /// A cell that reports a failure or a problem, such as `failed`. A
    /// terminal shows it in bold red, the style of the `error:` label that
    /// starts a refusal (ADR-0154).
    Error,
}

impl Style {
    /// Returns the ANSI style that a terminal shows this style with.
    pub(crate) const fn ansi(self) -> Ansi {
        match self {
            Self::Emphasis => Ansi::new().bold(),
            Self::Dim => Ansi::new().dimmed(),
            Self::Success => AnsiColor::Green.on_default(),
            Self::Error => AnsiColor::Red.on_default().bold(),
        }
    }
}
