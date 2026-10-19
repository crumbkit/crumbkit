use proc_macro2::TokenStream;
use quote::quote;

use crate::expand::expand;

/// Returns the derive's output for `input` as text.
fn expanded(input: TokenStream) -> String {
    let input = syn::parse2(input).expect("the test input is a Rust type");
    expand(&input)
        .expect("the derive accepts the test input")
        .to_string()
}

/// Returns the message of the compile error that the derive gives for
/// `input`.
fn refusal(input: TokenStream) -> String {
    let input = syn::parse2(input).expect("the test input is a Rust type");
    match expand(&input) {
        Ok(output) => panic!("the derive accepted the input and wrote {output}"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn the_expansion_of_the_sample_enum_matches_the_documented_expansion() {
    let input = quote! {
        pub(crate) enum BooksError {
            #[failure(
                code = NOT_FOUND,
                recovery = Recovery::Run(Call::new(&["books", "list"])),
                field(books = FieldValue::list(ids.iter().copied())),
            )]
            NotFound { ids: Vec<u32> },
            #[failure(
                code = REMOVED_FILE,
                hint = "make {name} a readable file",
                alternative = Recovery::Retry,
                url = "https://example.org/removed",
                diagnostic,
            )]
            RemovedFile { name: String, error: io::Error },
            #[failure(code = NO_STATE_DIRECTORY, hint = "set HOME to a directory")]
            NoStateDirectory,
            #[failure(forward)]
            Book(NoSuchBook),
        }
    };
    let expected = quote! {
        impl ::crumbkit::Failure for BooksError {
            fn code(&self) -> &'static ::crumbkit::Code {
                match self {
                    Self::NotFound { .. } => &NOT_FOUND,
                    Self::RemovedFile { .. } => &REMOVED_FILE,
                    Self::NoStateDirectory => &NO_STATE_DIRECTORY,
                    Self::Book(f0) => ::crumbkit::Failure::code(f0),
                }
            }

            fn recovery(&self) -> ::crumbkit::Recovery {
                match self {
                    Self::NotFound { ids } => Recovery::Run(Call::new(&["books", "list"])),
                    Self::RemovedFile { name, error } => ::crumbkit::Recovery::Hint(
                        ::std::format!("make {name} a readable file")
                    ),
                    Self::NoStateDirectory => ::crumbkit::Recovery::Hint(
                        ::std::string::String::from("set HOME to a directory")
                    ),
                    Self::Book(f0) => ::crumbkit::Failure::recovery(f0),
                }
            }

            fn alternatives(&self) -> ::std::vec::Vec<::crumbkit::Recovery> {
                match self {
                    Self::NotFound { .. } => ::std::vec::Vec::new(),
                    Self::RemovedFile { name, error } => ::std::vec![Recovery::Retry],
                    Self::NoStateDirectory => ::std::vec::Vec::new(),
                    Self::Book(f0) => ::crumbkit::Failure::alternatives(f0),
                }
            }

            fn fields(&self) -> ::std::vec::Vec<::crumbkit::Field<'_>> {
                match self {
                    Self::NotFound { ids } => ::std::vec![
                        ::crumbkit::Field::new("books", FieldValue::list(ids.iter().copied()))
                    ],
                    Self::RemovedFile { .. } => ::std::vec::Vec::new(),
                    Self::NoStateDirectory => ::std::vec::Vec::new(),
                    Self::Book(f0) => ::crumbkit::Failure::fields(f0),
                }
            }

            fn url(&self) -> ::std::option::Option<&str> {
                match self {
                    Self::NotFound { .. } => ::std::option::Option::None,
                    Self::RemovedFile { name, error } => ::std::option::Option::Some("https://example.org/removed"),
                    Self::NoStateDirectory => ::std::option::Option::None,
                    Self::Book(f0) => ::crumbkit::Failure::url(f0),
                }
            }

            fn diagnostic(&self) -> ::std::option::Option<&dyn ::miette::Diagnostic> {
                match self {
                    Self::NotFound { .. } => ::std::option::Option::None,
                    Self::RemovedFile { .. } => ::std::option::Option::Some(self),
                    Self::NoStateDirectory => ::std::option::Option::None,
                    Self::Book(..) => ::std::option::Option::None,
                }
            }
        }

        impl BooksError {
            #[doc = "Every code that a `BooksError` returns, in the order of its variants. `#[derive(Failure)]` writes this list, and it leaves out the codes of a failure that a variant forwards to."]
            pub(crate) const CODES: &'static [&'static ::crumbkit::Code] =
                &[&NOT_FOUND, &REMOVED_FILE, &NO_STATE_DIRECTORY];
        }
    };
    assert_eq!(expanded(input), expected.to_string());
}

