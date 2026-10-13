//! Which names `--fields` accepts for a result, and which view flags a
//! result refuses because it cannot use them.

use super::Output;
use crate::help::ViewFlag;
use crate::help::view::{self, Typed};
use crate::refusal::Refusal;
use crate::row;
use crate::{Row, Rows, Shape};

/// Returns the refusal of the view flags in `output` for a result of type
/// `T` in a document that has crumbkit's envelope when `envelope` is set,
/// or `None` when the result accepts them, as [`refusal`] describes.
pub(super) fn flags_refusal<T: Rows>(output: &Output, envelope: bool) -> Option<Refusal> {
    refusal::<T::Row>(Some(T::SHAPE), envelope, &output.fields, output.typed())
}

/// Returns the refusal of the view flags that the caller typed, the names
/// in `fields` and the flags in `typed`, for a result of `shape` whose rows
/// are `R`,
/// or a stream of `R` when `shape` is `None`. It returns `None` when the
/// result accepts them.
///
/// It first refuses a typed view flag that the result does not offer, by
/// the rule of [`view::unoffered`], in the order `--fields`, `--long`,
/// `--full`. It then refuses two flags that cannot work together, as
/// [`view::pairing`] describes, and then the first name in `fields` that is
/// not a field of `R`, as [`unknown_field`] describes.
pub(super) fn refusal<R: Row>(
    shape: Option<Shape>,
    envelope: bool,
    fields: &[String],
    typed: Typed,
) -> Option<Refusal> {
    let long_columns = !R::LONG_COLUMNS.is_empty();
    for flag in ViewFlag::ALL {
        let typed = match flag {
            ViewFlag::Fields => typed.fields,
            ViewFlag::Long => typed.long,
            ViewFlag::Full => typed.full,
        };
        if typed && let Some(reason) = view::unoffered(shape, long_columns, envelope, flag) {
            return Some(view::refusal(flag, reason));
        }
    }
    if let Some(refusal) = view::pairing(typed) {
        return Some(refusal);
    }
    if fields.is_empty() {
        return None;
    }
    unknown_field::<R>(fields)
}

/// Returns the refusal of the first name in `fields` that is not a field of
/// `R`, with a hint that lists the fields of `R`, or `None` when every name
/// is a field.
///
/// The fields of `R` are the properties that [`row::properties`] reads from
/// its JSON Schema, so a mistyped name fails even when the result has no
/// rows.
fn unknown_field<R: Row>(fields: &[String]) -> Option<Refusal> {
    let schema = schemars::schema_for!(R);
    let properties = row::properties(&schema);
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
