//! Reads a type that derives `Failure`, with its `#[failure(...)]`
//! attributes, into a [`FailureType`], which `expand` turns into code.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;
use syn::{Attribute, Data, DeriveInput, Expr, Fields, Ident, LitStr, Path, Visibility};

/// The keys that `#[failure(...)]` accepts, as the message about an unknown
/// key lists them.
const KEYS: &str = "`code`, `recovery`, `hint`, `alternative`, `field`, `url`, `diagnostic` and `forward`, and `crate` on the type";

/// A type that derives `Failure`, as its attributes describe it.
pub(crate) struct FailureType {
    /// The type's name.
    pub(crate) name: Ident,
    /// The type's visibility, which the generated `CODES` list takes.
    pub(crate) visibility: Visibility,
    /// The path of the crate that holds `Failure`, `::crumbkit` unless the
    /// type's attribute names another with `crate = ...`.
    pub(crate) krate: Path,
    /// One arm for each variant of an enum, or one arm for a struct.
    pub(crate) arms: Vec<Arm>,
}

/// One variant of an enum, or a whole struct, and what its failure answers.
pub(crate) struct Arm {
    /// `Self::NotFound` for a variant, or `Self` for a struct.
    pub(crate) path: TokenStream,
    /// The variant's fields, which a pattern binds.
    pub(crate) shape: Shape,
    /// The answers that the attribute gives.
    pub(crate) answers: Answers,
}

/// The fields of a variant or a struct.
pub(crate) enum Shape {
    /// No fields, as in `NoStateDirectory`.
    Unit,
    /// Named fields, as in `NotFound { ids }`. Each name has the span of the
    /// macro call, so a pattern that binds it and a user's expression that
    /// reads it resolve to the same variable.
    Named(Vec<Ident>),
    /// This many unnamed fields, as in `Book(NoSuchBook)`, which a pattern
    /// binds as `f0`, `f1` and so on.
    Unnamed(usize),
}

/// What a variant's failure answers.
pub(crate) enum Answers {
    /// The attribute gives the answers itself. They sit in a `Box`, because
    /// they are much larger than a forward.
    Own(Box<Own>),
    /// The variant returns the answers of the failure in `field`.
    Forward {
        /// The variable that the pattern binds to the wrapped failure.
        field: Ident,
        /// Whether `diagnostic()` returns the wrapped failure's diagnostic.
        diagnostic: bool,
    },
}

/// The answers that a variant's attribute gives itself.
pub(crate) struct Own {
    /// The path of the variant's `Code` constant.
    pub(crate) code: Path,
    /// The variant's recovery.
    pub(crate) recovery: RecoveryKey,
    /// The variant's alternatives, in the written order.
    pub(crate) alternatives: Vec<Expr>,
    /// The variant's fields as names and values, in the written order.
    pub(crate) fields: Vec<(String, Expr)>,
    /// The variant's url.
    pub(crate) url: Option<Expr>,
    /// Whether `diagnostic()` returns the failure itself.
    pub(crate) diagnostic: bool,
}

/// How a variant states its recovery.
pub(crate) enum RecoveryKey {
    /// `recovery = EXPR`, an expression of type `Recovery`.
    Expr(Expr),
    /// `hint = "text"`, the text of a `Recovery::Hint`.
    Hint(LitStr),
}

