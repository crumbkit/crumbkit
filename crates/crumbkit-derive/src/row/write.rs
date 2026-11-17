//! Writes the `impl Row` of a type from what the derive read.

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use syn::{DeriveInput, Path, Type};

use super::{ChildLines, Decision, Piece, RowType};

/// Writes the `impl Row` of `input` from what `row` says about it.
pub(super) fn write(input: &DeriveInput, row: &RowType) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let RowType {
        krate,
        whole,
        columns,
        long,
        uncut,
        flattened,
        problem,
        warning,
        children,
        ..
    } = row;
    let problem = decision(problem.as_ref(), &quote!(#krate::Problem));
    let warning = decision(warning.as_ref(), &quote!(#krate::Warning));
    let children = children.as_ref().map(|children| {
        let children = child_lines(children, krate);
        quote! {
            const CHILDREN: ::std::option::Option<#krate::Children<Self>> =
                ::std::option::Option::Some(#children);
        }
    });
    let whole = whole.then(|| {
        quote!(
            const WHOLE: bool = true;
        )
    });
    let lists = [
        List::new("COLUMNS", columns, krate),
        List::new("LONG_COLUMNS", long, krate),
        List::new("UNCUT_COLUMNS", uncut, krate),
    ];
    let [columns, long, uncut] = lists.each_ref().map(|list| &list.value);
    let impl_row = quote! {
        #[automatically_derived]
        impl #impl_generics #krate::Row for #name #type_generics #where_clause {
            const COLUMNS: &'static [&'static str] = #columns;
            const LONG_COLUMNS: &'static [&'static str] = #long;
            const UNCUT_COLUMNS: &'static [&'static str] = #uncut;
            const PROBLEM: ::std::option::Option<#krate::Problem<Self>> = #problem;
            const WARNING: ::std::option::Option<#krate::Warning<Self>> = #warning;
            #children
            #whole
        }
    };
    if flattened.is_empty() {
        return impl_row;
    }
    let spliced = lists.iter().filter_map(|list| list.constant.as_ref());
    let unset = flattened.iter().map(|ty| unset_assertions(ty, krate));
    quote! {
        const _: () = {
            #(#spliced)*
            #(#unset)*
            #impl_row
        };
    }
}

/// One of the three column lists, as the impl holds it.
struct List {
    /// The value of the list's associated constant.
    value: TokenStream,
    /// The constant that builds the list when a flattened field splices
    /// its type's list in, or `None` for a list of names alone.
    constant: Option<TokenStream>,
}

impl List {
    /// Writes the list `item`, which holds `pieces`.
    fn new(item: &str, pieces: &[Piece], krate: &Path) -> Self {
        let item = format_ident!("{item}");
        if pieces.iter().all(|piece| matches!(piece, Piece::Name(_))) {
            let names = pieces.iter().filter_map(|piece| match piece {
                Piece::Name(name) => Some(name),
                Piece::Splice(_) => None,
            });
            return Self {
                value: quote!(&[#(#names),*]),
                constant: None,
            };
        }
        // Consecutive names make one part, and each spliced list is a part.
        let mut parts: Vec<TokenStream> = Vec::new();
        let mut names: Vec<&String> = Vec::new();
        let mut count = 0_usize;
        let mut spliced: Vec<TokenStream> = Vec::new();
        let flush = |names: &mut Vec<&String>, parts: &mut Vec<TokenStream>| {
            if !names.is_empty() {
                let group = names.drain(..);
                parts.push(quote!(&[#(#group),*]));
            }
        };
        for piece in pieces {
            match piece {
                Piece::Name(name) => {
                    names.push(name);
                    count += 1;
                }
                Piece::Splice(ty) => {
                    flush(&mut names, &mut parts);
                    parts.push(quote!(<#ty as #krate::Row>::#item));
                    spliced.push(quote!(<#ty as #krate::Row>::#item.len()));
                }
            }
        }
        flush(&mut names, &mut parts);
        let local = format_ident!("__{item}");
        let count = Literal::usize_unsuffixed(count);
        Self {
            value: quote!(&#local),
            constant: Some(quote! {
                const #local: [&'static str; #count #(+ #spliced)*] =
                    #krate::__row_columns(&[#(#parts),*]);
            }),
        }
    }
}

/// Returns the assertions that fail the build when the flattened type `ty`
/// sets `PROBLEM`, `WARNING` or `CHILDREN`, which the row would drop.
fn unset_assertions(ty: &Type, krate: &Path) -> TokenStream {
    let shown = quote!(#ty).to_string();
    let assertions = ["PROBLEM", "WARNING", "CHILDREN"].map(|item| {
        let item = format_ident!("{item}");
        let message = format!(
            "the flattened type `{shown}` sets {item}, and the row that flattens it would drop the setting, so set it on the outer row"
        );
        quote!(::std::assert!(<#ty as #krate::Row>::#item.is_none(), #message);)
    });
    quote!(const _: () = { #(#assertions)* };)
}

/// Returns the value of `PROBLEM` or `WARNING`, built with the `field` or
/// `by` constructor of `kind`, or `None` when no field decides.
fn decision(decision: Option<&Decision>, kind: &TokenStream) -> TokenStream {
    let Some(Decision { name, by, .. }) = decision else {
        return quote!(::std::option::Option::None);
    };
    match by {
        Some(by) => quote!(::std::option::Option::Some(#kind::by(#name, #by))),
        None => quote!(::std::option::Option::Some(#kind::field(#name))),
    }
}

/// Returns the `Children` value that the field's `children` options build,
/// with one call for each option the field gave.
fn child_lines(children: &ChildLines, krate: &Path) -> TokenStream {
    let ChildLines { name, options, .. } = children;
    let indent = options.indent.map(|indent| {
        let indent = Literal::u8_unsuffixed(indent);
        quote!(.indent(#indent))
    });
    let spaced = options.spaced.then(|| quote!(.spaced()));
    let help = options.help.as_ref().map(|help| quote!(.help(#help)));
    let lines = options.lines.as_ref().map(|lines| quote!(.lines(#lines)));
    let long_lines = options
        .long_lines
        .as_ref()
        .map(|lines| quote!(.long_lines(#lines)));
    quote!(#krate::Children::field(#name) #indent #spaced #help #lines #long_lines)
}
