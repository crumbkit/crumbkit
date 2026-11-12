//! Reads a struct that derives `Row`, with its `#[row(...)]` and
//! `#[serde(...)]` attributes, and writes its `impl Row`.
//!
//! Each name in the impl is the field's name as serde serializes it, so a
//! column always names a field of the row's JSON.

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::ext::IdentExt;
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;
use syn::{Attribute, Data, DeriveInput, Expr, Field, Fields, Ident, LitStr, Path, Token};

#[cfg(test)]
mod tests;

/// The keys that `#[row(...)]` accepts, as the message about an unknown key
/// lists them.
const KEYS: &str = "`column`, `long`, `uncut` and `problem` on a field, and `crate` on the type";

/// The keys that `#[row(...)]` accepts on a field.
const FIELD_KEYS: [&str; 4] = ["column", "long", "uncut", "problem"];

/// Returns the code that `#[derive(Row)]` writes for `input`, or the compile
/// error that names what the input gets wrong.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let row = read(input)?;
    Ok(write(input, &row))
}

/// What the generated impl says about a row type.
struct RowType {
    /// The path of the crate that holds `Row`, `::crumbkit` unless the
    /// type's attribute names another with `crate = ...`.
    krate: Path,
    /// The names in `COLUMNS`, in field order.
    columns: Vec<String>,
    /// The names in `LONG_COLUMNS`, in field order.
    long: Vec<String>,
    /// The names in `UNCUT_COLUMNS`, in field order.
    uncut: Vec<String>,
    /// The field that gives `problem` and its name, which `PROBLEM` holds.
    problem: Option<(Ident, String)>,
}

/// Reads `input` into a [`RowType`], or returns the compile error that
/// names what the input gets wrong.
fn read(input: &DeriveInput) -> syn::Result<RowType> {
    let name = &input.ident;
    let refuse = |kind: &str| {
        Err(syn::Error::new_spanned(
            name,
            format!("#[derive(Row)] supports a struct with named fields, and `{name}` is {kind}"),
        ))
    };
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            Fields::Unnamed(_) => return refuse("a tuple struct"),
            Fields::Unit => return refuse("a unit struct"),
        },
        Data::Enum(_) => return refuse("an enum, so it writes `impl Row` by hand"),
        Data::Union(_) => return refuse("a union"),
    };
    let mut row = RowType {
        krate: type_crate(&input.attrs)?,
        columns: Vec::new(),
        long: Vec::new(),
        uncut: Vec::new(),
        problem: None,
    };
    let rule = rename_all(&input.attrs, name)?;
    // Every field of `Fields::Named` has a name.
    for (field, ident) in fields
        .iter()
        .filter_map(|field| Some((field, field.ident.as_ref()?)))
    {
        read_field(&mut row, field, ident, rule)?;
    }
    Ok(row)
}

/// Reads the `#[row(...)]` attributes on the type, which take only
/// `crate = PATH`, and returns the crate's path.
fn type_crate(attrs: &[Attribute]) -> syn::Result<Path> {
    let mut krate: Option<Path> = None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("row")) {
        attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident("crate") {
                let key = meta.path.to_token_stream().to_string();
                return Err(meta.error(if FIELD_KEYS.contains(&key.as_str()) {
                    format!("`{key}` goes in the #[row(...)] attribute of a field")
                } else {
                    format!(
                        "the type gives `{key}`, and #[row(...)] has no such key. Its keys are {KEYS}"
                    )
                }));
            }
            if krate.is_some() {
                return Err(meta.error("the type gives `crate` twice"));
            }
            krate = Some(meta.value()?.parse()?);
            Ok(())
        })?;
    }
    Ok(krate.unwrap_or_else(|| syn::parse_quote!(::crumbkit)))
}

/// Adds `field` to the lists that its `#[row(...)]` keys name.
fn read_field(row: &mut RowType, field: &Field, ident: &Ident, rule: RenameAll) -> syn::Result<()> {
    let keys = Keys::read(&field.attrs, ident)?;
    let Some(span) = keys.first() else {
        return Ok(());
    };
    let serde = SerdeField::read(&field.attrs)?;
    if let Some(dropped) = serde.dropped {
        return Err(syn::Error::new(
            span,
            format!(
                "the field `{ident}` is marked #[serde({dropped})], so the JSON holds no field of that name, and it takes no #[row(...)] key"
            ),
        ));
    }
    let name = serde
        .rename
        .unwrap_or_else(|| rule.apply(&ident.unraw().to_string()));
    match (keys.column, keys.long) {
        (Some(_), Some(long)) => {
            return Err(syn::Error::new(
                long,
                format!(
                    "the field `{ident}` gives both `column` and `long`, and a field is in only one of the two lists"
                ),
            ));
        }
        (Some(_), None) => row.columns.push(name.clone()),
        (None, Some(_)) => row.long.push(name.clone()),
        (None, None) => {
            if let Some(uncut) = keys.uncut {
                return Err(syn::Error::new(
                    uncut,
                    format!(
                        "the field `{ident}` gives `uncut` without `column` or `long`, and only a column can be uncut"
                    ),
                ));
            }
        }
    }
    if keys.uncut.is_some() {
        row.uncut.push(name.clone());
    }
    if let Some(problem) = keys.problem {
        if keys.column.is_none() {
            return Err(syn::Error::new(
                problem,
                format!(
                    "the field `{ident}` gives `problem` without `column`, and the problem field is one of the compact columns"
                ),
            ));
        }
        if let Some((first, _)) = &row.problem {
            return Err(syn::Error::new(
                problem,
                format!(
                    "the fields `{first}` and `{ident}` both give `problem`, and a row has at most one problem field"
                ),
            ));
        }
        row.problem = Some((ident.clone(), name));
    }
    Ok(())
}