#[test]
fn a_type_without_alternatives_fields_urls_or_diagnostics_gets_only_code_and_recovery() {
    let input = quote! {
        #[failure(code = TIMED_OUT, recovery = Recovery::Retry)]
        struct Timeout;
    };
    let expected = quote! {
        impl ::crumbkit::Failure for Timeout {
            fn code(&self) -> &'static ::crumbkit::Code {
                match self {
                    Self => &TIMED_OUT,
                }
            }

            fn recovery(&self) -> ::crumbkit::Recovery {
                match self {
                    Self => Recovery::Retry,
                }
            }
        }

        impl Timeout {
            #[doc = "Every code that a `Timeout` returns, in the order of its variants. `#[derive(Failure)]` writes this list, and it leaves out the codes of a failure that a variant forwards to."]
            const CODES: &'static [&'static ::crumbkit::Code] = &[&TIMED_OUT];
        }
    };
    assert_eq!(expanded(input), expected.to_string());
}

#[test]
fn the_crate_key_changes_the_path_of_every_crumbkit_item() {
    let output = expanded(quote! {
        #[failure(crate = ::my_crate, code = TIMED_OUT, recovery = Recovery::Retry)]
        struct Timeout;
    });
    assert!(
        output.contains(":: my_crate :: Failure for Timeout"),
        "{output}"
    );
    assert!(!output.contains(":: crumbkit ::"), "{output}");
}

#[test]
fn the_codes_list_names_a_code_that_two_variants_share_once() {
    let output = expanded(quote! {
        enum ListsError {
            #[failure(code = UNREADABLE, hint = "read it")]
            Unreadable,
            #[failure(code = UNREADABLE, hint = "open it")]
            CannotOpen,
            #[failure(code = OUT_OF_RANGE, hint = "count it")]
            OutOfRange,
        }
    });
    assert!(
        output.contains("= & [& UNREADABLE , & OUT_OF_RANGE] ;"),
        "{output}"
    );
}

#[test]
fn a_forward_to_a_named_field_binds_only_that_field() {
    let output = expanded(quote! {
        #[failure(forward = book)]
        struct ReadingList {
            name: String,
            book: NoSuchBook,
        }
    });
    assert!(
        output.contains("Self { book , .. } => :: crumbkit :: Failure :: code (book)"),
        "{output}"
    );
    assert!(
        output.contains("const CODES : & 'static [& 'static :: crumbkit :: Code] = & [] ;"),
        "{output}"
    );
}

#[test]
fn a_forward_marked_diagnostic_forwards_the_diagnostic_too() {
    let output = expanded(quote! {
        enum ConfigError {
            #[failure(forward, diagnostic)]
            Key(UnknownKey),
        }
    });
    assert!(
        output.contains("Self :: Key (f0) => :: crumbkit :: Failure :: diagnostic (f0)"),
        "{output}"
    );
}

#[test]
fn a_variant_without_a_code_is_refused_with_the_variant_name() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(hint = "list the books")]
            NotFound,
        }
    });
    assert_eq!(
        message,
        "the variant `NotFound` needs `code = ...` or `forward` in #[failure(...)]"
    );
}

#[test]
fn a_variant_without_an_attribute_is_refused_with_the_variant_name() {
    let message = refusal(quote! {
        enum BooksError {
            NotFound,
        }
    });
    assert_eq!(
        message,
        "the variant `NotFound` needs `code = ...` or `forward` in #[failure(...)]"
    );
}

