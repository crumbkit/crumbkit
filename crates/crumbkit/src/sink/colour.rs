//! What decides whether crumbkit writes styles to a standard stream: the
//! colour variables of the environment, whether the stream is a terminal,
//! the command's `ColorChoice` and the guide's terminal detection
//! (ADR-0131, ADR-0133, ADR-0150).

use std::ffi::{OsStr, OsString};
use std::io::{Stderr, StdoutLock};

use clap::ColorChoice;

use super::{Sink, stderr_is_terminal, stdout_is_terminal};

impl Sink<StdoutLock<'static>, Stderr> {
    /// Returns the standard sink that [`parse`](crate::parse) and
    /// [`parse_from`](crate::parse_from) print a refusal to. Its error
    /// writer wants clap's styles as [`wants_styles`] decides from
    /// `NO_COLOR`, `CLICOLOR_FORCE`, `CLICOLOR`, `TERM`, whether standard
    /// error is a terminal, the command's `choice` and the guide's terminal
    /// `detection` (ADR-0131, ADR-0133). Without crumbkit's `color` feature
    /// it wants none.
    pub(crate) fn standard_for_refusal(choice: ColorChoice, detection: bool) -> Self {
        let styled = cfg!(feature = "color")
            && wants_styles(variables(), stderr_is_terminal(), choice, detection);
        Self::standard().styled_errors(styled)
    }
}

/// Reports whether the result on standard output carries the styles of a
/// text table's cells, which [`finish`](crate::finish) asks its sink for:
/// [`wants_styles`] decides from the colour variables, whether standard
/// output is a terminal, the command's `choice` and the guide's terminal
/// `detection` (ADR-0150). A run without detection reads no variable and
/// asks about no stream.
pub(crate) fn stdout_styles(choice: ColorChoice, detection: bool) -> bool {
    detection && wants_styles(variables(), stdout_is_terminal(), choice, detection)
}

/// Returns what the colour variables of the environment say, each as its
/// reader reports it.
fn variables() -> ColorVariables {
    ColorVariables {
        no_color: color_off(colour_variable("NO_COLOR").as_deref()),
        clicolor_force: color_forced(colour_variable("CLICOLOR_FORCE").as_deref()),
        clicolor_off: clicolor_off(colour_variable("CLICOLOR").as_deref()),
        term_dumb: term_dumb(colour_variable("TERM").as_deref()),
    }
}

/// Returns the value of the environment variable `name`, one of the
/// variables that decide whether a refusal on standard error or a table's
/// cells on standard output are styled.
fn colour_variable(name: &str) -> Option<OsString> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the sink owns the standard streams, and the colour variables decide whether a refusal or a table's cells on them are styled"
    )]
    std::env::var_os(name)
}

/// What the colour variables of the environment say about styles on a
/// standard stream, each as its reader reports it.
#[derive(Clone, Copy, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is one independent environment variable, which tests set directly"
)]
pub(crate) struct ColorVariables {
    /// `NO_COLOR` turns colour off, as [`color_off`] reports.
    pub(crate) no_color: bool,
    /// `CLICOLOR_FORCE` forces colour, as [`color_forced`] reports.
    pub(crate) clicolor_force: bool,
    /// `CLICOLOR` turns colour off, as [`clicolor_off`] reports.
    pub(crate) clicolor_off: bool,
    /// `TERM` names a terminal without colour, as [`term_dumb`] reports.
    pub(crate) term_dumb: bool,
}

/// Reports whether a stream carries styles: a refusal and its group page on
/// standard error (ADR-0131, ADR-0133), and a text table's cell styles in
/// the result on standard output (ADR-0150). The rule is the same for both:
///
/// - never when terminal `detection` is off or the command's `choice` is
///   `ColorChoice::Never`, as help is never styled then (ADR-0102);
/// - never when `NO_COLOR` turns colour off, even with `CLICOLOR_FORCE`;
/// - always when `CLICOLOR_FORCE` forces colour, whatever standard error,
///   `TERM` and `CLICOLOR` are, because anstream checks `CLICOLOR_FORCE`
///   before `TERM` and `CLICOLOR` when it colours help;
/// - never when `TERM` is `dumb` or `CLICOLOR` is `0`;
/// - otherwise only when the stream is a `terminal`, so a pipe or a file
///   receives plain text.
///
/// `ColorChoice::Always` follows the same rule as `ColorChoice::Auto`.
//
// RUST-10: this reads the four variables itself and does not call
// anstream's choice for the stream, which clap applies to help. The
// rule keeps anstream's order but differs from it on purpose: an unset
// `TERM`, a `CLICOLOR` other than `0` and the `CI` variable change nothing,
// `CLICOLOR_FORCE=0` forces no colour, `NO_COLOR` beats
// `ColorChoice::Always`, and `CLICOLOR_FORCE` styles a refusal at a pipe
// (ADR-0131, ADR-0133). A call to anstream's choice replaces this code if
// a refusal is ever meant to follow every part of help's rule.
pub(crate) const fn wants_styles(
    variables: ColorVariables,
    terminal: bool,
    choice: ColorChoice,
    detection: bool,
) -> bool {
    if !detection || matches!(choice, ColorChoice::Never) || variables.no_color {
        return false;
    }
    if variables.clicolor_force {
        return true;
    }
    !variables.term_dumb && !variables.clicolor_off && terminal
}

/// Reports whether `NO_COLOR` with `value` turns colour off, which any
/// value that is not empty does.
pub(crate) fn color_off(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| !value.is_empty())
}

/// Reports whether `CLICOLOR_FORCE` with `value` forces colour, which any
/// value that is neither empty nor `0` does.
pub(crate) fn color_forced(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| !value.is_empty() && value != "0")
}

/// Reports whether `CLICOLOR` with `value` turns colour off, which only the
/// value `0` does, as anstream reads it.
pub(crate) fn clicolor_off(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| value == "0")
}

/// Reports whether `TERM` with `value` names a terminal without colour,
/// which only the value `dumb` does. An unset `TERM` names none.
pub(crate) fn term_dumb(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| value == "dumb")
}
