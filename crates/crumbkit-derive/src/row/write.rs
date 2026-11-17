//! Writes the `impl Row` of a type from what the derive read.

use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::DeriveInput;

use super::{ChildLines, Decision, RowType};

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
        problem,
        warning,
        children,
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
    quote! {
        #[automatically_derived]
        impl #impl_generics #krate::Row for #name #type_generics #where_clause {
            const COLUMNS: &'static [&'static str] = &[#(#columns),*];
            const LONG_COLUMNS: &'static [&'static str] = &[#(#long),*];
            const UNCUT_COLUMNS: &'static [&'static str] = &[#(#uncut),*];
            const PROBLEM: ::std::option::Option<#krate::Problem<Self>> = #problem;
            const WARNING: ::std::option::Option<#krate::Warning<Self>> = #warning;
            #children
            #whole
        }
    }
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
fn child_lines(children: &ChildLines, krate: &syn::Path) -> TokenStream {
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
