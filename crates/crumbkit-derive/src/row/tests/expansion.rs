use quote::quote;

use super::expanded;

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
            damage: Option<String>,
            #[row(column, warning)]
            wear: Option<String>,
            isbn: String,
        }
    };
    let expected = quote! {
        #[automatically_derived]
        impl ::crumbkit::Row for Book {
            const COLUMNS: &'static [&'static str] = &["id", "name", "damage", "wear"];
            const LONG_COLUMNS: &'static [&'static str] = &["pageCount"];
            const UNCUT_COLUMNS: &'static [&'static str] = &["id"];
            const PROBLEM: ::std::option::Option<::crumbkit::Problem<Self>> =
                ::std::option::Option::Some(::crumbkit::Problem::field("damage"));
            const WARNING: ::std::option::Option<::crumbkit::Warning<Self>> =
                ::std::option::Option::Some(::crumbkit::Warning::field("wear"));
        }
    };
    assert_eq!(expanded(input), expected.to_string());
}

#[test]
fn the_derive_names_a_warning_field_in_warning() {
    let output = expanded(quote! {
        struct Book {
            #[row(column)]
            id: u32,
            #[row(column, warning)]
            wear: Option<String>,
        }
    });
    assert!(output.contains(r#"Warning :: field ("wear")"#), "{output}");
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
            const WARNING: ::std::option::Option<::crumbkit::Warning<Self>> =
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
