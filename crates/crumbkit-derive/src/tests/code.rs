//! Tests for `code = EXPR` and the `codes` key.

use quote::quote;

use super::{expanded, refusal};

#[test]
fn a_code_expression_is_matched_with_every_field_bound() {
    let output = expanded(quote! {
        enum JobError {
            #[failure(
                code = if error.kind() == io::ErrorKind::NotFound { NOT_FOUND } else { NOT_STARTED },
                codes = [NOT_FOUND, NOT_STARTED],
                hint = "check the program",
            )]
            NotStarted { program: String, error: io::Error },
        }
    });
    assert!(
        output.contains(
            "Self :: NotStarted { program , error } => if error . kind () == io :: ErrorKind :: NotFound { NOT_FOUND } else { NOT_STARTED } ,"
        ),
        "{output}"
    );
}

#[test]
fn a_codes_key_names_each_code_once_in_variant_order() {
    let output = expanded(quote! {
        enum JobError {
            #[failure(code = pick(f0), codes = [SECOND, FIRST], hint = "retry")]
            Picked(u8),
            #[failure(code = FIRST, hint = "run it")]
            Plain,
            #[failure(code = pick(f0), codes = [SECOND, THIRD], hint = "wait")]
            Again(u8),
        }
    });
    assert!(output.contains("& [SECOND , FIRST , THIRD]"), "{output}");
}

#[test]
fn a_codes_key_with_an_empty_list_adds_no_code() {
    let output = expanded(quote! {
        enum JobError {
            #[failure(code = pick(f0), codes = [], hint = "retry")]
            Picked(u8),
            #[failure(code = FIRST, hint = "run it")]
            Plain,
        }
    });
    assert!(output.contains("& [FIRST]"), "{output}");
    assert!(
        output.contains("Self :: Picked (f0) => pick (f0) ,"),
        "{output}"
    );
}

#[test]
fn a_code_expression_without_a_codes_key_is_refused_with_the_variant_name() {
    let message = refusal(quote! {
        enum JobError {
            #[failure(code = pick(f0), hint = "retry")]
            NotStarted(u8),
        }
    });
    assert_eq!(
        message,
        "the variant `NotStarted` gives `code` as an expression, so it needs `codes = [...]` naming each code the expression returns, or `codes = []`"
    );
}

#[test]
fn a_codes_key_beside_a_code_path_is_refused() {
    let message = refusal(quote! {
        enum JobError {
            #[failure(code = NOT_FOUND, codes = [NOT_FOUND], hint = "retry")]
            NotStarted,
        }
    });
    assert_eq!(
        message,
        "the variant `NotStarted` gives `codes` beside a `code` that is a path, and `codes` goes with a `code` expression"
    );
}

#[test]
fn a_codes_key_without_a_code_is_refused() {
    let message = refusal(quote! {
        enum JobError {
            #[failure(codes = [NOT_FOUND], hint = "retry")]
            NotStarted,
        }
    });
    assert_eq!(
        message,
        "the variant `NotStarted` needs `code = ...` or `forward` in #[failure(...)]"
    );
}

#[test]
fn a_codes_key_beside_forward_is_refused() {
    let message = refusal(quote! {
        enum JobError {
            #[failure(forward, codes = [NOT_FOUND])]
            Inner(Other),
        }
    });
    assert!(
        message.contains("forwards to the failure it wraps"),
        "{message}"
    );
}

#[test]
fn a_code_that_names_a_field_is_an_expression_that_reads_it() {
    let output = expanded(quote! {
        enum JobError {
            #[failure(code = *held, codes = [], hint = "retry")]
            Starred { held: Code<'static> },
            #[failure(code = { held }, codes = [], hint = "retry")]
            Braced { held: Code<'static> },
            #[failure(code = held, codes = [], hint = "retry")]
            Bare { held: Code<'static> },
        }
    });
    assert!(
        output.contains("Self :: Bare { held } => held ,"),
        "{output}"
    );
}
