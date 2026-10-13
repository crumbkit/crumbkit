//! Writes the `impl Failure` and the `CODES` list of a type that derives
//! `Failure`.
//!
//! Each generated method is one `match` that names every arm, as a
//! hand-written impl does. A method that no arm answers is left out, so the
//! trait's default applies.

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{DeriveInput, Ident, Path};

use crate::parse::{self, Answers, Arm, FailureType, Own, RecoveryKey, Shape};

/// Returns the code that `#[derive(Failure)]` writes for `input`, or the
/// compile error that names what the input gets wrong.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let failure = parse::read(input)?;
    Ok(write(&failure))
}

/// Writes the impl and the `CODES` list of `failure`.
fn write(failure: &FailureType) -> TokenStream {
    let FailureType {
        name,
        visibility,
        krate,
        arms,
    } = failure;
    let code = code(krate, arms);
    let recovery = recovery(krate, arms);
    let alternatives = alternatives(krate, arms);
    let fields = fields(krate, arms);
    let url = url(krate, arms);
    let diagnostic = diagnostic(krate, arms);
    let codes = codes(arms);
    let doc = format!(
        "Every code that a `{name}` returns, in the order of its variants. \
         `#[derive(Failure)]` writes this list, and it leaves out the codes of a \
         failure that a variant forwards to."
    );
    quote! {
        impl #krate::Failure for #name {
            #code
            #recovery
            #alternatives
            #fields
            #url
            #diagnostic
        }

        impl #name {
            #[doc = #doc]
            #visibility const CODES: &'static [&'static #krate::Code] = &[#(&#codes),*];
        }
    }
}

/// Writes `code()`.
fn code(krate: &Path, arms: &[Arm]) -> TokenStream {
    let arms = arms.iter().map(|arm| match &arm.answers {
        Answers::Own(own) => {
            let pattern = unbound(arm);
            let code = &own.code;
            quote!(#pattern => &#code,)
        }
        Answers::Forward { field, .. } => forward(krate, arm, field, "code"),
    });
    quote! {
        fn code(&self) -> &'static #krate::Code {
            match self {
                #(#arms)*
            }
        }
    }
}

