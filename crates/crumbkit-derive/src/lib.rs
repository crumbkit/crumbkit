// The crate docs are the README, which links to files in the repository. The
// derive's own docs below are the reference for its attributes (ADR-0013).
#![doc = include_str!("../README.md")]

mod expand;
mod parse;
#[cfg(test)]
mod tests;

/// Writes the `Failure` impl and the `CODES` list of an enum or a struct
/// from its `#[failure(...)]` attributes.
///
/// On an enum, each variant has its own attribute. On a struct, the
/// attribute on the type gives the keys that a variant would give. The type
/// has no generic parameters, because `Failure` is used as a `'static` trait
/// object.
///
/// # Keys
///
/// | Key | Where | What it gives |
/// | --- | --- | --- |
/// | `code = PATH` | variant or struct | The code that `code()` returns, as a path to a `Code` constant. Every variant that does not forward needs it. |
/// | `recovery = EXPR` | variant or struct | The `Recovery` that `recovery()` returns. |
/// | `hint = "..."` | variant or struct | A `Recovery::Hint` with this text, in place of `recovery`. Text that holds `{` goes through `format!`, so `{name}` reads the field `name` or any constant in scope. |
/// | `alternative = EXPR` | variant or struct | One `Recovery` that `alternatives()` returns. The key can repeat, and the list keeps the written order. |
/// | `field(NAME = EXPR, ...)` | variant or struct | One `Field` for each name, in the written order. Each name must follow the rule of a code segment: one or more lowercase ASCII letters, digits and underscores. |
/// | `url = EXPR` | variant or struct | The `&str` that `url()` returns. |
/// | `diagnostic` | variant or struct | `diagnostic()` returns the failure itself, which must implement `miette::Diagnostic`. |
/// | `forward` | variant or struct with one field | Every method calls the same method on that field, which must implement `Failure`. |
/// | `forward = FIELD` | variant or struct with named fields | Every method calls the same method on the named field. |
/// | `crate = PATH` | type | The path of the crate that holds `Failure`. It is `::crumbkit` unless given, and a crate that reaches crumbkit through a re-export gives that path. |
///
/// Each variant that does not forward needs `code` and exactly one of
/// `recovery` and `hint`. A forwarding variant takes no other key except
/// `diagnostic`, which forwards `diagnostic()` too. A key given twice, an
/// unknown key and a missing key are compile errors that name the variant.
///
/// The expressions in `recovery`, `alternative`, `field` and `url` read the
/// variant's fields by name, and the fields of a tuple variant as `f0`, `f1`
/// and so on. Each field is a reference, as in a `match` on `&self`. An
/// expression can also call methods on `self`.
///
/// # What it writes
///
/// For this enum:
///
/// ```rust,ignore
/// #[derive(Debug, Failure)]
/// pub(crate) enum BooksError {
///     #[failure(
///         code = NOT_FOUND,
///         recovery = Recovery::Run(Call::new(&["books", "list"])),
///         field(books = FieldValue::list(ids.iter().copied())),
///     )]
///     NotFound { ids: Vec<u32> },
///     #[failure(
///         code = REMOVED_FILE,
///         hint = "make {name} a readable file",
///         alternative = Recovery::Retry,
///         url = "https://example.org/removed",
///         diagnostic,
///     )]
///     RemovedFile { name: String, error: io::Error },
///     #[failure(code = NO_STATE_DIRECTORY, hint = "set HOME to a directory")]
///     NoStateDirectory,
///     #[failure(forward)]
///     Book(NoSuchBook),
/// }
/// ```
///
/// the derive writes this code, which a unit test of this crate compares
/// with its output:
///
/// ```rust,ignore
/// impl ::crumbkit::Failure for BooksError {
///     fn code(&self) -> &'static ::crumbkit::Code {
///         match self {
///             Self::NotFound { .. } => &NOT_FOUND,
///             Self::RemovedFile { .. } => &REMOVED_FILE,
///             Self::NoStateDirectory => &NO_STATE_DIRECTORY,
///             Self::Book(f0) => ::crumbkit::Failure::code(f0),
///         }
///     }
///
///     fn recovery(&self) -> ::crumbkit::Recovery {
///         match self {
///             Self::NotFound { ids } => Recovery::Run(Call::new(&["books", "list"])),
///             Self::RemovedFile { name, error } => ::crumbkit::Recovery::Hint(
///                 ::std::format!("make {name} a readable file")
///             ),
///             Self::NoStateDirectory => ::crumbkit::Recovery::Hint(
///                 ::std::string::String::from("set HOME to a directory")
///             ),
///             Self::Book(f0) => ::crumbkit::Failure::recovery(f0),
///         }
///     }
///
///     fn alternatives(&self) -> ::std::vec::Vec<::crumbkit::Recovery> {
///         match self {
///             Self::NotFound { .. } => ::std::vec::Vec::new(),
///             Self::RemovedFile { name, error } => ::std::vec![Recovery::Retry],
///             Self::NoStateDirectory => ::std::vec::Vec::new(),
///             Self::Book(f0) => ::crumbkit::Failure::alternatives(f0),
///         }
///     }
///
///     fn fields(&self) -> ::std::vec::Vec<::crumbkit::Field<'_>> {
///         match self {
///             Self::NotFound { ids } => ::std::vec![
///                 ::crumbkit::Field::new("books", FieldValue::list(ids.iter().copied()))
///             ],
///             Self::RemovedFile { .. } => ::std::vec::Vec::new(),
///             Self::NoStateDirectory => ::std::vec::Vec::new(),
///             Self::Book(f0) => ::crumbkit::Failure::fields(f0),
///         }
///     }
///
///     fn url(&self) -> ::std::option::Option<&str> {
///         match self {
///             Self::NotFound { .. } => ::std::option::Option::None,
///             Self::RemovedFile { name, error } => {
///                 ::std::option::Option::Some("https://example.org/removed")
///             }
///             Self::NoStateDirectory => ::std::option::Option::None,
///             Self::Book(f0) => ::crumbkit::Failure::url(f0),
///         }
///     }
///
///     fn diagnostic(&self) -> ::std::option::Option<&dyn ::miette::Diagnostic> {
///         match self {
///             Self::NotFound { .. } => ::std::option::Option::None,
///             Self::RemovedFile { .. } => ::std::option::Option::Some(self),
///             Self::NoStateDirectory => ::std::option::Option::None,
///             Self::Book(..) => ::std::option::Option::None,
///         }
///     }
/// }
///
/// impl BooksError {
///     /// Every code that a `BooksError` returns, in the order of its
///     /// variants. ...
///     pub(crate) const CODES: &'static [&'static ::crumbkit::Code] =
///         &[&NOT_FOUND, &REMOVED_FILE, &NO_STATE_DIRECTORY];
/// }
/// ```
///
/// The derive leaves out `alternatives`, `fields` and `url` when no variant
/// gives the key and no variant forwards, so the trait's default applies. It
/// writes `diagnostic` only when a variant gives `diagnostic`, so a type that
/// gives none builds without the `miette` feature. The generated
/// `diagnostic` names `::miette`, so a type that gives `diagnostic` needs
/// miette as its own dependency.
///
/// `CODES` has the visibility of the type. It lists each code once, in the
/// order of the variants, and two codes count as the same when their paths
/// are written the same. It leaves out the codes of forwarding variants,
/// because the wrapped type has its own `CODES` list that the codes check
/// reads.
#[proc_macro_derive(Failure, attributes(failure))]
pub fn derive_failure(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    expand::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
