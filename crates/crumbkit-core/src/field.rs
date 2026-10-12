//! The named values of a failure, which `Failure::fields` returns.

use std::borrow::Cow;

use serde::{Serialize, Serializer};

use crate::segment::is_segment;

#[cfg(test)]
mod tests;

/// A value at fault in a failure under a name, such as the id of a book
/// that the catalogue does not hold.
///
/// [`Failure::fields`](crate::Failure::fields) returns a failure's fields,
/// and the JSON failure document holds them under `fields`, so an agent can
/// read the value without parsing the message. The text output prints no
/// extra line, because the message already names the value for a person.
///
/// A name follows the rule of a code segment: one or more lowercase ASCII
/// letters, digits and underscores, such as `book_id`.
///
/// ```
/// use crumbkit_core::{Field, FieldValue};
///
/// let field = Field::new("book", 7_u32);
///
/// assert_eq!(field.name(), "book");
/// assert_eq!(field.value(), &FieldValue::Unsigned(7));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field<'a> {
    /// The field's name, such as `book`.
    name: &'a str,
    /// The field's value.
    value: FieldValue<'a>,
}

impl<'a> Field<'a> {
    /// Returns the field named `name` that holds `value`, which is text, an
    /// integer or a boolean.
    ///
    /// `crumbkit::finish` leaves out a field whose name breaks the rule that
    /// [`Field::follows_name_rule`] checks, and a field whose name an earlier
    /// field of the same failure has.
    #[must_use]
    pub fn new(name: &'a str, value: impl Into<FieldValue<'a>>) -> Self {
        Self {
            name,
            value: value.into(),
        }
    }

    /// Returns the field's name.
    #[must_use]
    pub const fn name(&self) -> &'a str {
        self.name
    }

    /// Returns the field's value.
    #[must_use]
    pub const fn value(&self) -> &FieldValue<'a> {
        &self.value
    }

    /// Returns whether the name is one or more lowercase ASCII letters,
    /// digits and underscores, the rule that a code segment also follows.
    ///
    /// The failure document's JSON Schema promises every reader this rule
    /// for the names under `fields`.
    #[must_use]
    pub const fn follows_name_rule(&self) -> bool {
        is_segment(self.name, b'_')
    }
}

/// The value of a [`Field`]: a JSON string, integer or boolean.
///
/// It has no floating-point variant, because JSON has no form for NaN or an
/// infinity. Each `From` impl picks the variant of its type, so a caller
/// writes `Field::new("book", id)` with the id's own type.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldValue<'a> {
    /// Text, borrowed from the failure or owned.
    Text(Cow<'a, str>),
    /// A signed integer.
    Integer(i64),
    /// An unsigned integer.
    Unsigned(u64),
    /// A boolean.
    Boolean(bool),
}

impl FieldValue<'_> {
    /// Returns the value with any borrowed text copied, so it no longer
    /// borrows from the failure.
    #[must_use]
    pub fn into_owned(self) -> FieldValue<'static> {
        match self {
            Self::Text(text) => FieldValue::Text(Cow::Owned(text.into_owned())),
            Self::Integer(value) => FieldValue::Integer(value),
            Self::Unsigned(value) => FieldValue::Unsigned(value),
            Self::Boolean(value) => FieldValue::Boolean(value),
        }
    }
}

/// Serializes the value as a JSON string, number or boolean.
impl Serialize for FieldValue<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Text(text) => serializer.serialize_str(text),
            Self::Integer(value) => serializer.serialize_i64(*value),
            Self::Unsigned(value) => serializer.serialize_u64(*value),
            Self::Boolean(value) => serializer.serialize_bool(*value),
        }
    }
}

impl<'a> From<&'a str> for FieldValue<'a> {
    fn from(text: &'a str) -> Self {
        Self::Text(Cow::Borrowed(text))
    }
}

impl<'a> From<&'a String> for FieldValue<'a> {
    fn from(text: &'a String) -> Self {
        Self::Text(Cow::Borrowed(text))
    }
}

impl From<String> for FieldValue<'_> {
    fn from(text: String) -> Self {
        Self::Text(Cow::Owned(text))
    }
}

impl<'a> From<Cow<'a, str>> for FieldValue<'a> {
    fn from(text: Cow<'a, str>) -> Self {
        Self::Text(text)
    }
}

impl From<i8> for FieldValue<'_> {
    fn from(value: i8) -> Self {
        Self::Integer(value.into())
    }
}

impl From<i16> for FieldValue<'_> {
    fn from(value: i16) -> Self {
        Self::Integer(value.into())
    }
}

impl From<i32> for FieldValue<'_> {
    fn from(value: i32) -> Self {
        Self::Integer(value.into())
    }
}

impl From<i64> for FieldValue<'_> {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<u8> for FieldValue<'_> {
    fn from(value: u8) -> Self {
        Self::Unsigned(value.into())
    }
}

impl From<u16> for FieldValue<'_> {
    fn from(value: u16) -> Self {
        Self::Unsigned(value.into())
    }
}

impl From<u32> for FieldValue<'_> {
    fn from(value: u32) -> Self {
        Self::Unsigned(value.into())
    }
}

impl From<u64> for FieldValue<'_> {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

impl From<usize> for FieldValue<'_> {
    fn from(value: usize) -> Self {
        // A `usize` has at most 64 bits on every platform that Rust's
        // standard library supports, so the conversion does not fail there.
        Self::Unsigned(u64::try_from(value).unwrap_or(u64::MAX))
    }
}

impl From<bool> for FieldValue<'_> {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}
