//! Which names `--fields` accepts for a result, and which view flags a
//! result refuses because it cannot use them.

use schemars::Schema;

use super::Output;
use crate::help::ViewFlag;
use crate::help::view::{self, Kind, Printed};
use crate::refusal::Refusal;
use crate::row;
use crate::view::View;
use crate::{Document, Row, Rows};

/// Returns the refusal of the view in `output` for a result of type `T`
/// printed as `document`, or `None` when the result accepts it, as
/// [`refusal`] describes.
pub(super) fn flags_refusal<T: Rows>(output: &Output, document: Document<T>) -> Option<Refusal> {
    let printed = Printed::of::<T::Row, T>(Kind::Document(T::SHAPE), &document);
    refusal::<T::Row>(&printed, output.view())
}

/// Returns the refusal of the view flags that selected `view`, for a
/// command that prints `printed` with rows of type `R`. It returns `None`
/// when the result accepts them.
///
/// It first refuses a view flag that the result does not offer, by the
/// rule of [`view::unoffered`], in the order `--fields`, `--long`,
/// `--full`, and accepts a flag that adds nothing to the JSON, as
/// [`view::adds_nothing`] decides. For a headed document it then refuses
/// `--fields` without `--json`, as [`view::headed`] describes. It then
/// refuses the first name in `view` that is not a field of `R`, as
/// [`unknown_field`] describes.
/// The pairs that cannot work together were refused when `view` was built
/// (ADR-0093).
pub(super) fn refusal<R: Row>(printed: &Printed, view: &View) -> Option<Refusal> {
    let names = view.names();
    for flag in ViewFlag::ALL {
        let typed = match flag {
            ViewFlag::Fields => names.is_some(),
            ViewFlag::Long => view.is_long(),
            ViewFlag::Full => view.is_full(),
        };
        if typed
            && !view::adds_nothing(printed, flag, view.is_json())
            && let Some(reason) = view::unoffered(printed, flag)
        {
            return Some(view::refusal(flag, reason));
        }
    }
    if printed.headed
        && let Some(refusal) = view::headed(names.is_some(), view.is_json())
    {
        return Some(refusal);
    }
    unknown_field::<R>(names?)
}

/// Returns the note that the view flag in `view` adds nothing to the JSON
/// of a command that prints `printed`, as [`view::adds_nothing`] decides,
/// or `None` when it changes the result (ADR-0149). `finish` and `stream`
/// deliver it as a note after the result.
///
/// `--full` with `--long` selects every field, as `--full` alone does, so
/// the view holds `--full` alone and the note names `--full` once.
pub(super) fn adds_nothing_note(printed: &Printed, view: &View) -> Option<String> {
    let flag = if view.is_full() {
        ViewFlag::Full
    } else if view.is_long() {
        ViewFlag::Long
    } else {
        return None;
    };
    view::adds_nothing(printed, flag, view.is_json()).then(|| view::adds_nothing_note(flag))
}

/// Returns the refusal of the first name in `fields` that is not a field of
/// `R`, with a hint that lists the fields of `R`, or `None` when every name
/// is a field.
///
/// The fields of `R` are the properties that [`row::properties`] reads from
/// its JSON Schema, so a mistyped name fails even when the result has no
/// rows.
fn unknown_field<R: Row>(fields: &[String]) -> Option<Refusal> {
    let schema = crate::schemas::settings()
        .into_generator()
        .into_root_schema_for::<R>();
    unknown_field_in(&schema, fields)
}

/// Returns the refusal of the first name in `fields` that is not a
/// property of the row type whose JSON Schema is `schema`, as
/// [`unknown_field`] describes. `parse` calls it with the row schema of the
/// leaf's document.
pub(crate) fn unknown_field_in(schema: &Schema, fields: &[String]) -> Option<Refusal> {
    let properties = row::properties(schema);
    let is_field = |name: &str| properties.iter().any(|(field, _)| *field == name);
    let unknown = fields.iter().find(|name| !is_field(name))?;

    // serde_json keeps an object's keys sorted unless a crate in the build
    // enables its `preserve_order` feature, which keeps the schema's order.
    let valid: Vec<&str> = properties.iter().map(|(name, _)| *name).collect();
    Some(Refusal::flag_value(
        format!("--fields names an unknown field '{unknown}'"),
        format!("use the valid fields: {}", valid.join(", ")),
        "--fields",
        Some(unknown.clone()),
    ))
}
