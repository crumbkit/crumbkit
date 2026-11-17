//! Reads a struct that derives `Row`, with its `#[row(...)]` and
//! `#[serde(...)]` attributes, and writes its `impl Row`.
//!
//! Each name in the impl is the field's name as serde serializes it, so a
//! column always names a field of the row's JSON.

use proc_macro2::TokenStream;
use syn::ext::IdentExt;
use syn::{Data, DeriveInput, Field, Fields, Ident, Path};

mod keys;
mod serde;
#[cfg(test)]
mod tests;
mod write;

use keys::{Keys, type_crate};
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
    /// The names in `COLUMNS`, in field order.
    columns: Vec<String>,
    /// The names in `LONG_COLUMNS`, in field order.
    long: Vec<String>,
    /// The names in `UNCUT_COLUMNS`, in field order.
    uncut: Vec<String>,
    /// The field that gives `problem` and its name, which `PROBLEM` holds.
    problem: Option<(Ident, String)>,
    /// The field that gives `warning` and its name, which `WARNING` holds.
    warning: Option<(Ident, String)>,
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
        warning: None,
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
        if keys.warning.is_some() {
            return Err(syn::Error::new(
                problem,
                format!(
                    "the field `{ident}` gives both `problem` and `warning`, and a row that reports a problem counts only as a problem"
                ),
            ));
        }
        row.problem = Some((ident.clone(), name.clone()));
    }
    if let Some(warning) = keys.warning {
        if keys.column.is_none() {
            return Err(syn::Error::new(
                warning,
                format!(
                    "the field `{ident}` gives `warning` without `column`, and the warning field is one of the compact columns"
                ),
            ));
        }
        if let Some((first, _)) = &row.warning {
            return Err(syn::Error::new(
                warning,
                format!(
                    "the fields `{first}` and `{ident}` both give `warning`, and a row has at most one warning field"
                ),
            ));
        }
        row.warning = Some((ident.clone(), name));
    }
    Ok(())
}