/// Reads `input` into a [`FailureType`], or returns the compile error that
/// names what the input gets wrong.
pub(crate) fn read(input: &DeriveInput) -> syn::Result<FailureType> {
    let name = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "#[derive(Failure)] does not support a type with generic parameters",
        ));
    }
    let type_keys = Keys::read(&input.attrs, &format!("the type `{name}`"))?;
    let krate = type_keys
        .krate
        .clone()
        .unwrap_or_else(|| syn::parse_quote!(::crumbkit));
    let arms = match &input.data {
        Data::Struct(data) => {
            let owner = format!("the type `{name}`");
            let shape = shape(&data.fields);
            let answers = type_keys.answers(&owner, &shape, name.span())?;
            vec![Arm {
                path: quote!(Self),
                shape,
                answers,
            }]
        }
        Data::Enum(data) => {
            if let Some((key, span)) = type_keys.given.iter().find(|(key, _)| key != "crate") {
                return Err(syn::Error::new(
                    *span,
                    format!(
                        "on an enum, `{key}` goes in the #[failure(...)] attribute of each variant"
                    ),
                ));
            }
            if data.variants.is_empty() {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("the enum `{name}` has no variants, so it has no failure to describe"),
                ));
            }
            let mut arms = Vec::new();
            for variant in &data.variants {
                let ident = &variant.ident;
                let owner = format!("the variant `{ident}`");
                let keys = Keys::read(&variant.attrs, &owner)?;
                if let Some((_, span)) = keys.given.iter().find(|(key, _)| key == "crate") {
                    return Err(syn::Error::new(
                        *span,
                        "`crate` goes in the #[failure(...)] attribute of the type",
                    ));
                }
                let shape = shape(&variant.fields);
                let answers = keys.answers(&owner, &shape, ident.span())?;
                arms.push(Arm {
                    path: quote!(Self::#ident),
                    shape,
                    answers,
                });
            }
            arms
        }
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                name,
                format!("#[derive(Failure)] supports enums and structs, and `{name}` is a union"),
            ));
        }
    };
    Ok(FailureType {
        name: name.clone(),
        visibility: input.vis.clone(),
        krate,
        arms,
    })
}

/// Returns the shape of `fields`, with each name moved to the span of the
/// macro call.
fn shape(fields: &Fields) -> Shape {
    match fields {
        Fields::Unit => Shape::Unit,
        Fields::Named(named) => Shape::Named(
            named
                .named
                .iter()
                .filter_map(|field| field.ident.as_ref())
                .map(|ident| {
                    let mut ident = ident.clone();
                    ident.set_span(Span::call_site());
                    ident
                })
                .collect(),
        ),
        Fields::Unnamed(unnamed) => Shape::Unnamed(unnamed.unnamed.len()),
    }
}

/// The keys of the `#[failure(...)]` attributes on one type or variant, as
/// they were written.
#[derive(Default)]
struct Keys {
    /// Each key in the written order, with its span, so a check can name the
    /// first key that does not belong.
    given: Vec<(String, Span)>,
    /// `crate = PATH`.
    krate: Option<Path>,
    /// `code = PATH`.
    code: Option<Path>,
    /// `recovery = EXPR`.
    recovery: Option<Expr>,
    /// `hint = "text"`.
    hint: Option<LitStr>,
    /// Each `alternative = EXPR`.
    alternatives: Vec<Expr>,
    /// Each `field(NAME = EXPR)`.
    fields: Vec<(String, Expr)>,
    /// `url = EXPR`.
    url: Option<Expr>,
    /// `diagnostic`.
    diagnostic: bool,
    /// `forward` or `forward = FIELD`.
    forward: Option<Forward>,
}

/// How a `forward` key names the field that holds the wrapped failure.
enum Forward {
    /// `forward`: the variant's only field.
    Only,
    /// `forward = FIELD`: the named field.
    Field(Ident),
}

impl Keys {
    /// Reads every `#[failure(...)]` attribute in `attrs`. `owner` names the
    /// type or variant in error messages, such as "the variant `NotFound`".
    fn read(attrs: &[Attribute], owner: &str) -> syn::Result<Self> {
        let mut keys = Self::default();
        for attr in attrs.iter().filter(|attr| attr.path().is_ident("failure")) {
            attr.parse_nested_meta(|meta| keys.read_key(&meta, owner))?;
        }
        Ok(keys)
    }