/// Writes `recovery()`. A hint whose text holds `{` goes through `format!`,
/// so its braces name variables, and any other hint is copied as written.
fn recovery(krate: &Path, arms: &[Arm]) -> TokenStream {
    let arms = arms.iter().map(|arm| match &arm.answers {
        Answers::Own(own) => {
            let pattern = bound(arm);
            let recovery = match &own.recovery {
                RecoveryKey::Expr(expr) => expr.to_token_stream(),
                RecoveryKey::Hint(hint) if hint.value().contains('{') => {
                    quote!(#krate::Recovery::Hint(::std::format!(#hint)))
                }
                RecoveryKey::Hint(hint) => {
                    quote!(#krate::Recovery::Hint(::std::string::String::from(#hint)))
                }
            };
            quote!(#pattern => #recovery,)
        }
        Answers::Forward { field, .. } => forward(krate, arm, field, "recovery"),
    });
    quote! {
        fn recovery(&self) -> #krate::Recovery {
            match self {
                #(#arms)*
            }
        }
    }
}

/// Writes `alternatives()`, unless no arm has an alternative or forwards.
fn alternatives(krate: &Path, arms: &[Arm]) -> TokenStream {
    if !needed(arms, |own| !own.alternatives.is_empty()) {
        return TokenStream::new();
    }
    let arms = arms.iter().map(|arm| match &arm.answers {
        Answers::Own(own) if !own.alternatives.is_empty() => {
            let pattern = bound(arm);
            let alternatives = &own.alternatives;
            quote!(#pattern => ::std::vec![#(#alternatives),*],)
        }
        Answers::Own(_) => {
            let pattern = unbound(arm);
            quote!(#pattern => ::std::vec::Vec::new(),)
        }
        Answers::Forward { field, .. } => forward(krate, arm, field, "alternatives"),
    });
    quote! {
        fn alternatives(&self) -> ::std::vec::Vec<#krate::Recovery> {
            match self {
                #(#arms)*
            }
        }
    }
}

/// Writes `fields()`, unless no arm has a field or forwards.
fn fields(krate: &Path, arms: &[Arm]) -> TokenStream {
    if !needed(arms, |own| !own.fields.is_empty()) {
        return TokenStream::new();
    }
    let arms = arms.iter().map(|arm| match &arm.answers {
        Answers::Own(own) if !own.fields.is_empty() => {
            let pattern = bound(arm);
            let fields = own
                .fields
                .iter()
                .map(|(name, value)| quote!(#krate::Field::new(#name, #value)));
            quote!(#pattern => ::std::vec![#(#fields),*],)
        }
        Answers::Own(_) => {
            let pattern = unbound(arm);
            quote!(#pattern => ::std::vec::Vec::new(),)
        }
        Answers::Forward { field, .. } => forward(krate, arm, field, "fields"),
    });
    quote! {
        fn fields(&self) -> ::std::vec::Vec<#krate::Field<'_>> {
            match self {
                #(#arms)*
            }
        }
    }
}

/// Writes `url()`, unless no arm has a url or forwards.
fn url(krate: &Path, arms: &[Arm]) -> TokenStream {
    if !needed(arms, |own| own.url.is_some()) {
        return TokenStream::new();
    }
    let arms = arms.iter().map(|arm| match &arm.answers {
        Answers::Own(own) if own.url.is_some() => {
            let pattern = bound(arm);
            let url = &own.url;
            quote!(#pattern => ::std::option::Option::Some(#url),)
        }
        Answers::Own(_) => {
            let pattern = unbound(arm);
            quote!(#pattern => ::std::option::Option::None,)
        }
        Answers::Forward { field, .. } => forward(krate, arm, field, "url"),
    });
    quote! {
        fn url(&self) -> ::std::option::Option<&str> {
            match self {
                #(#arms)*
            }
        }
    }
}

/// Writes `diagnostic()`, unless no arm is marked `diagnostic`. The method
/// exists only with crumbkit's `miette` feature, so a type that marks no arm
/// builds without that feature.
fn diagnostic(krate: &Path, arms: &[Arm]) -> TokenStream {
    let marked = arms.iter().any(|arm| match &arm.answers {
        Answers::Own(own) => own.diagnostic,
        Answers::Forward { diagnostic, .. } => *diagnostic,
    });
    if !marked {
        return TokenStream::new();
    }
    let arms = arms.iter().map(|arm| match &arm.answers {
        Answers::Own(own) if own.diagnostic => {
            let pattern = unbound(arm);
            quote!(#pattern => ::std::option::Option::Some(self),)
        }
        Answers::Forward {
            field,
            diagnostic: true,
        } => forward(krate, arm, field, "diagnostic"),
        Answers::Own(_) | Answers::Forward { .. } => {
            let pattern = unbound(arm);
            quote!(#pattern => ::std::option::Option::None,)
        }
    });
    quote! {
        fn diagnostic(&self) -> ::std::option::Option<&dyn ::miette::Diagnostic> {
            match self {
                #(#arms)*
            }
        }
    }
}

/// Returns each arm's own code once, in arm order. Two codes are the same
/// when their paths are written the same.
fn codes(arms: &[Arm]) -> Vec<&Path> {
    let mut codes: Vec<&Path> = Vec::new();
    for arm in arms {
        if let Answers::Own(own) = &arm.answers {
            let text = own.code.to_token_stream().to_string();
            if !codes
                .iter()
                .any(|seen| seen.to_token_stream().to_string() == text)
            {
                codes.push(&own.code);
            }
        }
    }
    codes
}

/// Returns whether a method is needed: some arm forwards, or `answers`
/// holds for some arm's own answers.
fn needed(arms: &[Arm], answers: impl Fn(&Own) -> bool) -> bool {
    arms.iter().any(|arm| match &arm.answers {
        Answers::Own(own) => answers(own),
        Answers::Forward { .. } => true,
    })
}

/// Writes the arm that calls `method` on the wrapped failure in `field`.
fn forward(krate: &Path, arm: &Arm, field: &Ident, method: &str) -> TokenStream {
    let path = &arm.path;
    let method = format_ident!("{method}");
    let pattern = match &arm.shape {
        Shape::Named(_) => quote!(#path { #field, .. }),
        Shape::Unit | Shape::Unnamed(_) => quote!(#path(#field)),
    };
    quote!(#pattern => #krate::Failure::#method(#field),)
}

/// Returns the arm's pattern with every field bound to a variable, so an
/// expression from the attribute can read the fields.
fn bound(arm: &Arm) -> TokenStream {
    let path = &arm.path;
    match &arm.shape {
        Shape::Unit => quote!(#path),
        Shape::Named(names) => quote!(#path { #(#names),* }),
        Shape::Unnamed(count) => {
            let names = (0..*count).map(|index| format_ident!("f{index}"));
            quote!(#path(#(#names),*))
        }
    }
}

/// Returns the arm's pattern with no field bound.
fn unbound(arm: &Arm) -> TokenStream {
    let path = &arm.path;
    match &arm.shape {
        Shape::Unit => quote!(#path),
        Shape::Named(_) => quote!(#path { .. }),
        Shape::Unnamed(_) => quote!(#path(..)),
    }
}