/// The `#[row(...)]` keys on one field, each field named after its key and
/// holding the span where the key was written.
#[derive(Default)]
struct Keys {
    column: Option<Span>,
    long: Option<Span>,
    uncut: Option<Span>,
    problem: Option<Span>,
}

impl Keys {
    /// Reads every `#[row(...)]` attribute on the field `ident`.
    fn read(attrs: &[Attribute], ident: &Ident) -> syn::Result<Self> {
        let mut keys = Self::default();
        for attr in attrs.iter().filter(|attr| attr.path().is_ident("row")) {
            attr.parse_nested_meta(|meta| keys.read_key(&meta, ident))?;
        }
        Ok(keys)
    }

    /// Reads one key.
    fn read_key(&mut self, meta: &ParseNestedMeta<'_>, ident: &Ident) -> syn::Result<()> {
        let key = meta.path.to_token_stream().to_string();
        let slot = match key.as_str() {
            "column" => &mut self.column,
            "long" => &mut self.long,
            "uncut" => &mut self.uncut,
            "problem" => &mut self.problem,
            "crate" => {
                return Err(meta.error("`crate` goes in the #[row(...)] attribute of the type"));
            }
            _ => {
                return Err(meta.error(format!(
                    "the field `{ident}` gives `{key}`, and #[row(...)] has no such key. Its keys are {KEYS}"
                )));
            }
        };
        if slot.is_some() {
            return Err(meta.error(format!("the field `{ident}` gives `{key}` twice")));
        }
        *slot = Some(meta.path.span());
        Ok(())
    }

    /// Returns the span of the first key the field gives, or `None` when it
    /// gives none.
    fn first(&self) -> Option<Span> {
        self.column.or(self.long).or(self.uncut).or(self.problem)
    }
}

/// What a field's `#[serde(...)]` attributes say about its name in the JSON.
#[derive(Default)]
struct SerdeField {
    /// The name that `rename = "..."` or `rename(serialize = "...")` gives.
    rename: Option<String>,
    /// `skip`, `skip_serializing` or `flatten`, the first of these keys that
    /// the field gives, any of which leaves no field of its name in the JSON.
    dropped: Option<&'static str>,
}

impl SerdeField {
    /// Reads the `#[serde(...)]` attributes of one field, passing over every
    /// key that does not change the field's serialized name.
    fn read(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut field = Self::default();
        for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    if let Some(name) = serialize_value(&meta)? {
                        field.rename = Some(name.value());
                    }
                    return Ok(());
                }
                for dropped in ["skip", "skip_serializing", "flatten"] {
                    if meta.path.is_ident(dropped) {
                        field.dropped = field.dropped.or(Some(dropped));
                    }
                }
                skip_value(&meta)
            })?;
        }
        Ok(field)
    }
}

/// Returns the type's `#[serde(rename_all = "...")]` rule for serializing,
/// or [`RenameAll::None`] when it gives none. It refuses a type marked
/// `#[serde(transparent)]`, whose JSON is the value of its one field and has
/// no field names.
fn rename_all(attrs: &[Attribute], name: &Ident) -> syn::Result<RenameAll> {
    let mut rule = RenameAll::None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("transparent") {
                return Err(meta.error(format!(
                    "the type `{name}` is marked #[serde(transparent)], so its JSON has no field names for #[derive(Row)] to list"
                )));
            }
            if !meta.path.is_ident("rename_all") {
                return skip_value(&meta);
            }
            if let Some(text) = serialize_value(&meta)? {
                rule = RenameAll::parse(&text)?;
            }
            Ok(())
        })?;
    }
    Ok(rule)
}

/// Returns the serializing value of a serde key written as `key = "..."` or
/// as `key(serialize = "...", deserialize = "...")`, or `None` when the key
/// gives a value for deserializing alone.
fn serialize_value(meta: &ParseNestedMeta<'_>) -> syn::Result<Option<LitStr>> {
    if meta.input.peek(Token![=]) {
        return Ok(Some(meta.value()?.parse()?));
    }
    let mut value = None;
    meta.parse_nested_meta(|inner| {
        if inner.path.is_ident("serialize") {
            value = Some(inner.value()?.parse()?);
            Ok(())
        } else {
            skip_value(&inner)
        }
    })?;
    Ok(value)
}