    /// Reads one key and its value.
    fn read_key(&mut self, meta: &ParseNestedMeta<'_>, owner: &str) -> syn::Result<()> {
        let key = meta.path.to_token_stream().to_string();
        let span = meta.path.span();
        let twice = || syn::Error::new_spanned(&meta.path, format!("{owner} gives `{key}` twice"));
        match key.as_str() {
            "crate" => set(&mut self.krate, meta.value()?.parse()?, twice)?,
            "code" => set(&mut self.code, meta.value()?.parse()?, twice)?,
            "recovery" => set(&mut self.recovery, meta.value()?.parse()?, twice)?,
            "hint" => set(&mut self.hint, meta.value()?.parse()?, twice)?,
            "url" => set(&mut self.url, meta.value()?.parse()?, twice)?,
            "alternative" => self.alternatives.push(meta.value()?.parse()?),
            "field" => meta.parse_nested_meta(|field| self.read_field(&field, owner))?,
            "diagnostic" => {
                if self.diagnostic {
                    return Err(twice());
                }
                self.diagnostic = true;
            }
            "forward" => {
                let field = if meta.input.peek(syn::Token![=]) {
                    Forward::Field(meta.value()?.parse()?)
                } else {
                    Forward::Only
                };
                set(&mut self.forward, field, twice)?;
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    &meta.path,
                    format!("#[failure(...)] has no key `{key}`. Its keys are {KEYS}"),
                ));
            }
        }
        self.given.push((key, span));
        Ok(())
    }

    /// Reads one `NAME = EXPR` of a `field(...)`, and refuses a name that
    /// breaks the rule of a code segment or repeats an earlier name.
    fn read_field(&mut self, field: &ParseNestedMeta<'_>, owner: &str) -> syn::Result<()> {
        let name = field.path.to_token_stream().to_string();
        let follows_rule = !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        if !follows_rule {
            return Err(syn::Error::new_spanned(
                &field.path,
                format!(
                    "the field name `{name}` breaks the rule of a code segment: it must be one or more lowercase ASCII letters, digits and underscores"
                ),
            ));
        }
        if self.fields.iter().any(|(seen, _)| *seen == name) {
            return Err(syn::Error::new_spanned(
                &field.path,
                format!("{owner} names the field `{name}` twice"),
            ));
        }
        let value = field.value()?.parse()?;
        self.fields.push((name, value));
        Ok(())
    }

    /// Checks the keys of one variant, or of a struct, and returns its
    /// answers. `span` is where an error about a missing key points.
    fn answers(self, owner: &str, shape: &Shape, span: Span) -> syn::Result<Answers> {
        if let Some(field) = self.forward {
            if self
                .given
                .iter()
                .any(|(key, _)| !matches!(key.as_str(), "forward" | "diagnostic" | "crate"))
            {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "{owner} forwards to the failure it wraps, so it takes no key beside `forward` except `diagnostic`"
                    ),
                ));
            }
            let field = forwarded_field(field, owner, shape, span)?;
            return Ok(Answers::Forward {
                field,
                diagnostic: self.diagnostic,
            });
        }
        let Some(code) = self.code else {
            return Err(syn::Error::new(
                span,
                format!("{owner} needs `code = ...` or `forward` in #[failure(...)]"),
            ));
        };
        let recovery = match (self.recovery, self.hint) {
            (Some(expr), None) => RecoveryKey::Expr(expr),
            (None, Some(hint)) => RecoveryKey::Hint(hint),
            (Some(_), Some(_)) => {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "{owner} gives both `recovery` and `hint`, and it may give only one of them"
                    ),
                ));
            }
            (None, None) => {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "{owner} needs `recovery = ...` or `hint = \"...\"` in #[failure(...)]"
                    ),
                ));
            }
        };
        Ok(Answers::Own(Box::new(Own {
            code,
            recovery,
            alternatives: self.alternatives,
            fields: self.fields,
            url: self.url,
            diagnostic: self.diagnostic,
        })))
    }
}

/// Sets `slot` to `value`, or returns the error that `twice` makes when the
/// key was already given.
fn set<T>(slot: &mut Option<T>, value: T, twice: impl Fn() -> syn::Error) -> syn::Result<()> {
    if slot.is_some() {
        return Err(twice());
    }
    *slot = Some(value);
    Ok(())
}

/// Returns the variable that a forwarding pattern binds: the named field, or
/// the only field when the attribute names none.
fn forwarded_field(forward: Forward, owner: &str, shape: &Shape, span: Span) -> syn::Result<Ident> {
    match (forward, shape) {
        (Forward::Field(name), Shape::Named(fields)) => fields
            .iter()
            .find(|field| **field == name)
            .cloned()
            .ok_or_else(|| {
                syn::Error::new_spanned(&name, format!("{owner} has no field named `{name}`"))
            }),
        (Forward::Field(name), _) => Err(syn::Error::new_spanned(
            &name,
            format!("{owner} has no field named `{name}`"),
        )),
        (Forward::Only, Shape::Named(fields)) if fields.len() == 1 => Ok(fields[0].clone()),
        (Forward::Only, Shape::Unnamed(1)) => Ok(Ident::new("f0", Span::call_site())),
        (Forward::Only, _) => Err(syn::Error::new(
            span,
            format!(
                "{owner} has more or fewer than one field, so `forward` needs the name of the field it wraps, as in `forward = field`"
            ),
        )),
    }
}
