//! Writes the `impl Row` of a type from what the derive read.

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use super::RowType;

/// Writes the `impl Row` of `input` from what `row` says about it.
pub(super) fn write(input: &DeriveInput, row: &RowType) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let RowType {
        krate,
        columns,
        long,
        uncut,
        problem,
        warning,
    } = row;
    let problem = problem.as_ref().map_or_else(
        || quote!(::std::option::Option::None),
        |(_, field)| quote!(::std::option::Option::Some(#krate::Problem::field(#field))),
    );
    let warning = warning.as_ref().map_or_else(
        || quote!(::std::option::Option::None),
        |(_, field)| quote!(::std::option::Option::Some(#krate::Warning::field(#field))),
    );
    quote! {
        #[automatically_derived]
        impl #impl_generics #krate::Row for #name #type_generics #where_clause {
            const COLUMNS: &'static [&'static str] = &[#(#columns),*];
            const LONG_COLUMNS: &'static [&'static str] = &[#(#long),*];
            const UNCUT_COLUMNS: &'static [&'static str] = &[#(#uncut),*];
            const PROBLEM: ::std::option::Option<#krate::Problem<Self>> = #problem;
            const WARNING: ::std::option::Option<#krate::Warning<Self>> = #warning;
        }
    }
}
