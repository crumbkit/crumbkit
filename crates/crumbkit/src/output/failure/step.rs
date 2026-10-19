//! A failure's recovery ready to print, with a `Run` call rendered as text.

use std::borrow::Cow;

use clap::Command;
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

use super::UNAVAILABLE;
use crate::help::quote;
use crate::{CallArg, CallTarget, FieldValue, Recovery};

/// A [`Recovery`] ready to print: a `Run` call is rendered against the tool's
/// command, and a call that cannot be rendered becomes `None` with
/// crumbkit's reason.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Step {
    /// Run this command line, such as `shelf books list`.
    Run(String),
    /// Run this command line once with every value of the failure's list
    /// field `field` in place of the argument's placeholder, such as
    /// `shelf books show <ID>...` with every value of `left_out`. A call
    /// built with `Call::all` or `Call::typed_all` renders as this step.
    RunAll {
        /// The command line, with the argument's placeholder in place of the
        /// values.
        call: String,
        /// The name of the failure's field whose values the call takes.
        field: String,
    },
    /// Run the same command again later.
    Retry,
    /// Follow this advice.
    Hint(String),
    /// There is no next step, for this reason.
    None(String),
}

impl Step {
    /// Reads `recovery` into a step, rendering a `Run` call against
    /// `command`. It returns `Err` with a `Step::None` when the recovery
    /// offers no step: a `Recovery::None`, or a `Run` call that cannot be
    /// rendered because the call does not resolve. It adds the reason that a
    /// `Run` call does not resolve to `unrendered`.
    pub(super) fn offered(
        recovery: Recovery,
        command: &mut Command,
        unrendered: &mut Vec<String>,
    ) -> Result<Self, Self> {
        match recovery {
            Recovery::Run(call) => {
                let rendered = crate::call::render(&call, command).map_err(|e| e.to_string());
                let all = all_field(&call);
                rendered
                    .map(|call| match all {
                        Some(field) => Self::RunAll {
                            call,
                            field: field.to_owned(),
                        },
                        None => Self::Run(call),
                    })
                    .map_err(|reason| {
                        unrendered.push(reason);
                        Self::None(UNAVAILABLE.to_owned())
                    })
            }
            Recovery::Retry => Ok(Self::Retry),
            Recovery::Hint(text) => Ok(Self::Hint(text)),
            Recovery::None { reason } => Err(Self::None(reason)),
        }
    }

    /// Returns the text of the step's `help:` line. A `RunAll` step prints
    /// its call with the values of its field from `fields` in place of the
    /// placeholder, each quoted for a POSIX shell when a shell would split
    /// or expand it, such as ``run `shelf books show 4 5 'Le Guin'` ``. When
    /// the field holds no value, the line names the call and the field.
    pub(super) fn help(&self, fields: &[(String, FieldValue<'static>)]) -> Cow<'_, str> {
        match self {
            Self::Run(call) => Cow::Owned(format!("run `{call}`")),
            Self::RunAll { call, field } => {
                let values = fields
                    .iter()
                    .find(|(name, _)| name == field)
                    .map(|(_, value)| list_words(value))
                    .unwrap_or_default();
                match placeholder(call) {
                    Some((start, end)) if !values.is_empty() => Cow::Owned(format!(
                        "run `{}{}{}`",
                        &call[..start],
                        values.join(" "),
                        &call[end..]
                    )),
                    _ => Cow::Owned(format!("run `{call}` with every value of {field}")),
                }
            }
            Self::Retry => Cow::Borrowed("run the same command again later"),
            Self::Hint(text) => Cow::Borrowed(text),
            Self::None(reason) => Cow::Owned(format!("no next step: {reason}")),
        }
    }
}

/// Returns the field that a `Call::all` argument of `call`, or the place of
/// a call from `Call::typed_all`, takes its values from, or `None` when the
/// call has neither.
fn all_field(call: &crate::Call) -> Option<&'static str> {
    if let Some((_, _, field)) = call.typed_slot() {
        return Some(field);
    }
    let CallTarget::Command { args, .. } = call.target() else {
        return None;
    };
    args.iter().find_map(|arg| match arg {
        CallArg::All { field, .. } => Some(*field),
        CallArg::Placeholder(_) | CallArg::Value { .. } => None,
    })
}

/// Returns the start and end of the first word of `call` that is a
/// placeholder, such as `<ID>...` or `<ID>`, or `None` when it has none.
///
/// The words of a rendered call are separated by spaces, and a value that
/// holds a space or a `<` sits between single quotes, so the scan skips the
/// text between quotes and the `\'` that `quote` writes for a quote inside a
/// value. A call from `Call::all` holds no other placeholder, which
/// rendering enforces, and a call from `Call::typed_all` quotes each of its
/// words, so the first placeholder is the one the values replace.
fn placeholder(call: &str) -> Option<(usize, usize)> {
    let mut quoted = false;
    let mut escaped = false;
    let mut start = 0;
    for (index, c) in call.char_indices().chain([(call.len(), ' ')]) {
        if escaped {
            escaped = false;
        } else if quoted {
            quoted = c != '\'';
        } else if c == '\\' {
            escaped = true;
        } else if c == '\'' {
            quoted = true;
        } else if c == ' ' {
            if is_placeholder(&call[start..index]) {
                return Some((start, index));
            }
            start = index + 1;
        }
    }
    None
}

/// Returns whether `word` is one placeholder: `<`, one or more ASCII
/// letters, digits, `_` or `-`, and `>`, with clap's `...` after it when the
/// argument takes many values.
fn is_placeholder(word: &str) -> bool {
    let word = word.strip_suffix("...").unwrap_or(word);
    word.strip_prefix('<')
        .and_then(|rest| rest.strip_suffix('>'))
        .is_some_and(|name| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
}

/// Returns the items of a list field as words that a caller types: text
/// quoted for a POSIX shell, and any other value as its JSON. A value that
/// is not a list is one word.
fn list_words(value: &FieldValue<'_>) -> Vec<String> {
    let FieldValue::List(items) = value else {
        return vec![item_text(value)];
    };
    items.iter().map(item_text).collect()
}

/// Returns one value as a caller types it: text quoted for a POSIX shell,
/// and any other value as its JSON.
fn item_text(value: &FieldValue<'_>) -> String {
    match value {
        FieldValue::Text(text) => quote(text),
        // A field value's keys are all strings, so the conversion cannot
        // fail, and an empty string stands in rather than a panic if it ever
        // did.
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// Serializes the step as an object tagged by `kind`, such as
/// `{"kind":"run","call":"shelf books list"}` or `{"kind":"retry"}`. A
/// `RunAll` step is a `run` step with `all`, the name of its field, such as
/// `{"kind":"run","call":"shelf books show <ID>...","all":"left_out"}`.
impl Serialize for Step {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (kind, field, all) = match self {
            Self::Run(call) => ("run", Some(("call", call)), None),
            Self::RunAll { call, field } => ("run", Some(("call", call)), Some(field)),
            Self::Retry => ("retry", None, None),
            Self::Hint(text) => ("hint", Some(("text", text)), None),
            Self::None(reason) => ("none", Some(("reason", reason)), None),
        };
        let entries = 1 + usize::from(field.is_some()) + usize::from(all.is_some());
        let mut map = serializer.serialize_map(Some(entries))?;
        map.serialize_entry("kind", kind)?;
        if let Some((name, value)) = field {
            map.serialize_entry(name, value)?;
        }
        if let Some(all) = all {
            map.serialize_entry("all", all)?;
        }
        map.end()
    }
}
