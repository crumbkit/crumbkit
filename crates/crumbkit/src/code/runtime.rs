//! Failure codes known only at run time, such as the code that a plugin
//! answers with, and the owned code that a failure keeps them in.

use std::fmt;

use super::{Code, SpellingError, split_spelling};

#[cfg(test)]
mod tests;

impl<'a> Code<'a> {
    /// Returns the code with the spelling `spelling`, such as
    /// `lending::loans::overdue`, which the tool learns only at run time,
    /// such as from a plugin's answer.
    ///
    /// It applies the rule of [`Code::from_spelling`] and returns
    /// [`RuntimeCodeError::Parts`] for text that does not split into three
    /// segments at `::`, and [`RuntimeCodeError::Segment`] for the first
    /// segment that breaks the rule. The returned code borrows `spelling`,
    /// and crumbkit keeps no copy of it, so a tool makes any number of
    /// run-time codes. A failure that keeps the code holds the [`CodeBuf`]
    /// that [`Code::to_buf`] returns, and answers
    /// [`Failure::code`](crate::Failure::code) with [`CodeBuf::as_code`].
    ///
    /// A `CODES` list cannot name run-time codes, because the tool does not
    /// know them when its tests run. A failure with a run-time code supplies
    /// its own recovery, which is `Recovery::None` with a reason when the plugin
    /// gives no next step.
    ///
    /// ```
    /// use crumbkit::{Code, Spelling};
    ///
    /// const PLUGIN_FAILED: Code<'static> = Code::from_spelling("shelf::plugin::failed");
    ///
    /// let answer = "lending::loans::overdue";
    /// let code = Code::runtime(answer).unwrap_or(PLUGIN_FAILED);
    ///
    /// assert_eq!(Spelling::COLONS.spell(code), "lending::loans::overdue");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeCodeError::Parts`] when `spelling` does not split
    /// into three segments, and [`RuntimeCodeError::Segment`] when a segment
    /// breaks the rule of [`Code::from_spelling`].
    pub fn runtime(spelling: &'a str) -> Result<Self, RuntimeCodeError> {
        match split_spelling(spelling) {
            Ok((area, reason)) => Ok(Self {
                spelling,
                area,
                reason,
            }),
            Err(SpellingError::Parts) => Err(RuntimeCodeError::Parts {
                text: spelling.to_owned(),
            }),
            Err(SpellingError::Segment(segment)) => {
                let text = match segment {
                    Segment::Component => spelling.split("::").next().unwrap_or_default(),
                    Segment::Area => spelling.split("::").nth(1).unwrap_or_default(),
                    Segment::Reason => spelling.split("::").nth(2).unwrap_or_default(),
                };
                Err(RuntimeCodeError::Segment {
                    segment,
                    text: text.to_owned(),
                })
            }
        }
    }

    /// Returns a copy of the code that owns its spelling.
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
    ///     Code::runtime(&answer).unwrap().to_buf()
    /// };
    ///
    /// assert_eq!(Spelling::COLONS.spell(owned.as_code()), "lending::loans::overdue");
    /// ```
    #[must_use]
    pub fn to_buf(self) -> CodeBuf {
        CodeBuf {
            spelling: self.spelling.into(),
            area: self.area,
            reason: self.reason,
        }
    }
}

/// A failure [`Code`] that owns its spelling.
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
///     code: Code::runtime("lending::loans::overdue").unwrap().to_buf(),
/// };
/// assert_eq!(failure.code(), Code::from_spelling("lending::loans::overdue"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CodeBuf {
    /// The spelling, such as `shelf::books::not_found`.
    spelling: Box<str>,
    /// The byte offset where the area starts.
    area: usize,
    /// The byte offset where the reason starts.
    reason: usize,
}

impl CodeBuf {
    /// Returns the code, which borrows its segments from this value.
    #[must_use]
    pub fn as_code(&self) -> Code<'_> {
        Code {
            spelling: &self.spelling,
            area: self.area,
            reason: self.reason,
        }
    }
}

/// The three segments of a [`Code`], in the order of its spelling.
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
    /// The text does not split into three segments at `::`.
    #[non_exhaustive]
    Parts {
        /// The text.
        text: String,
    },
    /// A segment breaks the rule of [`Code::from_spelling`]: it is not words of
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
            Self::Parts { text } => write!(
                f,
                "the code {text:?} is not three segments joined by `::`, such as `shelf::books::not_found`"
            ),
            Self::Segment { segment, text } => write!(
                f,
                "the code {segment} {text:?} is not words of lowercase ASCII letters, digits and underscores joined by single hyphens"
            ),
        }
    }
}

impl std::error::Error for RuntimeCodeError {}
