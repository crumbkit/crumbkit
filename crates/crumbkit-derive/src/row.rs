//! Reads a struct that derives `Row`, with its `#[row(...)]` and
//! `#[serde(...)]` attributes, and writes its `impl Row`.
//!
//! Each name in the impl is the field's name as serde serializes it, so a
//! column always names a field of the row's JSON.

use proc_macro2::{Span, TokenStream};
use syn::ext::IdentExt;
use syn::{Data, DeriveInput, Expr, Field, Fields, Ident, Path};

mod keys;
mod serde;
#[cfg(test)]
mod tests;
mod write;

use keys::{ChildrenKey, Keys, TypeKeys};
use serde::{RenameAll, SerdeField, rename_all};
use write::write;

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
    /// Whether the type gives `whole`, which sets `WHOLE`.
    whole: bool,
    /// The names in `COLUMNS`, in field order.
    columns: Vec<String>,
    /// The names in `LONG_COLUMNS`, in field order.
    long: Vec<String>,
    /// The names in `UNCUT_COLUMNS`, in field order.
    uncut: Vec<String>,
    /// The field that decides `problem`, which `PROBLEM` holds.
    problem: Option<Decision>,
    /// The field that decides `warning`, which `WARNING` holds.
    warning: Option<Decision>,
    /// The field that holds the child lines, which `CHILDREN` holds.
    children: Option<ChildLines>,
}

/// The field that decides which rows report a problem or a warning.
struct Decision {
    /// The field's Rust name, for the messages that name it.
    field: Ident,
    /// The key that the field gave, such as `problem` or `warning_by`.
    key: &'static str,
    /// The field's name as serde serializes it, which names the column.
    name: String,
    /// The expression of `problem_by` or `warning_by`, or `None` when the
    /// column's own value decides.
    by: Option<Expr>,
}

/// The field that holds the child lines.
struct ChildLines {
    /// The field's Rust name, for the messages that name it.
    field: Ident,
    /// The field's name as serde serializes it.
    name: String,
    /// The options of the `children` key.
    options: ChildrenKey,
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
    let type_keys = TypeKeys::read(&input.attrs)?;
    let mut row = RowType {
        krate: type_keys.krate,
        whole: type_keys.whole,
        columns: Vec::new(),
        long: Vec::new(),
        uncut: Vec::new(),
        problem: None,
        warning: None,
        children: None,
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

/// Returns the key, the span and the expression of the `problem` or
/// `warning` key that a field gives, or `None` when it gives neither. A field
/// that gives both the plain key and the `_by` key is an error.
fn decision_key<'a>(
    plain: Option<(&'static str, Span)>,
    by: Option<(&'static str, &'a (Span, Expr))>,
    ident: &Ident,
) -> syn::Result<Option<(&'static str, Span, Option<&'a Expr>)>> {
    Ok(match (plain, by) {
        (Some((key, _)), Some((by_key, (span, _)))) => {
            return Err(syn::Error::new(
                *span,
                format!(
                    "the field `{ident}` gives both `{key}` and `{by_key}`, and a field names how it decides once"
                ),
            ));
        }
        (Some((key, span)), None) => Some((key, span, None)),
        (None, Some((key, (span, expr)))) => Some((key, *span, Some(expr))),
        (None, None) => None,
    })
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
    if keys.children.is_some() {
        return read_children(row, &keys, ident, name);
    }
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
    let problem = decision_key(
        keys.problem.map(|span| ("problem", span)),
        keys.problem_by.as_ref().map(|by| ("problem_by", by)),
        ident,
    )?;
    let warning = decision_key(
        keys.warning.map(|span| ("warning", span)),
        keys.warning_by.as_ref().map(|by| ("warning_by", by)),
        ident,
    )?;
    if let Some((key, span, by)) = problem {
        if keys.column.is_none() {
            return Err(syn::Error::new(
                span,
                format!(
                    "the field `{ident}` gives `{key}` without `column`, and the problem field is one of the compact columns"
                ),
            ));
        }
        if let Some(first) = &row.problem {
            return Err(second_decision(span, "problem", first, key, ident));
        }
        if let Some((warning_key, ..)) = warning {
            return Err(syn::Error::new(
                span,
                format!(
                    "the field `{ident}` gives both `{key}` and `{warning_key}`, and a row that reports a problem counts only as a problem"
                ),
            ));
        }
        row.problem = Some(Decision {
            field: ident.clone(),
            key,
            name: name.clone(),
            by: by.cloned(),
        });
    }
    if let Some((key, span, by)) = warning {
        if keys.column.is_none() {
            return Err(syn::Error::new(
                span,
                format!(
                    "the field `{ident}` gives `{key}` without `column`, and the warning field is one of the compact columns"
                ),
            ));
        }
        if let Some(first) = &row.warning {
            return Err(second_decision(span, "warning", first, key, ident));
        }
        row.warning = Some(Decision {
            field: ident.clone(),
            key,
            name,
            by: by.cloned(),
        });
    }
    Ok(())
}

/// Returns the error for a second field that decides the row's problem or
/// warning, which `noun` names.
fn second_decision(
    span: Span,
    noun: &str,
    first: &Decision,
    key: &str,
    ident: &Ident,
) -> syn::Error {
    let first_field = &first.field;
    let gives = if first.key == key {
        format!("both give `{key}`")
    } else {
        format!("give `{}` and `{key}`", first.key)
    };
    syn::Error::new(
        span,
        format!(
            "the fields `{first_field}` and `{ident}` {gives}, and a row has at most one {noun} field"
        ),
    )
}

/// Records the field `ident` as the row's child lines, or returns the error
/// for a field that also gives a key of a column or a second children field.
fn read_children(row: &mut RowType, keys: &Keys, ident: &Ident, name: String) -> syn::Result<()> {
    let Some((span, options)) = keys.children.as_ref() else {
        return Ok(());
    };
    let others = [
        ("column", keys.column),
        ("long", keys.long),
        ("uncut", keys.uncut),
        ("problem", keys.problem),
        (
            "problem_by",
            keys.problem_by.as_ref().map(|(span, _)| *span),
        ),
        ("warning", keys.warning),
        (
            "warning_by",
            keys.warning_by.as_ref().map(|(span, _)| *span),
        ),
    ];
    if let Some((key, other)) = others
        .into_iter()
        .find_map(|(key, span)| Some((key, span?)))
    {
        return Err(syn::Error::new(
            other,
            format!(
                "the field `{ident}` gives `children` and `{key}`, and a field that holds the child lines is in no column"
            ),
        ));
    }
    if let Some(first) = &row.children {
        return Err(syn::Error::new(
            *span,
            format!(
                "the fields `{}` and `{ident}` both give `children`, and a row has at most one children field",
                first.field
            ),
        ));
    }
    row.children = Some(ChildLines {
        field: ident.clone(),
        name,
        options: options.clone(),
    });
    Ok(())
}
