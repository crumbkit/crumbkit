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

/// Returns the `COLUMNS` that the derive writes for a struct with the
/// type attributes `attrs` and one column, `first_page_count`.
fn column_of(attrs: &TokenStream) -> String {
    expanded(quote! {
        #attrs
        struct Book {
            #[row(column)]
            first_page_count: u32,
        }
    })
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
        impl ::crumbkit::Row for Book {
            const COLUMNS: &'static [&'static str] = &["id", "name", "warning"];
            const LONG_COLUMNS: &'static [&'static str] = &["pageCount"];
            const UNCUT_COLUMNS: &'static [&'static str] = &["id"];
            const PROBLEM: ::std::option::Option<&'static str> =
                ::std::option::Option::Some("warning");
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
        impl ::crumbkit::Row for Empty {
            const COLUMNS: &'static [&'static str] = &[];
            const LONG_COLUMNS: &'static [&'static str] = &[];
            const UNCUT_COLUMNS: &'static [&'static str] = &[];
            const PROBLEM: ::std::option::Option<&'static str> = ::std::option::Option::None;
        }
    };
    assert_eq!(output, expected.to_string());
}

#[test]
fn each_rename_all_rule_changes_a_field_name_as_serde_does() {
    let rules = [
        ("lowercase", "first_page_count"),
        ("UPPERCASE", "FIRST_PAGE_COUNT"),
        ("PascalCase", "FirstPageCount"),
        ("camelCase", "firstPageCount"),
        ("snake_case", "first_page_count"),
        ("SCREAMING_SNAKE_CASE", "FIRST_PAGE_COUNT"),
        ("kebab-case", "first-page-count"),
        ("SCREAMING-KEBAB-CASE", "FIRST-PAGE-COUNT"),
    ];
    for (rule, name) in rules {
        let output = column_of(&quote!(#[serde(rename_all = #rule)]));
        let columns = format!("const COLUMNS : & 'static [& 'static str] = & [{name:?}] ;");
        assert!(output.contains(&columns), "{rule}: {output}");
    }
}

#[test]
fn a_rename_all_rule_for_serialize_alone_changes_the_field_name() {
    let output = column_of(&quote!(#[serde(rename_all(serialize = "kebab-case"))]));
    assert!(output.contains(r#"& ["first-page-count"]"#), "{output}");
}

#[test]
fn a_rename_on_the_field_wins_over_the_rename_all_rule() {
    let output = expanded(quote! {
        #[serde(rename_all = "UPPERCASE")]
        struct Book {
            #[row(column)]
            #[serde(rename = "Title")]
            title: String,
            #[row(column)]
            #[serde(rename(serialize = "Year", deserialize = "year"))]
            year: u16,
        }
    });
    assert!(output.contains(r#"& ["Title" , "Year"]"#), "{output}");
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
    assert!(
        output.starts_with(
            &quote!(impl<'a, T: Serialize> ::crumbkit::Row for Labelled<'a, T> where T: JsonSchema,)
                .to_string()
        ),
        "{output}"
    );
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
fn an_unknown_key_is_refused_with_the_field_name_and_the_accepted_keys() {
    let message = refusal(quote! {
        struct Book {
            #[row(colum)]
            id: u32,
        }
    });
    assert_eq!(
        message,
        "the field `id` gives `colum`, and #[row(...)] has no such key. Its keys are `column`, `long`, `uncut` and `problem` on a field, and `crate` on the type"
    );
}

#[test]
fn a_key_given_twice_is_refused() {
    let message = refusal(quote! {
        struct Book {
            #[row(column)]
            #[row(column)]
            id: u32,
        }
    });
    assert_eq!(message, "the field `id` gives `column` twice");
}

#[test]
fn a_field_with_both_column_and_long_is_refused() {
    let message = refusal(quote! {
        struct Book {
            #[row(column, long)]
            id: u32,
        }
    });
    assert_eq!(
        message,
        "the field `id` gives both `column` and `long`, and a field is in only one of the two lists"
    );
}

#[test]
fn uncut_without_column_or_long_is_refused() {
    let message = refusal(quote! {
        struct Book {
            #[row(uncut)]
            id: u32,
        }
    });
    assert_eq!(
        message,
        "the field `id` gives `uncut` without `column` or `long`, and only a column can be uncut"
    );
}

#[test]
fn a_row_key_on_a_field_that_serde_skips_or_flattens_is_refused() {
    for key in ["skip", "skip_serializing", "flatten"] {
        let key = syn::Ident::new(key, proc_macro2::Span::call_site());
        let message = refusal(quote! {
            struct Book {
                #[serde(#key)]
                #[row(long)]
                notes: String,
            }
        });
        assert_eq!(
            message,
            format!(
                "the field `notes` is marked #[serde({key})], so the JSON holds no field of that name, and it takes no #[row(...)] key"
            )
        );
    }
}

#[test]
fn a_problem_key_on_two_fields_is_refused_with_both_names() {
    let message = refusal(quote! {
        struct Check {
            #[row(column, problem)]
            error: Option<String>,
            #[row(column, problem)]
            warning: Option<String>,
        }
    });
    assert_eq!(
        message,
        "the fields `error` and `warning` both give `problem`, and a row has at most one problem field"
    );
}

#[test]
fn a_field_key_on_the_type_is_refused() {
    let message = refusal(quote! {
        #[row(column)]
        struct Book {
            id: u32,
        }
    });
    assert_eq!(
        message,
        "`column` goes in the #[row(...)] attribute of a field"
    );
}

#[test]
fn the_crate_key_on_a_field_is_refused() {
    let message = refusal(quote! {
        struct Book {
            #[row(crate = ::crumbkit)]
            id: u32,
        }
    });
    assert_eq!(
        message,
        "`crate` goes in the #[row(...)] attribute of the type"
    );
}

#[test]
fn an_enum_is_refused() {
    let message = refusal(quote! {
        enum State {
            Done,
        }
    });
    assert_eq!(
        message,
        "#[derive(Row)] supports a struct with named fields, and `State` is an enum, so it writes `impl Row` by hand"
    );
}

#[test]
fn a_union_is_refused() {
    let message = refusal(quote! {
        union Bits {
            whole: u32,
        }
    });
    assert_eq!(
        message,
        "#[derive(Row)] supports a struct with named fields, and `Bits` is a union"
    );
}

#[test]
fn a_tuple_struct_is_refused() {
    let message = refusal(quote! {
        struct Pair(u32, u32);
    });
    assert_eq!(
        message,
        "#[derive(Row)] supports a struct with named fields, and `Pair` is a tuple struct"
    );
}

#[test]
fn a_unit_struct_is_refused() {
    let message = refusal(quote! {
        struct Nothing;
    });
    assert_eq!(
        message,
        "#[derive(Row)] supports a struct with named fields, and `Nothing` is a unit struct"
    );
}

#[test]
fn an_unknown_rename_all_rule_is_refused() {
    let message = refusal(quote! {
        #[serde(rename_all = "Title Case")]
        struct Book {
            #[row(column)]
            id: u32,
        }
    });
    assert_eq!(
        message,
        "#[serde(rename_all)] has no rule \"Title Case\". Its rules are \"lowercase\", \"UPPERCASE\", \"PascalCase\", \"camelCase\", \"snake_case\", \"SCREAMING_SNAKE_CASE\", \"kebab-case\" and \"SCREAMING-KEBAB-CASE\""
    );
}