/// Reads past the value of a serde key that the derive does not use, which
/// is nothing, `= EXPR` or a parenthesised list.
fn skip_value(meta: &ParseNestedMeta<'_>) -> syn::Result<()> {
    if meta.input.peek(Token![=]) {
        meta.value()?.parse::<Expr>()?;
    } else if meta.input.peek(syn::token::Paren) {
        meta.input.parse::<TokenTree>()?;
    }
    Ok(())
}

/// A `#[serde(rename_all = "...")]` rule, which serde applies to the name of
/// each field that has no `rename` of its own.
///
/// RUST-10: `serde_derive` keeps its rules private, so this copies its
/// `apply_to_field` for the eight rules that serde 1 accepts. It misses any
/// rule that a later serde adds, and the derive refuses such a rule as
/// unknown. `serde_derive_internals` replaces this if the derive comes to read
/// more of serde's attributes, at the cost of a dependency.
#[derive(Clone, Copy)]
enum RenameAll {
    /// No rule, so each field keeps its Rust name.
    None,
    /// `lowercase`.
    Lower,
    /// `UPPERCASE`.
    Upper,
    /// `PascalCase`.
    Pascal,
    /// `camelCase`.
    Camel,
    /// `snake_case`.
    Snake,
    /// `SCREAMING_SNAKE_CASE`.
    ScreamingSnake,
    /// `kebab-case`.
    Kebab,
    /// `SCREAMING-KEBAB-CASE`.
    ScreamingKebab,
}

/// Each rule's spelling in serde's attribute, in the order serde lists them.
const RULES: [(&str, RenameAll); 8] = [
    ("lowercase", RenameAll::Lower),
    ("UPPERCASE", RenameAll::Upper),
    ("PascalCase", RenameAll::Pascal),
    ("camelCase", RenameAll::Camel),
    ("snake_case", RenameAll::Snake),
    ("SCREAMING_SNAKE_CASE", RenameAll::ScreamingSnake),
    ("kebab-case", RenameAll::Kebab),
    ("SCREAMING-KEBAB-CASE", RenameAll::ScreamingKebab),
];

impl RenameAll {
    /// Returns the rule that `text` spells, or the error that lists the
    /// rules.
    fn parse(text: &LitStr) -> syn::Result<Self> {
        let value = text.value();
        if let Some((_, rule)) = RULES.iter().find(|(spelling, _)| *spelling == value) {
            return Ok(*rule);
        }
        let mut spellings: Vec<String> = RULES
            .iter()
            .map(|(spelling, _)| format!("{spelling:?}"))
            .collect();
        let last = spellings.pop().unwrap_or_default();
        Err(syn::Error::new_spanned(
            text,
            format!(
                "#[serde(rename_all)] has no rule {value:?}. Its rules are {} and {last}",
                spellings.join(", ")
            ),
        ))
    }

    /// Returns the name that serde gives the field `field` under this rule.
    fn apply(self, field: &str) -> String {
        match self {
            Self::None | Self::Lower | Self::Snake => field.to_owned(),
            Self::Upper | Self::ScreamingSnake => field.to_ascii_uppercase(),
            Self::Kebab => field.replace('_', "-"),
            Self::ScreamingKebab => field.to_ascii_uppercase().replace('_', "-"),
            Self::Pascal => pascal(field),
            Self::Camel => {
                let pascal = pascal(field);
                let mut chars = pascal.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_ascii_lowercase().to_string() + chars.as_str()
                })
            }
        }
    }
}

/// Returns `field` in `PascalCase`: each underscore is dropped, and the letter
/// after it and the first letter are upper case.
fn pascal(field: &str) -> String {
    let mut pascal = String::with_capacity(field.len());
    let mut capitalize = true;
    for ch in field.chars() {
        if ch == '_' {
            capitalize = true;
        } else if capitalize {
            pascal.push(ch.to_ascii_uppercase());
            capitalize = false;
        } else {
            pascal.push(ch);
        }
    }
    pascal
}

/// Writes the `impl Row` of `input` from what `row` says about it.
fn write(input: &DeriveInput, row: &RowType) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let RowType {
        krate,
        columns,
        long,
        uncut,
        problem,
    } = row;
    let problem = problem.as_ref().map_or_else(
        || quote!(::std::option::Option::None),
        |(_, field)| quote!(::std::option::Option::Some(#krate::Problem::field(#field))),
    );
    quote! {
        #[automatically_derived]
        impl #impl_generics #krate::Row for #name #type_generics #where_clause {
            const COLUMNS: &'static [&'static str] = &[#(#columns),*];
            const LONG_COLUMNS: &'static [&'static str] = &[#(#long),*];
            const UNCUT_COLUMNS: &'static [&'static str] = &[#(#uncut),*];
            const PROBLEM: ::std::option::Option<#krate::Problem<Self>> = #problem;
        }
    }
}
