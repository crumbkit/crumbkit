//! Reads the `#[row(...)]` keys of a type and of its fields.

use proc_macro2::Span;
use quote::ToTokens;
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;
use syn::{Attribute, Ident, Path};

/// The keys that `#[row(...)]` accepts, as the message about an unknown key
/// lists them.
const KEYS: &str =
    "`column`, `long`, `uncut`, `problem` and `warning` on a field, and `crate` on the type";

/// The keys that `#[row(...)]` accepts on a field.
const FIELD_KEYS: [&str; 5] = ["column", "long", "uncut", "problem", "warning"];

/// Reads the `#[row(...)]` attributes on the type, which take only
/// `crate = PATH`, and returns the crate's path.
pub(super) fn type_crate(attrs: &[Attribute]) -> syn::Result<Path> {
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

/// The `#[row(...)]` keys on one field, each field named after its key and
/// holding the span where the key was written.
#[derive(Default)]
pub(super) struct Keys {
    pub(super) column: Option<Span>,
    pub(super) long: Option<Span>,
    pub(super) uncut: Option<Span>,
    pub(super) problem: Option<Span>,
    pub(super) warning: Option<Span>,
}

impl Keys {
    /// Reads every `#[row(...)]` attribute on the field `ident`.
    pub(super) fn read(attrs: &[Attribute], ident: &Ident) -> syn::Result<Self> {
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
            "warning" => &mut self.warning,
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
    pub(super) fn first(&self) -> Option<Span> {
        self.column
            .or(self.long)
            .or(self.uncut)
            .or(self.problem)
            .or(self.warning)
    }
}
