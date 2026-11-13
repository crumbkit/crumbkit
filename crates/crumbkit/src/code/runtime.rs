//! Failure codes known only at run time, such as the code that a plugin
//! answers with, and the owned code that a failure keeps them in.

use std::fmt;

use super::Code;
use crate::segment::is_code_segment;

#[cfg(test)]
mod tests;

impl<'a> Code<'a> {
    /// Returns the code with the segments `component`, `area` and `reason`,
    /// which the tool learns only at run time, such as from a plugin's
    /// answer.
    ///
    /// It applies the rule of [`Code::new`] to each segment and returns
    /// [`RuntimeCodeError::Segment`] for the first one that breaks it. The
    /// returned code borrows the three segments, and crumbkit keeps no copy
    /// of it, so a tool makes any number of run-time codes. A failure that
    /// keeps the code holds the [`CodeBuf`] that [`Code::to_buf`] returns,
    /// and answers [`Failure::code`](crate::Failure::code) with
    /// [`CodeBuf::as_code`].
    ///
    /// A `CODES` list cannot name run-time codes, because the tool does not
    /// know them when its tests run. A failure with a run-time code supplies
    /// its own recovery, which is `Recovery::None` with a reason when the plugin
    /// gives no next step.
    ///
    /// ```
    /// use crumbkit::{Code, Spelling};
    ///
    /// const PLUGIN_FAILED: Code<'static> = Code::new("shelf", "plugin", "failed");
    ///
    /// let answer = "lending::loans::overdue";
    /// let code = Spelling::COLONS
    ///     .split(answer)
    ///     .and_then(|(component, area, reason)| Code::runtime(component, area, reason).ok())
    ///     .unwrap_or(PLUGIN_FAILED);
    ///
    /// assert_eq!(Spelling::COLONS.spell(code), "lending::loans::overdue");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeCodeError::Segment`] when a segment breaks the rule
    /// of [`Code::new`].
    pub fn runtime(
        component: &'a str,
        area: &'a str,
        reason: &'a str,
    ) -> Result<Self, RuntimeCodeError> {
        for (segment, text) in [
            (Segment::Component, component),
            (Segment::Area, area),
            (Segment::Reason, reason),
        ] {
            if !is_code_segment(text) {
                return Err(RuntimeCodeError::Segment {
                    segment,
                    text: text.to_owned(),
                });
            }
        }
        Ok(Self {
            component,
            area,
            reason,
        })
    }

    /// Returns a copy of the code that owns its segments.
    ///
    /// A value that keeps a code after the text the code borrows is gone,
    /// such as a failure made from a plugin's answer or an observer that
    /// holds the codes of a run, stores the copy and reads it with
    /// [`CodeBuf::as_code`].
    ///
    /// ```
    /// use crumbkit::{Code, CodeBuf, Spelling};
    ///
    /// let owned: CodeBuf = {
    ///     let answer = String::from("lending::loans::overdue");
    ///     let (component, area, reason) = Spelling::COLONS.split(&answer).unwrap();
    ///     Code::runtime(component, area, reason).unwrap().to_buf()
    /// };
    ///
    /// assert_eq!(Spelling::COLONS.spell(owned.as_code()), "lending::loans::overdue");
    /// ```
    #[must_use]
    pub fn to_buf(self) -> CodeBuf {
        CodeBuf {
            component: self.component.into(),
            area: self.area.into(),
            reason: self.reason.into(),
        }
    }
}

/// A failure [`Code`] that owns its three segments.
///
/// A failure that learns its code at run time stores a `CodeBuf`, which
/// [`Code::to_buf`] returns, and answers
/// [`Failure::code`](crate::Failure::code) with [`CodeBuf::as_code`]. Two
/// `CodeBuf`s are equal when their segments are equal.
///
/// ```
/// use std::fmt;
///
/// use crumbkit::{Code, CodeBuf, Failure, Recovery};
///
/// /// A failure with the code that a plugin answered with.
/// #[derive(Debug)]
/// struct PluginFailure {
///     code: CodeBuf,
/// }
///
/// impl fmt::Display for PluginFailure {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         f.write_str("the plugin failed")
///     }
/// }
///
/// impl std::error::Error for PluginFailure {}
///
/// impl Failure for PluginFailure {
///     fn code(&self) -> Code<'_> {
///         self.code.as_code()
///     }
///
///     fn recovery(&self) -> Recovery {
///         Recovery::none("the plugin gave no next step")
///     }
/// }
///
/// let failure = PluginFailure {
///     code: Code::runtime("lending", "loans", "overdue").unwrap().to_buf(),
/// };
/// assert_eq!(failure.code(), Code::new("lending", "loans", "overdue"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CodeBuf {
    /// The component that owns the code, such as `shelf`.
    component: Box<str>,
    /// The area of the tool, such as `books`.
    area: Box<str>,
    /// The reason for the failure, such as `not_found`.
    reason: Box<str>,
}

impl CodeBuf {
    /// Returns the code, which borrows its segments from this value.
    #[must_use]
    pub fn as_code(&self) -> Code<'_> {
        Code {
            component: &self.component,
            area: &self.area,
            reason: &self.reason,
        }
    }
}

/// The three segments of a [`Code`], in the order that [`Code::new`] and
/// [`Code::runtime`] take them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Segment {
    /// The first segment, which names the tool or the plugin that
    /// raises the failure.
    Component,
    /// The second segment, which names the part of the component that
    /// raises the failure.
    Area,
    /// The third segment, which names why the failure happened.
    Reason,
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Component => "component",
            Self::Area => "area",
            Self::Reason => "reason",
        })
    }
}

/// Why [`Code::runtime`] returned no code.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeCodeError {
    /// A segment breaks the rule of [`Code::new`]: it is not words of
    /// lowercase ASCII letters, digits and underscores joined by single
    /// hyphens. The variant may gain fields, so a match on it ends with
    /// `..`.
    #[non_exhaustive]
    Segment {
        /// Which segment broke the rule.
        segment: Segment,
        /// The segment's text.
        text: String,
    },
}

impl fmt::Display for RuntimeCodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Segment { segment, text } => write!(
                f,
                "the code {segment} {text:?} is not words of lowercase ASCII letters, digits and underscores joined by single hyphens"
            ),
        }
    }
}

impl std::error::Error for RuntimeCodeError {}
