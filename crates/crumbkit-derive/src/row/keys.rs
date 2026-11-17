//! Reads the `#[row(...)]` keys of a type and of its fields.

use proc_macro2::Span;
use quote::ToTokens;
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;
use syn::{Attribute, Expr, Ident, LitInt, LitStr, Path};

/// The keys that `#[row(...)]` accepts, as the message about an unknown key
/// lists them.
const KEYS: &str = "`column`, `long`, `uncut`, `problem`, `problem_by`, `warning`, `warning_by` and `children` on a field, and `crate` and `whole` on the type";

/// The keys that `#[row(...)]` accepts on a field.
const FIELD_KEYS: [&str; 8] = [
    "column",
    "long",
    "uncut",
    "problem",
    "problem_by",
    "warning",
    "warning_by",
    "children",
];

/// The keys that `#[row(...)]` accepts on the type.
const TYPE_KEYS: [&str; 2] = ["crate", "whole"];

/// The options that `children(...)` accepts, as the message about an
/// unknown option lists them.
const CHILDREN_OPTIONS: &str = "`indent`, `spaced`, `help`, `lines` and `long_lines`";

/// What the `#[row(...)]` attributes on the type give.
pub(super) struct TypeKeys {
    /// The path of the crate that holds `Row`, `::crumbkit` unless the type
    /// gives `crate = PATH`.
    pub(super) krate: Path,
    /// Whether the type gives `whole`, which sets `WHOLE` to true.
    pub(super) whole: bool,
}

impl TypeKeys {
    /// Reads the `#[row(...)]` attributes on the type, which take `crate =
    /// PATH` and `whole`.
    pub(super) fn read(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut krate: Option<Path> = None;
        let mut whole = false;
        for attr in attrs.iter().filter(|attr| attr.path().is_ident("row")) {
            attr.parse_nested_meta(|meta| {
                let key = meta.path.to_token_stream().to_string();
                if !TYPE_KEYS.contains(&key.as_str()) {
                    return Err(meta.error(if FIELD_KEYS.contains(&key.as_str()) {
                        format!("`{key}` goes in the #[row(...)] attribute of a field")
                    } else {
                        format!(
                            "the type gives `{key}`, and #[row(...)] has no such key. Its keys are {KEYS}"
                        )
                    }));
                }
                let twice = || meta.error(format!("the type gives `{key}` twice"));
                if key == "whole" {
                    if whole {
                        return Err(twice());
                    }
                    whole = true;
                } else {
                    if krate.is_some() {
                        return Err(twice());
                    }
                    krate = Some(meta.value()?.parse()?);
                }
                Ok(())
            })?;
        }
        Ok(Self {
            krate: krate.unwrap_or_else(|| syn::parse_quote!(::crumbkit)),
            whole,
        })
    }
}

/// The options of one `children(...)` key. Each option left out keeps the
/// default of the `Children` it builds.
#[derive(Clone, Default)]
pub(super) struct ChildrenKey {
    /// `indent = N`, the spaces before each child line.
    pub(super) indent: Option<u8>,
    /// `spaced`, which puts a blank line between rows.
    pub(super) spaced: bool,
    /// `help = "..."`, the sentence that the leaf's help gives.
    pub(super) help: Option<LitStr>,
    /// `lines = EXPR`, the function that builds the child lines.
    pub(super) lines: Option<Expr>,
    /// `long_lines = EXPR`, the function that builds them for `--long`.
    pub(super) long_lines: Option<Expr>,
}

impl ChildrenKey {
    /// Reads the options in parentheses after `children`, when there are
    /// any.
    fn read(meta: &ParseNestedMeta<'_>, ident: &Ident) -> syn::Result<Self> {
        let mut key = Self::default();
        if !meta.input.peek(syn::token::Paren) {
            return Ok(key);
        }
        meta.parse_nested_meta(|option| {
            let name = option.path.to_token_stream().to_string();
            let twice = || {
                option.error(format!(
                    "the field `{ident}` gives the `children` option `{name}` twice"
                ))
            };
            match name.as_str() {
                "indent" if key.indent.is_some() => return Err(twice()),
                "indent" => {
                    let spaces: LitInt = option.value()?.parse()?;
                    key.indent = Some(spaces.base10_parse()?);
                }
                "spaced" if key.spaced => return Err(twice()),
                "spaced" => key.spaced = true,
                "help" if key.help.is_some() => return Err(twice()),
                "help" => key.help = Some(option.value()?.parse()?),
                "lines" if key.lines.is_some() => return Err(twice()),
                "lines" => key.lines = Some(option.value()?.parse()?),
                "long_lines" if key.long_lines.is_some() => return Err(twice()),
                "long_lines" => key.long_lines = Some(option.value()?.parse()?),
                _ => {
                    return Err(option.error(format!(
                        "the field `{ident}` gives the `children` option `{name}`, and `children` has no such option. Its options are {CHILDREN_OPTIONS}"
                    )));
                }
            }
            Ok(())
        })?;
        Ok(key)
    }
}

/// The `#[row(...)]` keys on one field, each field named after its key and
/// holding the span where the key was written.
#[derive(Default)]
pub(super) struct Keys {
    pub(super) column: Option<Span>,
    pub(super) long: Option<Span>,
    pub(super) uncut: Option<Span>,
    pub(super) problem: Option<Span>,
    /// `problem_by = EXPR`, with the expression.
    pub(super) problem_by: Option<(Span, Expr)>,
    pub(super) warning: Option<Span>,
    /// `warning_by = EXPR`, with the expression.
    pub(super) warning_by: Option<(Span, Expr)>,
    /// `children` or `children(...)`, with its options.
    pub(super) children: Option<(Span, ChildrenKey)>,
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
        let span = meta.path.span();
        let twice = || meta.error(format!("the field `{ident}` gives `{key}` twice"));
        match key.as_str() {
            "column" | "long" | "uncut" | "problem" | "warning" => {
                let slot = match key.as_str() {
                    "column" => &mut self.column,
                    "long" => &mut self.long,
                    "uncut" => &mut self.uncut,
                    "problem" => &mut self.problem,
                    _ => &mut self.warning,
                };
                if slot.is_some() {
                    return Err(twice());
                }
                *slot = Some(span);
            }
            "problem_by" | "warning_by" => {
                let slot = if key == "problem_by" {
                    &mut self.problem_by
                } else {
                    &mut self.warning_by
                };
                if slot.is_some() {
                    return Err(twice());
                }
                *slot = Some((span, meta.value()?.parse()?));
            }
            "children" => {
                if self.children.is_some() {
                    return Err(twice());
                }
                self.children = Some((span, ChildrenKey::read(meta, ident)?));
            }
            _ if TYPE_KEYS.contains(&key.as_str()) => {
                return Err(meta.error(format!(
                    "`{key}` goes in the #[row(...)] attribute of the type"
                )));
            }
            _ => {
                return Err(meta.error(format!(
                    "the field `{ident}` gives `{key}`, and #[row(...)] has no such key. Its keys are {KEYS}"
                )));
            }
        }
        Ok(())
    }

    /// Returns the span of the first key the field gives, or `None` when it
    /// gives none.
    pub(super) fn first(&self) -> Option<Span> {
        self.column
            .or(self.long)
            .or(self.uncut)
            .or(self.problem)
            .or(self.problem_by.as_ref().map(|(span, _)| *span))
            .or(self.warning)
            .or(self.warning_by.as_ref().map(|(span, _)| *span))
            .or(self.children.as_ref().map(|(span, _)| *span))
    }
}
