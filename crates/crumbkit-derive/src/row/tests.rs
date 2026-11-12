use proc_macro2::TokenStream;
use quote::quote;

use super::expand;

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
fn the_expansion_of_the_sample_struct_matches_the_documented_expansion() {
    let input = quote! {
        #[serde(rename_all = "camelCase")]
        struct Book {
            #[row(column, uncut)]
            id: u32,
            #[row(column)]
            #[serde(rename = "name")]
            title: String,
            #[row(long)]
            page_count: u32,
            #[row(column, problem)]
            warning: Option<String>,
            isbn: String,
        }
    };
    let expected = quote! {
        #[automatically_derived]
        impl ::crumbkit::Row for Book {
            const COLUMNS: &'static [&'static str] = &["id", "name", "warning"];
            const LONG_COLUMNS: &'static [&'static str] = &["pageCount"];
            const UNCUT_COLUMNS: &'static [&'static str] = &["id"];
            const PROBLEM: ::std::option::Option<::crumbkit::Problem<Self>> =
                ::std::option::Option::Some(::crumbkit::Problem::field("warning"));
        }
    };
    assert_eq!(expanded(input), expected.to_string());
}

#[test]
fn a_struct_without_row_keys_gets_empty_lists_and_no_problem_field() {
    let output = expanded(quote! {
        struct Empty {
            id: u32,
        }
    });
    let expected = quote! {
        #[automatically_derived]
        impl ::crumbkit::Row for Empty {
            const COLUMNS: &'static [&'static str] = &[];
            const LONG_COLUMNS: &'static [&'static str] = &[];
            const UNCUT_COLUMNS: &'static [&'static str] = &[];
            const PROBLEM: ::std::option::Option<::crumbkit::Problem<Self>> =
                ::std::option::Option::None;
        }
    };
    assert_eq!(output, expected.to_string());
}

#[test]
fn a_raw_field_name_is_written_without_its_prefix() {
    let output = expanded(quote! {
        struct Book {
            #[row(column)]
            r#type: String,
        }
    });
    assert!(output.contains(r#"& ["type"]"#), "{output}");
}

#[test]
fn serde_keys_that_do_not_rename_a_field_are_read_past() {
    let output = expanded(quote! {
        #[serde(deny_unknown_fields, bound(serialize = "T: Serialize"))]
        struct Book {
            #[row(column)]
            #[serde(default, skip_serializing_if = "Option::is_none", alias = "name")]
            title: Option<String>,
        }
    });
    assert!(output.contains(r#"& ["title"]"#), "{output}");
}

#[test]
fn the_impl_copies_the_generic_parameters_and_the_where_clause() {
    let output = expanded(quote! {
        struct Labelled<'a, T: Serialize>
        where
            T: JsonSchema,
        {
            #[row(column)]
            label: &'a str,
            value: T,
        }
    });
    let head = quote! {
        #[automatically_derived]
        impl<'a, T: Serialize> ::crumbkit::Row for Labelled<'a, T> where T: JsonSchema,
    };
    assert!(output.starts_with(&head.to_string()), "{output}");
}

#[test]
fn the_crate_key_changes_the_path_of_the_trait() {
    let output = expanded(quote! {
        #[row(crate = ::my_crate)]
        struct Book {
            #[row(column)]
            id: u32,
        }
    });
    assert!(output.contains(":: my_crate :: Row for Book"), "{output}");
    assert!(!output.contains(":: crumbkit ::"), "{output}");
}

#[test]
fn each_refused_use_is_a_compile_error_that_names_the_field_or_the_type() {
    let dropped = |key: &str| {
        format!(
            "the field `notes` is marked #[serde({key})], so the JSON holds no field of that name, and it takes no #[row(...)] key"
        )
    };
    let keys =
        "Its keys are `column`, `long`, `uncut` and `problem` on a field, and `crate` on the type";
    let refused = "#[derive(Row)] supports a struct with named fields, and";
    let cases = [
        (
            quote! { struct Book { #[row(colum)] id: u32 } },
            format!("the field `id` gives `colum`, and #[row(...)] has no such key. {keys}"),
        ),
        (
            quote! { #[row(colum)] struct Book { id: u32 } },
            format!("the type gives `colum`, and #[row(...)] has no such key. {keys}"),
        ),
        (
            quote! { struct Book { #[row(column)] #[row(column)] id: u32 } },
            "the field `id` gives `column` twice".to_owned(),
        ),
        (
            quote! { struct Book { #[row(column, long)] id: u32 } },
            "the field `id` gives both `column` and `long`, and a field is in only one of the two lists".to_owned(),
        ),
        (
            quote! { struct Book { #[row(uncut)] id: u32 } },
            "the field `id` gives `uncut` without `column` or `long`, and only a column can be uncut".to_owned(),
        ),
        (
            quote! { struct Book { #[row(long, problem)] error: Option<String> } },
            "the field `error` gives `problem` without `column`, and the problem field is one of the compact columns".to_owned(),
        ),
        (
            quote! { struct Book { #[serde(skip)] #[row(long)] notes: String } },
            dropped("skip"),
        ),
        (
            quote! { struct Book { #[serde(skip_serializing)] #[row(long)] notes: String } },
            dropped("skip_serializing"),
        ),
        (
            quote! { struct Book { #[serde(flatten)] #[row(long)] notes: Notes } },
            dropped("flatten"),
        ),
        (
            quote! {
                struct Check {
                    #[row(column, problem)] error: Option<String>,
                    #[row(column, problem)] warning: Option<String>,
                }
            },
            "the fields `error` and `warning` both give `problem`, and a row has at most one problem field".to_owned(),
        ),
        (
            quote! { #[row(column)] struct Book { id: u32 } },
            "`column` goes in the #[row(...)] attribute of a field".to_owned(),
        ),
        (
            quote! { struct Book { #[row(crate = ::crumbkit)] id: u32 } },
            "`crate` goes in the #[row(...)] attribute of the type".to_owned(),
        ),
        (
            quote! { #[serde(transparent)] struct Id { #[row(column)] id: u32 } },
            "the type `Id` is marked #[serde(transparent)], so its JSON has no field names for #[derive(Row)] to list".to_owned(),
        ),
        (
            quote! { enum State { Done } },
            format!("{refused} `State` is an enum, so it writes `impl Row` by hand"),
        ),
        (
            quote! { union Bits { whole: u32 } },
            format!("{refused} `Bits` is a union"),
        ),
        (
            quote! { struct Pair(u32, u32); },
            format!("{refused} `Pair` is a tuple struct"),
        ),
        (
            quote! { struct Nothing; },
            format!("{refused} `Nothing` is a unit struct"),
        ),
        (
            quote! { #[serde(rename_all = "Title Case")] struct Book { #[row(column)] id: u32 } },
            "#[serde(rename_all)] has no rule \"Title Case\". Its rules are \"lowercase\", \"UPPERCASE\", \"PascalCase\", \"camelCase\", \"snake_case\", \"SCREAMING_SNAKE_CASE\", \"kebab-case\" and \"SCREAMING-KEBAB-CASE\"".to_owned(),
        ),
    ];
    for (input, message) in cases {
        assert_eq!(refusal(input.clone()), message, "{input}");
    }
}
