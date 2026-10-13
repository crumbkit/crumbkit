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
    /// Run this command line once for each value of the failure's list
    /// field `field`, with the value in place of the argument's value name,
    /// such as `shelf books show <ID>` for each value of `left_out`. A call
    /// built with `Call::each` renders as this step.
    RunEach {
        /// The command line, with the argument's value name in place of
        /// each value.
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
    /// rendered because no command is set or the call does not resolve. It
    /// adds the reason that a `Run` call does not render to `unrendered`.
    pub(super) fn offered(
        recovery: Recovery,
        command: Option<&mut Command>,
        unrendered: &mut Vec<String>,
    ) -> Result<Self, Self> {
        match recovery {
            Recovery::Run(call) => {
                let rendered = match command {
                    Some(command) => crate::call::render(&call, command).map_err(|e| e.to_string()),
                    None => Err("no command is set".to_owned()),
                };
                let each = each_field(&call);
                rendered
                    .map(|call| match each {
                        Some(field) => Self::RunEach {
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
            // `Recovery` is `#[non_exhaustive]` and lives in crumbkit-core, so
            // the compiler requires this arm. crumbkit-core releases with
            // crumbkit (ADR-0012), so a new variant gets its own arm in the
            // release that adds it, and this arm matches nothing until then.
            _ => Err(Self::None(UNAVAILABLE.to_owned())),
        }
    }

    /// Returns the text of the step's `help:` line. A `RunEach` step lists
    /// the values of its field from `fields`, each quoted for a POSIX shell
    /// when a shell would split or expand it, such as ``run `shelf books
    /// show <ID>` for each value of left_out: 4, 5, 6``.
    pub(super) fn help(&self, fields: &[(String, FieldValue<'static>)]) -> Cow<'_, str> {
        match self {
            Self::Run(call) => Cow::Owned(format!("run `{call}`")),
            Self::RunEach { call, field } => {
                let values = fields
                    .iter()
                    .find(|(name, _)| name == field)
                    .map(|(_, value)| list_text(value))
                    .unwrap_or_default();
                Cow::Owned(format!("run `{call}` for each value of {field}: {values}"))
            }
            Self::Retry => Cow::Borrowed("run the same command again later"),
            Self::Hint(text) => Cow::Borrowed(text),
            Self::None(reason) => Cow::Owned(format!("no next step: {reason}")),
        }
    }
}

/// Returns the field that a `Call::each` argument of `call` takes its values
/// from, or `None` when the call has no such argument.
fn each_field(call: &crate::Call) -> Option<&'static str> {
    let CallTarget::Command { args, .. } = call.target() else {
        return None;
    };
    args.iter().find_map(|arg| match arg {
        CallArg::Each { field, .. } => Some(*field),
        CallArg::Placeholder(_) | CallArg::Value { .. } => None,
    })
}

/// Returns the items of a list field joined by `, `, each as a caller types
/// it: text quoted for a POSIX shell, and any other value as its JSON. A
/// value that is not a list prints as its JSON alone.
fn list_text(value: &FieldValue<'_>) -> String {
    let FieldValue::List(items) = value else {
        return item_text(value);
    };
    items.iter().map(item_text).collect::<Vec<_>>().join(", ")
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
/// `RunEach` step is a `run` step with `each`, the name of its field, such
/// as `{"kind":"run","call":"shelf books show <ID>","each":"left_out"}`.
impl Serialize for Step {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (kind, field, each) = match self {
            Self::Run(call) => ("run", Some(("call", call)), None),
            Self::RunEach { call, field } => ("run", Some(("call", call)), Some(field)),
            Self::Retry => ("retry", None, None),
            Self::Hint(text) => ("hint", Some(("text", text)), None),
            Self::None(reason) => ("none", Some(("reason", reason)), None),
        };
        let entries = 1 + usize::from(field.is_some()) + usize::from(each.is_some());
        let mut map = serializer.serialize_map(Some(entries))?;
        map.serialize_entry("kind", kind)?;
        if let Some((name, value)) = field {
            map.serialize_entry(name, value)?;
        }
        if let Some(each) = each {
            map.serialize_entry("each", each)?;
        }
        map.end()
    }
}
