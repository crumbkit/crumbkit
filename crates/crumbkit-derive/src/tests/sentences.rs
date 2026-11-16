//! Tests for the `sentence` and `reviewed` keys.

use quote::quote;

use super::{expanded, refusal};

#[test]
fn the_expansion_of_a_variant_with_a_sentence_returns_it() {
    let output = expanded(quote! {
        enum BooksError {
            #[failure(
                code = NOT_FOUND,
                sentence = "The catalogue has no book with that id.",
                hint = "list the books",
            )]
            NotFound { ids: Vec<u32> },
        }
    });
    assert!(
        output.contains("fn sentence (& self) -> :: std :: option :: Option < & 'static str >"),
        "{output}"
    );
    assert!(
        output.contains(
            ":: std :: option :: Option :: Some (\"The catalogue has no book with that id.\")"
        ),
        "{output}"
    );
}

#[test]
fn a_type_with_no_sentence_gets_no_sentence_method() {
    let output = expanded(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, hint = "list the books")]
            NotFound,
        }
    });
    assert!(!output.contains("sentence"), "{output}");
}

#[test]
fn a_variant_with_forward_forwards_the_sentence() {
    let output = expanded(quote! {
        #[failure(reviewed)]
        enum BooksError {
            #[failure(code = NOT_FOUND, sentence = "The book is missing.", hint = "list the books")]
            NotFound,
            #[failure(forward)]
            Lending(LendingError),
        }
    });
    assert!(
        output.contains("Self :: Lending (f0) => :: crumbkit :: Failure :: sentence (f0)"),
        "{output}"
    );
}

#[test]
fn a_reviewed_enum_with_a_variant_that_has_no_sentence_is_a_compile_error_that_names_the_variant() {
    let message = refusal(quote! {
        #[failure(reviewed)]
        enum BooksError {
            #[failure(code = NOT_FOUND, sentence = "The book is missing.", hint = "list the books")]
            NotFound,
            #[failure(code = NO_STATE, hint = "set HOME")]
            NoState,
        }
    });
    assert_eq!(
        message,
        "the variant `NoState` needs `sentence = \"...\"` because its type is marked `#[failure(reviewed)]`"
    );
}

#[test]
fn a_reviewed_enum_accepts_a_forwarding_variant_without_a_sentence() {
    let output = expanded(quote! {
        #[failure(reviewed)]
        enum BooksError {
            #[failure(code = NOT_FOUND, sentence = "The book is missing.", hint = "list the books")]
            NotFound,
            #[failure(forward)]
            Lending(LendingError),
        }
    });
    assert!(output.contains("fn sentence"), "{output}");
}

#[test]
fn an_enum_without_reviewed_accepts_a_variant_without_a_sentence() {
    let output = expanded(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, sentence = "The book is missing.", hint = "list the books")]
            NotFound,
            #[failure(code = NO_STATE, hint = "set HOME")]
            NoState,
        }
    });
    assert!(
        output.contains("Self :: NoState => :: std :: option :: Option :: None"),
        "{output}"
    );
}

#[test]
fn a_reviewed_struct_without_a_sentence_is_refused() {
    let message = refusal(quote! {
        #[failure(reviewed, code = NO_STATE, hint = "set HOME")]
        struct NoState;
    });
    assert_eq!(
        message,
        "the type `NoState` needs `sentence = \"...\"` because its type is marked `#[failure(reviewed)]`"
    );
}

#[test]
fn a_reviewed_struct_with_a_sentence_returns_it() {
    let output = expanded(quote! {
        #[failure(reviewed, code = NO_STATE, sentence = "The state directory is missing.", hint = "set HOME")]
        struct NoState;
    });
    assert!(
        output.contains("Option :: Some (\"The state directory is missing.\")"),
        "{output}"
    );
}

#[test]
fn an_empty_sentence_is_a_compile_error() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, sentence = "", hint = "list the books")]
            NotFound,
        }
    });
    assert_eq!(
        message,
        "the variant `NotFound` gives an empty `sentence`, and a sentence needs text"
    );
}

#[test]
fn a_sentence_with_a_line_break_is_a_compile_error() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, sentence = "One.\nTwo.", hint = "list the books")]
            NotFound,
        }
    });
    assert_eq!(
        message,
        "the variant `NotFound` gives a `sentence` with a line break, and a sentence is one line"
    );
}

#[test]
fn a_sentence_with_a_brace_is_a_compile_error() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, sentence = "No book {id}.", hint = "list the books")]
            NotFound,
        }
    });
    assert_eq!(
        message,
        "the variant `NotFound` gives a `sentence` with `{`, and a sentence is fixed text with no placeholders"
    );
}

#[test]
fn a_sentence_that_is_not_a_string_literal_is_a_compile_error() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, sentence = SENTENCE, hint = "list the books")]
            NotFound,
        }
    });
    assert_eq!(message, "expected string literal");
}

#[test]
fn a_sentence_given_twice_is_a_compile_error() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, sentence = "One.", sentence = "Two.", hint = "list the books")]
            NotFound,
        }
    });
    assert_eq!(message, "the variant `NotFound` gives `sentence` twice");
}

#[test]
fn a_sentence_on_a_forwarding_variant_is_a_compile_error() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(forward, sentence = "One.")]
            Lending(LendingError),
        }
    });
    assert_eq!(
        message,
        "the variant `Lending` forwards to the failure it wraps, so it takes no key beside `forward` except `diagnostic`"
    );
}

#[test]
fn the_reviewed_key_on_a_variant_is_a_compile_error() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, reviewed, hint = "list the books")]
            NotFound,
        }
    });
    assert_eq!(
        message,
        "`reviewed` goes in the #[failure(...)] attribute of the type"
    );
}
