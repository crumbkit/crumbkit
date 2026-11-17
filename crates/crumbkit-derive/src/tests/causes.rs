//! Tests for the `causes` key.

use quote::quote;

use super::{expanded, refusal};

#[test]
fn a_type_without_a_causes_key_or_a_forward_gets_no_causes_method() {
    let output = expanded(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, hint = "list the books")]
            NotFound,
        }
    });
    assert!(!output.contains("causes"), "{output}");
}

#[test]
fn a_causes_key_returns_the_expression_for_its_variant_and_an_empty_list_for_the_others() {
    let output = expanded(quote! {
        enum FlowsError {
            #[failure(code = INVALID, hint = "run check", causes = findings.clone())]
            Invalid { findings: Vec<String> },
            #[failure(code = EMPTY, hint = "add a flow")]
            Empty,
        }
    });
    assert!(
        output
            .contains("fn causes (& self) -> :: std :: vec :: Vec < :: std :: string :: String >"),
        "{output}"
    );
    assert!(
        output.contains("Self :: Invalid { findings } => findings . clone () ,"),
        "{output}"
    );
    assert!(
        output.contains("Self :: Empty => :: std :: vec :: Vec :: new () ,"),
        "{output}"
    );
}

#[test]
fn a_variant_with_forward_forwards_the_causes() {
    let output = expanded(quote! {
        enum FlowsError {
            #[failure(code = EMPTY, hint = "add a flow")]
            Empty,
            #[failure(forward)]
            Book(BooksError),
        }
    });
    assert!(
        output.contains("Self :: Book (f0) => :: crumbkit :: Failure :: causes (f0)"),
        "{output}"
    );
}

#[test]
fn a_causes_key_on_a_forwarding_variant_is_a_compile_error_that_names_the_variant() {
    let message = refusal(quote! {
        enum FlowsError {
            #[failure(forward, causes = Vec::new())]
            Book(BooksError),
        }
    });
    assert!(message.contains("`Book`"), "{message}");
}

#[test]
fn a_causes_key_given_twice_is_a_compile_error_that_names_the_variant() {
    let message = refusal(quote! {
        enum FlowsError {
            #[failure(code = INVALID, hint = "run check", causes = a(), causes = b())]
            Invalid,
        }
    });
    assert!(
        message.contains("`Invalid`") && message.contains("`causes`"),
        "{message}"
    );
}
