//! Which names `--fields` accepts for a result.

use crate::refusal::Refusal;
use crate::row;
use crate::{Row, Rows, Shape};

/// Returns the refusal of `fields` for a result of type `T`, or `None` when
/// `T` accepts every name in `fields`.
///
/// A prose result refuses any name, because it has no columns. Any other
/// result refuses the first name that is not a field of its row type, as
/// [`unknown_field`] describes.
pub(super) fn refusal<T: Rows>(fields: &[String]) -> Option<Refusal> {
    if fields.is_empty() {
        return None;
    }
    if T::SHAPE == Shape::Prose {
        return Some(Refusal::flag_value(
            "--fields selects the columns of a table, and this command prints prose".to_owned(),
            "use --json to read every field of the result".to_owned(),
        ));
    }
    unknown_field::<T::Row>(fields)
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
    ))
}
