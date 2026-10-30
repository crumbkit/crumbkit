//! Value types that print their `Display` in text and a form of their own in
//! JSON (ADR-0147).

use std::borrow::Cow;
use std::cell::Cell;
use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Serialize, Serializer};

#[cfg(test)]
mod tests;

/// A value that a tool prints as its `Display` where a person reads it, and
/// in a form of its own in JSON.
///
/// The text form is the `Display`. crumbkit prints it in the cells of a text
/// table, in `--fields` text, in a stream's text line, in the default
/// [`Row::child_lines`](crate::Row::child_lines) and in the message of a
/// failure whose [`Field::shown`](crate::Field::shown) holds the value. The
/// JSON form is [`Shown::json`], which `--json` in every view, a stream's
/// JSON lines, a failure's `fields` and a tool that serializes the value
/// itself all hold. [`Shown::schema`] describes the JSON form in the row's
/// schema.
///
/// The defaults make the JSON form the text form as a string, so a status
/// or an id that reads the same in both forms implements the trait with an
/// empty body. A value with a typed JSON form, such as a number with a unit
/// in text, overrides [`Shown::json`] and [`Shown::schema`] together.
///
/// [`shown!`](crate::shown!) then writes the type's `Serialize` and
/// `JsonSchema`, and the type is an ordinary field of a row:
///
/// ```
/// use std::fmt;
///
/// use crumbkit::Shown;
/// use crumbkit::schemars::{JsonSchema, Schema, SchemaGenerator};
/// use crumbkit::serde::Serializer;
///
/// /// A number of pages, which reads `432 pages` in text and `432` in JSON.
/// #[derive(Clone, Copy)]
/// struct Pages(u32);
///
/// impl fmt::Display for Pages {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "{} pages", self.0)
///     }
/// }
///
/// impl Shown for Pages {
///     fn json<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
///         serializer.serialize_u32(self.0)
///     }
///
///     fn schema(generator: &mut SchemaGenerator) -> Schema {
///         u32::json_schema(generator)
///     }
/// }
///
/// crumbkit::shown!(Pages);
///
/// assert_eq!(Pages(432).to_string(), "432 pages");
/// assert_eq!(serde_json::to_string(&Pages(432)).unwrap(), "432");
/// ```
///
/// The trait requires `Display`, because a value that reads one way in a
/// sentence and another in a table cell is two values, and a tool then
/// writes two types (ADR-0147).
pub trait Shown: fmt::Display {
    /// Writes the JSON form. The default writes the text form as a string.
    ///
    /// # Errors
    ///
    /// Returns the serializer's error.
    fn json<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }

    /// Returns the JSON Schema of the JSON form. The default is a string.
    fn schema(generator: &mut SchemaGenerator) -> Schema {
        <String as JsonSchema>::json_schema(generator)
    }

    /// Returns the name that a schema defines the JSON form under, or `None`
    /// to put the schema inline in each property that holds the value. The
    /// default is `None`, so a row's property holds the schema with the
    /// field's own description, and an `Option` of the value is the schema
    /// or null.
    #[must_use]
    fn schema_name() -> Option<Cow<'static, str>> {
        None
    }
}

/// Serializes `value` as its `Display` while crumbkit builds text cells on
/// this thread, and as [`Shown::json`] otherwise. The `Serialize` that
/// [`shown!`](crate::shown!) writes calls it, and a tool that writes its own
/// `Serialize` calls it the same way.
///
/// crumbkit sets the text mode on the current thread only, for the duration
/// of the serialization whose value becomes the cells of a text table, of a
/// stream's text line, of the table that [`Room`](crate::Room) measures, of
/// the labels of [`pick::from_rows`](crate::pick::from_rows), or of the
/// default child lines. A tool that serializes a value itself, with
/// `serde_json` or another serializer, always gets the JSON form.
///
/// The tool's own code that runs inside such a serialization sees the text
/// form of every shown value: a `serialize_with` function, a hand-written
/// `Serialize`, a `Display`, or a [`Row::is_problem`](crate::Row::is_problem)
/// override that serializes the row. crumbkit decides `is_problem` before
/// it sets the mode, so the default judges the JSON form.
///
/// # Errors
///
/// Returns the serializer's error.
pub fn serialize<T: Shown + ?Sized, S: Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if TextCells::active() {
        serializer.collect_str(value)
    } else {
        value.json(serializer)
    }
}