#[test]
fn a_variant_with_both_a_recovery_and_a_hint_is_refused() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, recovery = Recovery::Retry, hint = "wait")]
            NotFound,
        }
    });
    assert_eq!(
        message,
        "the variant `NotFound` gives both `recovery` and `hint`, and it may give only one of them"
    );
}

#[test]
fn a_variant_with_no_recovery_is_refused() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND)]
            NotFound,
        }
    });
    assert_eq!(
        message,
        "the variant `NotFound` needs `recovery = ...` or `hint = \"...\"` in #[failure(...)]"
    );
}

#[test]
fn a_field_name_with_an_uppercase_letter_is_refused_at_compile_time() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, hint = "list them", field(Books = 7))]
            NotFound,
        }
    });
    assert_eq!(
        message,
        "the field name `Books` breaks the rule of a code segment: it must be one or more lowercase ASCII letters, digits and underscores"
    );
}

#[test]
fn a_field_name_repeated_in_one_variant_is_refused() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, hint = "list them", field(books = 7), field(books = 8))]
            NotFound,
        }
    });
    assert_eq!(
        message,
        "the variant `NotFound` names the field `books` twice"
    );
}

#[test]
fn a_key_given_twice_is_refused() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, code = MISSING, hint = "list them")]
            NotFound,
        }
    });
    assert_eq!(message, "the variant `NotFound` gives `code` twice");
}

#[test]
fn a_forwarding_variant_with_another_key_is_refused() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(forward, code = NOT_FOUND)]
            Book(NoSuchBook),
        }
    });
    assert_eq!(
        message,
        "the variant `Book` forwards to the failure it wraps, so it takes no key beside `forward` except `diagnostic`"
    );
}

#[test]
fn a_forward_without_a_field_name_on_a_variant_with_two_fields_is_refused() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(forward)]
            Book { name: String, book: NoSuchBook },
        }
    });
    assert_eq!(
        message,
        "the variant `Book` has more or fewer than one field, so `forward` needs the name of the field it wraps, as in `forward = field`"
    );
}

#[test]
fn a_forward_to_a_field_that_does_not_exist_is_refused() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(forward = volume)]
            Book { name: String, book: NoSuchBook },
        }
    });
    assert_eq!(message, "the variant `Book` has no field named `volume`");
}

#[test]
fn a_generic_type_is_refused() {
    let message = refusal(quote! {
        #[failure(forward)]
        struct Wrapped<F>(F);
    });
    assert_eq!(
        message,
        "#[derive(Failure)] does not support a type with generic parameters"
    );
}

#[test]
fn an_enum_without_variants_is_refused() {
    let message = refusal(quote! {
        enum Never {}
    });
    assert_eq!(
        message,
        "the enum `Never` has no variants, so it has no failure to describe"
    );
}

#[test]
fn a_union_is_refused() {
    let message = refusal(quote! {
        union Bits { a: u32, b: f32 }
    });
    assert_eq!(
        message,
        "#[derive(Failure)] supports enums and structs, and `Bits` is a union"
    );
}

#[test]
fn an_unknown_key_is_refused_with_the_list_of_accepted_keys() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(code = NOT_FOUND, hint = "list them", colour = "red")]
            NotFound,
        }
    });
    assert_eq!(
        message,
        "#[failure(...)] has no key `colour`. Its keys are `code`, `recovery`, `hint`, `alternative`, `field`, `url`, `diagnostic` and `forward`, and `crate` on the type"
    );
}

#[test]
fn a_variant_key_on_an_enum_type_is_refused() {
    let message = refusal(quote! {
        #[failure(code = NOT_FOUND)]
        enum BooksError {
            #[failure(code = NOT_FOUND, hint = "list them")]
            NotFound,
        }
    });
    assert_eq!(
        message,
        "on an enum, `code` goes in the #[failure(...)] attribute of each variant"
    );
}

#[test]
fn the_crate_key_on_a_variant_is_refused() {
    let message = refusal(quote! {
        enum BooksError {
            #[failure(crate = ::my_crate, code = NOT_FOUND, hint = "list them")]
            NotFound,
        }
    });
    assert_eq!(
        message,
        "`crate` goes in the #[failure(...)] attribute of the type"
    );
}
