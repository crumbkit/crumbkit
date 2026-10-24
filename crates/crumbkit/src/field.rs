//! The named values of a failure, which `Failure::fields` returns.

use std::borrow::Cow;

use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};

use crate::Shown;
use crate::segment::is_segment;
use crate::shown::JsonForm;

#[cfg(test)]
mod tests;

/// A value at fault in a failure under a name, such as the id of a book
/// that the catalogue does not hold.
///
/// [`Failure::fields`](crate::Failure::fields) returns a failure's fields,
/// and the JSON failure document holds them under `fields`, so an agent can
/// read the value without parsing the message. The text output prints no
/// extra line, because the message already describes the value for a
/// person.
///
/// A name follows the rule of a code segment: one or more lowercase ASCII
/// letters, digits and underscores, such as `book_id`.
///
/// ```
/// use crumbkit::{Field, FieldValue};
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
    /// integer, a boolean, null, or a list or object of such values, as
    /// [`FieldValue`] describes.
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

    /// Returns the field named `name` that holds the JSON form of a
    /// [`Shown`] value, such as a [`Duration`](crate::Duration).
    ///
    /// The failure's message holds the value's text form, which the tool
    /// writes with the value's `Display`, and the JSON failure document holds
    /// [`Shown::json`] under `fields`. A JSON form that `FieldValue` cannot
    /// hold, such as a number with a fraction, gives the text form as
    /// [`FieldValue::Text`].
    ///
    /// ```
    /// use std::time::Duration as Std;
    ///
    /// use crumbkit::{Duration, Field, FieldValue};
    ///
    /// let waited = Duration(Std::from_secs(90));
    /// let message = format!("the shelf stayed locked for {waited}");
    /// let field = Field::shown("waited", &waited);
    ///
    /// assert_eq!(message, "the shelf stayed locked for 1m 30s");
    /// assert_eq!(field.value(), &FieldValue::from("PT1M30S"));
    /// ```
    //
    // RUST-10: the JSON form passes through a `serde_json::Value`, which
    // sorts an object's keys unless a crate in the build turns on
    // `serde_json`'s `preserve_order` feature, so an object keeps its key
    // order only then. A serializer that writes a `FieldValue` directly
    // replaces it once a tool needs the order kept.
    #[must_use]
    pub fn shown<T: Shown + ?Sized>(name: &'a str, value: &T) -> Self {
        let json = serde_json::to_value(JsonForm(value)).ok();
        let value = json.as_ref().and_then(FieldValue::from_json).map_or_else(
            || FieldValue::from(value.to_string()),
            FieldValue::into_owned,
        );
        Self { name, value }
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

/// The value of a [`Field`]: a JSON string, integer, boolean or null, or a
/// list or object of such values.
///
/// It has no floating-point variant at any depth, because JSON has no form
/// for NaN or an infinity. Each `From` impl picks the variant of its type,
/// so a caller writes `Field::new("book", id)` with the id's own type, and
/// an `Option` gives `Null` for `None`. [`FieldValue::list`] and
/// [`FieldValue::object`] build the values that hold other values. A tool
/// chooses the shape of its own fields, so no shape needs an opt-out.
///
/// ```
/// use crumbkit::{Field, FieldValue};
///
/// let refusals = [(3_u32, "not a number"), (9, "empty")];
/// let field = Field::new(
///     "refusals",
///     FieldValue::list(refusals.iter().map(|&(line, reason)| {
///         FieldValue::object([
///             ("line", FieldValue::from(line)),
///             ("reason", FieldValue::from(reason)),
///         ])
///     })),
/// );
/// let found = Field::new("found", None::<&str>);
///
/// assert_eq!(found.value(), &FieldValue::Null);
/// ```
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
    /// JSON null, such as a `found` field when nothing was found.
    Null,
    /// A JSON array of values, in order.
    List(Vec<FieldValue<'a>>),
    /// A JSON object of keys and values, in the order given. A key is any
    /// text. `crumbkit::finish` leaves out an entry whose key an earlier
    /// entry of the same object has.
    Object(Vec<(Cow<'a, str>, FieldValue<'a>)>),
}

impl<'a> FieldValue<'a> {
    /// Returns a list of `items`, each converted as [`Field::new`] converts
    /// a value, such as `FieldValue::list(&tables)` for a `Vec<String>`.
    #[must_use]
    pub fn list<V: Into<Self>>(items: impl IntoIterator<Item = V>) -> Self {
        Self::List(items.into_iter().map(Into::into).collect())
    }

    /// Returns an object of `entries`, each a key and a value, in the order
    /// given, such as `FieldValue::object([("line", FieldValue::from(3))])`.
    #[must_use]
    pub fn object<K, V>(entries: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<Cow<'a, str>>,
        V: Into<Self>,
    {
        Self::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        )
    }

    /// Returns the field value that serializes to the JSON value `value`,
    /// borrowing its text, or `None` when `value` holds a number at any
    /// depth that `FieldValue` cannot hold, such as a fraction.
    ///
    /// A tool or a test that reads a failure's `fields` back from
    /// JSON uses it. A non-negative integer becomes `Unsigned` and a
    /// negative one `Integer`, because JSON does not say which Rust type
    /// wrote it. `serde_json` holds a number with a fraction or an exponent,
    /// and an integer below `i64::MIN` or above `u64::MAX`, as a float,
    /// and `FieldValue` has no float, so such a number gives `None`. An
    /// object's entries keep the order of `serde_json`'s map, which sorts
    /// the keys unless a crate in the build turns on `serde_json`'s
    /// `preserve_order` feature.
    ///
    /// ```
    /// use crumbkit::FieldValue;
    ///
    /// let json = serde_json::json!({"book": 7, "found": null});
    /// let fraction = serde_json::json!(1.5);
    ///
    /// assert_eq!(
    ///     FieldValue::from_json(&json),
    ///     Some(FieldValue::object([
    ///         ("book", FieldValue::from(7_u32)),
    ///         ("found", FieldValue::Null),
    ///     ]))
    /// );
    /// assert_eq!(FieldValue::from_json(&fraction), None);
    /// ```
    #[must_use]
    pub fn from_json(value: &'a serde_json::Value) -> Option<Self> {
        use serde_json::Value;

        let value = match value {
            Value::Null => Self::Null,
            Value::Bool(value) => Self::Boolean(*value),
            Value::Number(number) => match number.as_u64() {
                Some(value) => Self::Unsigned(value),
                None => Self::Integer(number.as_i64()?),
            },
            Value::String(text) => Self::Text(Cow::Borrowed(text)),
            Value::Array(items) => Self::List(
                items
                    .iter()
                    .map(Self::from_json)
                    .collect::<Option<Vec<_>>>()?,
            ),
            Value::Object(entries) => Self::Object(
                entries
                    .iter()
                    .map(|(key, value)| {
                        Some((Cow::Borrowed(key.as_str()), Self::from_json(value)?))
                    })
                    .collect::<Option<Vec<_>>>()?,
            ),
        };
        Some(value)
    }
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
            Self::Null => FieldValue::Null,
            Self::List(items) => {
                FieldValue::List(items.into_iter().map(FieldValue::into_owned).collect())
            }
            Self::Object(entries) => FieldValue::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (Cow::Owned(key.into_owned()), value.into_owned()))
                    .collect(),
            ),
        }
    }
}

/// Serializes the value as a JSON string, number, boolean, null, array or
/// object, with an object's keys in their order.
impl Serialize for FieldValue<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Text(text) => serializer.serialize_str(text),
            Self::Integer(value) => serializer.serialize_i64(*value),
            Self::Unsigned(value) => serializer.serialize_u64(*value),
            Self::Boolean(value) => serializer.serialize_bool(*value),
            Self::Null => serializer.serialize_none(),
            Self::List(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Self::Object(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key.as_ref(), value)?;
                }
                map.end()
            }
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

impl<'a, T: Into<FieldValue<'a>>> From<Option<T>> for FieldValue<'a> {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Into::into)
    }
}