/// Writes `Serialize` and `JsonSchema` for a type that implements
/// [`Shown`](crate::Shown).
///
/// `Serialize` calls [`shown::serialize`](crate::shown::serialize), so the
/// value prints its `Display` in text and [`Shown::json`](crate::Shown::json)
/// in JSON. `JsonSchema` returns [`Shown::schema`](crate::Shown::schema),
/// inline when [`Shown::schema_name`](crate::Shown::schema_name) is `None`
/// and under that name otherwise. The schema's id is the type's module path
/// and the type as written, so two crates' types of one name do not share a
/// definition. A type with no schema name is inline, so the type as written,
/// which `JsonSchema::schema_name` returns for it, never appears in a
/// schema, and a path such as `units::Pages` is as good as `Pages`.
///
/// The macro call names serde and schemars through crumbkit, so a crate
/// needs neither dependency for the call itself. A type that overrides
/// [`Shown::json`](crate::Shown::json) names serde's `Serializer` through
/// `crumbkit::serde`, and an override of
/// [`Shown::schema`](crate::Shown::schema) names its types through
/// `crumbkit::schemars`.
///
/// The macro takes one concrete type, which may be a generic type with its
/// parameters filled in, such as `shown!(Count<u32>)`. A generic impl for
/// every parameter, `impl<T> Serialize for Count<T>`, is written by hand, as
/// the macro's source shows.
///
/// A value that a row flattens with `#[serde(flatten)]` must serialize as a
/// map in both forms, because serde flattens only maps and structs, so its
/// `Serialize` calls [`Shown::json`](crate::Shown::json) directly, as
/// [`Bytes`](crate::Bytes) does.
#[macro_export]
macro_rules! shown {
    ($type:ty) => {
        impl $crate::serde::Serialize for $type {
            fn serialize<S: $crate::serde::Serializer>(
                &self,
                serializer: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                $crate::shown::serialize(self, serializer)
            }
        }

        impl $crate::schemars::JsonSchema for $type {
            fn inline_schema() -> bool {
                <$type as $crate::Shown>::schema_name().is_none()
            }

            fn schema_name() -> ::std::borrow::Cow<'static, str> {
                <$type as $crate::Shown>::schema_name()
                    .unwrap_or(::std::borrow::Cow::Borrowed(::core::stringify!($type)))
            }

            fn schema_id() -> ::std::borrow::Cow<'static, str> {
                ::std::borrow::Cow::Borrowed(::core::concat!(
                    ::core::module_path!(),
                    "::",
                    ::core::stringify!($type)
                ))
            }

            fn json_schema(
                generator: &mut $crate::schemars::SchemaGenerator,
            ) -> $crate::schemars::Schema {
                <$type as $crate::Shown>::schema(generator)
            }
        }
    };
}

/// A shown value that serializes as its JSON form in either mode, which
/// [`Field::shown`](crate::Field::shown) reads.
pub(crate) struct JsonForm<'a, T: ?Sized>(pub(crate) &'a T);

impl<T: Shown + ?Sized> Serialize for JsonForm<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.json(serializer)
    }
}

thread_local! {
    /// Whether a shown value on this thread serializes as its `Display`,
    /// which [`TextCells`] sets while crumbkit builds text cells.
    static TEXT_CELLS: Cell<bool> = const { Cell::new(false) };
}

/// The text mode of this thread, which makes a shown value serialize as its
/// `Display` until the guard is dropped. Dropping it restores the mode that
/// [`TextCells::enter`] found, so a guard inside another leaves the outer
/// one in effect, and a panic during the serialization ends the mode too.
///
/// crumbkit holds one only around the serialization of rows whose value
/// becomes the cells of a text table, of a stream's text line, or the
/// default child lines of a row (ADR-0147).
#[must_use = "the text mode ends when the guard is dropped"]
pub(crate) struct TextCells {
    /// The mode that the thread had before this guard.
    previous: bool,
}

impl TextCells {
    /// Sets the text mode on this thread until the guard is dropped.
    pub(crate) fn enter() -> Self {
        let previous = TEXT_CELLS
            .try_with(|text| text.replace(true))
            .unwrap_or(false);
        Self { previous }
    }

    /// Reports whether this thread is building text cells.
    pub(crate) fn active() -> bool {
        TEXT_CELLS.try_with(Cell::get).unwrap_or(false)
    }
}

impl Drop for TextCells {
    fn drop(&mut self) {
        // The thread-local holds a `bool`, so it has no destructor and the
        // access fails only while the thread itself is being torn down.
        let _ = TEXT_CELLS.try_with(|text| text.set(self.previous));
    }
}
