//! Reads the `#[serde(...)]` attributes that change a field's name in the
//! JSON.

use proc_macro2::TokenTree;
use syn::meta::ParseNestedMeta;
use syn::{Attribute, Expr, Ident, LitStr, Token};

/// What a field's `#[serde(...)]` attributes say about its name in the JSON.
#[derive(Default)]
pub(super) struct SerdeField {
    /// The name that `rename = "..."` or `rename(serialize = "...")` gives.
    pub(super) rename: Option<String>,
    /// `skip`, `skip_serializing` or `flatten`, the first of these keys that
    /// the field gives, any of which leaves no field of its name in the JSON.
    pub(super) dropped: Option<&'static str>,
    /// Whether the field gives `flatten`, which a `#[row(flatten)]` key
    /// needs.
    pub(super) flatten: bool,
}

impl SerdeField {
    /// Reads the `#[serde(...)]` attributes of one field, passing over every
    /// key that does not change the field's serialized name.
    pub(super) fn read(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut field = Self::default();
        for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    if let Some(name) = serialize_value(&meta)? {
                        field.rename = Some(name.value());
                    }
                    return Ok(());
                }
                field.flatten |= meta.path.is_ident("flatten");
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
/// no field names, unless the type is a mirror of a foreign type (`remote`),
/// whose own JSON nothing prints.
pub(super) fn rename_all(
    attrs: &[Attribute],
    name: &Ident,
    remote: bool,
) -> syn::Result<RenameAll> {
    let mut rule = RenameAll::None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("transparent") && !remote {
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
pub(super) enum RenameAll {
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
    pub(super) fn apply(self, field: &str) -> String {
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
