// The crate docs are the README, which links to files in the repository. The
// docs of each derive below are the reference for its attributes (ADR-0013).
#![doc = include_str!("../README.md")]

mod expand;
mod parse;
mod row;
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
/// | `no_next_step = "..."` | variant or struct | A `Recovery::None` with this reason, in place of `recovery`. Text that holds `{` goes through `format!`, as `hint` does. |
/// | `alternative = EXPR` | variant or struct | One `Recovery` that `alternatives()` returns. The key can repeat, and the list keeps the written order. |
/// | `causes = EXPR` | variant or struct | The `Vec<String>` that `causes()` returns, in order, one sentence each. The expression reads the variant's fields. A variant that forwards returns the causes of the failure it wraps. |
/// | `field(NAME = EXPR, ...)` | variant or struct | One `Field` for each name, in the written order. Each name must be one or more lowercase ASCII letters, digits and underscores. |
/// | `url = EXPR` | variant or struct | The `&str` that `url()` returns. |
/// | `sentence = "..."` | variant or struct | The fixed sentence that `sentence()` returns. It is one line of text with no `{`, so no value from the run reaches it. |
/// | `diagnostic` | variant or struct | `diagnostic()` returns the failure itself, which must implement `miette::Diagnostic`. |
/// | `forward` | variant or struct with one field | Every method except `diagnostic()` calls the same method on that field, which must implement `Failure`. `diagnostic()` forwards only with the `diagnostic` key. |
/// | `forward = FIELD` | variant or struct with named fields | Every method except `diagnostic()` calls the same method on the named field, as `forward` does. |
/// | `reviewed` | type | Every variant that does not forward, or the struct, needs `sentence`, and a missing one is a compile error that names the variant. |
/// | `crate = PATH` | type | The path of the crate that holds `Failure`. It is `::crumbkit` unless given, and a crate that reaches crumbkit through a re-export gives that path. |
///
/// Each variant that does not forward needs `code` and exactly one of
/// `recovery`, `hint` and `no_next_step`. A type marked `reviewed` also needs
/// `sentence` on each such variant. A variant that forwards is not checked,
/// so a forwarded type is checked only when it is marked `reviewed` itself.
/// The check covers a type that derives
/// `Failure`, and a failure written by hand or a code from `Code::runtime`
/// escapes it. A forwarding variant takes no other
/// key except `diagnostic`, which forwards `diagnostic()` too. A key given
/// twice, an unknown key and a missing key are compile errors that name the
/// variant.
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
///     fn code(&self) -> ::crumbkit::Code<'_> {
///         match self {
///             Self::NotFound { .. } => NOT_FOUND,
///             Self::RemovedFile { .. } => REMOVED_FILE,
///             Self::NoStateDirectory => NO_STATE_DIRECTORY,
///             Self::Book(f0) => ::crumbkit::Failure::code(f0),
///         }
///     }
///
///     fn causes(&self) -> ::std::vec::Vec<::std::string::String> {
///         match self {
///             Self::NotFound { .. } => ::std::vec::Vec::new(),
///             Self::RemovedFile { .. } => ::std::vec::Vec::new(),
///             Self::NoStateDirectory => ::std::vec::Vec::new(),
///             Self::Book(f0) => ::crumbkit::Failure::causes(f0),
///         }
///     }
///
///     fn recovery(&self) -> ::crumbkit::Recovery {
///         match self {
///             Self::NotFound { ids } => Recovery::Run(Call::new(&["books", "list"])),
///             Self::RemovedFile { name, error } => ::crumbkit::Recovery::hint(
///                 ::std::format!("make {name} a readable file")
///             ),
///             Self::NoStateDirectory => ::crumbkit::Recovery::hint(
///                 "set HOME to a directory"
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
///     fn sentence(&self) -> ::std::option::Option<&'static str> {
///         match self {
///             Self::NotFound { .. } => ::std::option::Option::None,
///             Self::RemovedFile { .. } => ::std::option::Option::None,
///             Self::NoStateDirectory => ::std::option::Option::None,
///             Self::Book(f0) => ::crumbkit::Failure::sentence(f0),
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
///     pub(crate) const CODES: &'static [::crumbkit::Code<'static>] =
///         &[NOT_FOUND, REMOVED_FILE, NO_STATE_DIRECTORY];
/// }
/// ```
///
/// The derive leaves out `causes`, `alternatives`, `fields`, `url` and `sentence` when no variant
/// gives the key and no variant forwards, so the trait's default applies. It
/// writes `diagnostic` only when a variant gives `diagnostic`, so a type that
/// gives none builds without the `miette` feature. The generated
/// `diagnostic` names `::miette`, so a type that gives `diagnostic` needs
/// miette as its own dependency.
///
/// `CODES` has the visibility of the type. It lists each code once, in the
/// order of the variants, and two codes count as the same when their paths
/// are written the same. It leaves out the codes of forwarding variants,
/// because the wrapped type has its own `CODES` list that the twin and
/// recovery checks read.
#[proc_macro_derive(Failure, attributes(failure))]
pub fn derive_failure(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    expand::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Writes the `Row` impl of a struct with named fields from `#[row(...)]`
/// attributes on its fields.
///
/// # Keys
///
/// | Key | Where | What it gives |
/// | --- | --- | --- |
/// | `column` | field | The field's name in `COLUMNS`. |
/// | `long` | field | The field's name in `LONG_COLUMNS`. A field gives `column` or `long`, and never both. |
/// | `uncut` | field that gives `column` or `long` | The field's name in `UNCUT_COLUMNS` as well. |
/// | `problem` | field that gives `column` | The field's name in `PROBLEM`, as `Problem::field`. At most one field gives `problem` or `problem_by`. |
/// | `problem_by = EXPR` | field that gives `column` | The field's name in `PROBLEM`, as `Problem::by`, with `EXPR` deciding which rows report a problem. |
/// | `warning` | field that gives `column` | The field's name in `WARNING`, as `Warning::field`. At most one field gives `warning` or `warning_by`, and never the field that gives `problem` or `problem_by`. |
/// | `warning_by = EXPR` | field that gives `column` | The field's name in `WARNING`, as `Warning::by`, with `EXPR` deciding which rows report a warning. |
/// | `children` or `children(...)` | field | The field in `CHILDREN`, as `Children::field`. The options `indent = N`, `spaced`, `help = "..."`, `lines = EXPR` and `long_lines = EXPR` call the `Children` builder of the same name. At most one field gives it, and the field gives no other key. |
/// | `flatten` | field marked `#[serde(flatten)]` | The columns of the field's type, spliced into each list at the field's position. The field gives no other key. |
/// | `whole` | type | `WHOLE` is true. |
/// | `remote = TYPE` | type | The impl is for `TYPE`, which the struct mirrors, in place of the struct. |
/// | `crate = PATH` | type | The path of the crate that holds `Row`, `Problem`, `Warning` and `Children`. It is `::crumbkit` unless given, and a crate that reaches crumbkit through a re-export gives that path. |
///
/// `EXPR` is a path or a closure that does not capture, which coerces to the
/// function pointer that the constructor takes, and `Self` in it names the
/// type that gets the impl.
///
/// Each list keeps the order of the fields in the struct, so a type
/// reorders its columns by reordering its fields. Each name is the field's
/// name as serde serializes it. The derive reads the field's
/// `#[serde(rename = "...")]` or `#[serde(rename(serialize = "..."))]`, and
/// otherwise applies the type's `#[serde(rename_all = "...")]` or
/// `#[serde(rename_all(serialize = "..."))]` rule to the field's Rust name.
/// It reads no other serde key, because no other key changes the name.
///
/// A field without a key is in no list, so it appears only with
/// `--json --full`. A struct whose fields give no `column` still gets an
/// impl, and `finish` and `stream` then fail to compile for it, as they do
/// for a hand-written impl with an empty `COLUMNS`.
///
/// The derive writes `COLUMNS`, `LONG_COLUMNS`, `UNCUT_COLUMNS`, `PROBLEM`
/// and `WARNING`, and `CHILDREN` and `WHOLE` when a key sets them, and leaves
/// every other item of `Row` to its default. A row that sets `LINE` or
/// overrides `cell_style` or `in_default_text` writes `impl Row` by hand. So
/// does an enum, and a row whose `Serialize` impl is written by hand, because
/// the derive reads the names from serde's attributes.
///
/// The impl copies the type's generic parameters and its where-clause, so
/// the type's own bounds make it `Serialize` and `JsonSchema`.
///
/// # What it writes
///
/// For this struct:
///
/// ```rust,ignore
/// #[derive(Serialize, JsonSchema, Row)]
/// #[serde(rename_all = "camelCase")]
/// struct Book {
///     #[row(column, uncut)]
///     id: u32,
///     #[row(column)]
///     #[serde(rename = "name")]
///     title: String,
///     #[row(long)]
///     page_count: u32,
///     #[row(column, problem)]
///     damage: Option<String>,
///     #[row(column, warning)]
///     wear: Option<String>,
///     isbn: String,
/// }
/// ```
///
/// the derive writes this code, which a unit test of this crate compares
/// with its output:
///
/// ```rust,ignore
/// #[automatically_derived]
/// impl ::crumbkit::Row for Book {
///     const COLUMNS: &'static [&'static str] = &["id", "name", "damage", "wear"];
///     const LONG_COLUMNS: &'static [&'static str] = &["pageCount"];
///     const UNCUT_COLUMNS: &'static [&'static str] = &["id"];
///     const PROBLEM: ::std::option::Option<::crumbkit::Problem<Self>> =
///         ::std::option::Option::Some(::crumbkit::Problem::field("damage"));
///     const WARNING: ::std::option::Option<::crumbkit::Warning<Self>> =
///         ::std::option::Option::Some(::crumbkit::Warning::field("wear"));
/// }
/// ```
///
/// # Flattened fields
///
/// `#[row(flatten)]` on a field marked `#[serde(flatten)]` takes the
/// columns of the field's type, which must implement `Row`, usually by
/// deriving it. For
///
/// ```rust,ignore
/// #[derive(Serialize, JsonSchema, Row)]
/// struct File {
///     #[row(column)]
///     path: String,
///     #[serde(flatten)]
///     #[row(flatten)]
///     stats: Stats,
/// }
/// ```
///
/// the derive writes each list as a constant that joins the names and the
/// field type's list, and an assertion for each item that the row would
/// drop:
///
/// ```rust,ignore
/// const _: () = {
///     const __COLUMNS: [&'static str; 1 + <Stats as ::crumbkit::Row>::COLUMNS.len()] =
///         ::crumbkit::__row_columns(&[&["path"], <Stats as ::crumbkit::Row>::COLUMNS]);
///     // __LONG_COLUMNS and __UNCUT_COLUMNS follow in the same way.
///     const _: () = {
///         ::std::assert!(<Stats as ::crumbkit::Row>::PROBLEM.is_none(), "...");
///         // The same for WARNING and CHILDREN.
///     };
///     impl ::crumbkit::Row for File {
///         const COLUMNS: &'static [&'static str] = &__COLUMNS;
///         // ...
///     }
/// };
/// ```
///
/// The build fails when the flattened type sets `PROBLEM`, `WARNING` or
/// `CHILDREN`, because the outer row would drop the setting, and set it on
/// the outer row. A flattened type that does not implement `Row` fails with
/// the compiler's missing-trait error at the generated path.
///
/// # Rows for a foreign type
///
/// `#[row(remote = TYPE)]` follows serde's `remote`. A local struct, the
/// mirror, lists the fields of a type that another crate owns, with the
/// `#[row(...)]` keys and the `#[serde(rename)]` and `#[serde(rename_all)]`
/// attributes that the foreign type's JSON has, and the derive writes
/// `impl Row for TYPE` from it. A derive reads only the type that carries
/// it, so the mirror stands in for fields that the derive cannot see, such as
/// the fields of a type from another crate behind a local newtype like
/// `Printed<RunReport>`. One mirror serves one instance of a generic type.
///
/// ```rust,ignore
/// #[derive(Row)]
/// #[row(remote = Printed<RunReport>)]
/// #[serde(rename_all = "camelCase")]
/// struct PrintedRunReport {
///     #[row(column)]
///     run_name: String,
///     #[row(column, problem_by = |row: &Self| row.0.failed)]
///     failed: bool,
/// }
/// ```
///
/// The mirror derives nothing else, so `serde` is a helper attribute of this
/// derive. The derive reads the mirror's field names and attributes and
/// never its field types, and it writes a constant function that builds the
/// mirror from itself so the unread fields draw no `dead_code` warning. Nothing ties the
/// mirror to the foreign type at compile time: crumbkit-test's `check_help`
/// fails a column that is not a field of the leaf's schema, and only for a
/// leaf that the tool record gives.
///
/// # Refused uses
///
/// Each of these is a compile error that names the field or the type: an
/// unknown key, a key given twice, `column` with `long`, `uncut` without
/// either, `problem` or `warning` without `column`, `problem_by` or
/// `warning_by` without `column`, `problem` with `problem_by` and `warning`
/// with `warning_by` on one field, `problem` or `warning` on a second field,
/// `problem` and `warning` on one field, `children` with `column`, `long`,
/// `uncut`, `problem` or `warning` or on a second field, an unknown or
/// repeated `children` option, `flatten` on a field without
/// `#[serde(flatten)]` or with any other key, `flatten` or `remote` on a
/// generic type, `whole`, `remote` or `crate` on a field, and a field key on
/// the type. Any other key on a field marked `#[serde(skip)]`,
/// `#[serde(skip_serializing)]` or `#[serde(flatten)]` is an error as well.
/// The derive also refuses a type marked `#[serde(transparent)]` unless it
/// gives `remote`, an enum, a union, a tuple struct and a unit struct.
#[proc_macro_derive(Row, attributes(row, serde))]
pub fn derive_row(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    row::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
