//! Failure codes known only at run time, such as the code that a plugin
//! answers with.

use std::fmt;
use std::sync::{Mutex, PoisonError};

use super::Code;
use crate::segment::is_code_segment;

#[cfg(test)]
mod tests;

/// The most distinct run-time codes that one process holds. A plugin that
/// put an id in its reason would otherwise grow the process's memory with
/// every failure.
const MOST_CODES: usize = 256;

/// The run-time codes of this process, each made once and kept until the
/// process exits.
static CODES: Mutex<Table> = Mutex::new(Table::new());

impl Code {
    /// Returns the code with the segments `component`, `area` and `reason`,
    /// which the tool learns only at run time, such as from a plugin's
    /// answer.
    ///
    /// It applies the rule of [`Code::new`] to each segment and returns
    /// [`RuntimeCodeError::Segment`] for the first one that breaks it. The
    /// returned code is `'static`, so it fits
    /// [`Failure::code`](crate::Failure::code) as a constant does.
    ///
    /// The process keeps each distinct run-time code until it exits, and
    /// returns the same code for the same segments. It holds at most 256 of
    /// them. Once it holds 256, a call with segments it does not hold returns
    /// [`RuntimeCodeError::Full`], and a call with segments it holds still
    /// returns their code. A tool that receives `Full` reports the failure
    /// under a code of its own.
    ///
    /// crumbkit-test's codes check cannot list run-time codes, because the
    /// tool does not know them when its tests run. A failure with a run-time
    /// code supplies its own recovery, which is `Recovery::None` with a
    /// reason when the plugin gives no next step.
    ///
    /// ```
    /// use crumbkit::{Code, Spelling};
    ///
    /// const PLUGIN_FAILED: Code = Code::new("shelf", "plugin", "failed");
    ///
    /// let answer = "lending::loans::overdue";
    /// let code = Spelling::COLONS
    ///     .split(answer)
    ///     .and_then(|(component, area, reason)| Code::runtime(component, area, reason).ok())
    ///     .unwrap_or(&PLUGIN_FAILED);
    ///
    /// assert_eq!(Spelling::COLONS.spell(code), "lending::loans::overdue");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeCodeError::Segment`] when a segment breaks the rule
    /// of [`Code::new`], and
    /// [`RuntimeCodeError::Full`] when the process already holds 256 other
    /// run-time codes.
    pub fn runtime(
        component: &str,
        area: &str,
        reason: &str,
    ) -> Result<&'static Self, RuntimeCodeError> {
        // `Table::code` pushes only a finished code, so the table is whole
        // even after a thread panicked while it held the lock, and a
        // poisoned lock is used as it is.
        let mut table = CODES.lock().unwrap_or_else(PoisonError::into_inner);
        table.code(component, area, reason, MOST_CODES)
    }
}

/// The run-time codes that one process holds.
pub(super) struct Table {
    /// The codes, in the order they were first asked for.
    codes: Vec<&'static Code>,
}

impl Table {
    /// Returns an empty table.
    pub(super) const fn new() -> Self {
        Self { codes: Vec::new() }
    }

    /// Returns the code with these segments: the one the table holds, or a
    /// new one that it keeps, when the segments follow the rule and the
    /// table holds fewer than `most` codes.
    pub(super) fn code(
        &mut self,
        component: &str,
        area: &str,
        reason: &str,
        most: usize,
    ) -> Result<&'static Code, RuntimeCodeError> {
        for (segment, text) in [("component", component), ("area", area), ("reason", reason)] {
            if !is_code_segment(text) {
                return Err(RuntimeCodeError::Segment {
                    segment,
                    text: text.to_owned(),
                });
            }
        }
        let held = self
            .codes
            .iter()
            .find(|code| code.component == component && code.area == area && code.reason == reason);
        if let Some(code) = held {
            return Ok(code);
        }
        if self.codes.len() >= most {
            return Err(RuntimeCodeError::Full);
        }
        // Leaking gives the `'static` lifetime that `Failure::code`
        // requires. The table bounds the leak at `most` codes per process.
        let code: &'static Code = Box::leak(Box::new(Code {
            component: leak(component),
            area: leak(area),
            reason: leak(reason),
        }));
        self.codes.push(code);
        Ok(code)
    }
}

/// Returns a copy of `text` that lives until the process exits.
fn leak(text: &str) -> &'static str {
    Box::leak(text.into())
}

/// Why [`Code::runtime`] returned no code.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeCodeError {
    /// A segment breaks the rule of [`Code::new`]: it is not words of
    /// lowercase ASCII letters, digits and underscores joined by single
    /// hyphens.
    Segment {
        /// Which segment broke the rule: `component`, `area` or `reason`.
        segment: &'static str,
        /// The segment's text.
        text: String,
    },
    /// The process already holds 256 run-time codes, and these segments are
    /// not one of them.
    Full,
}

impl fmt::Display for RuntimeCodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Segment { segment, text } => write!(
                f,
                "the code {segment} {text:?} is not words of lowercase ASCII letters, digits and underscores joined by single hyphens"
            ),
            Self::Full => write!(
                f,
                "the process already holds {MOST_CODES} codes known only at run time, the most it can hold"
            ),
        }
    }
}

impl std::error::Error for RuntimeCodeError {}
