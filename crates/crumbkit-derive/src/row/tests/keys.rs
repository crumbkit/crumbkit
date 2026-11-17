use quote::quote;

use super::expanded;

#[test]
fn a_children_key_writes_children_field_with_each_option_it_gives() {
    let output = expanded(quote! {
        struct Shelf {
            #[row(column)]
            id: u32,
            #[row(children(indent = 4, spaced, help = "the copies", lines = Self::copies, long_lines = |row: &Self| row.copies()))]
            #[serde(rename = "copies")]
            lines: Vec<String>,
        }
    });
    let expected = quote! {
        const CHILDREN: ::std::option::Option<::crumbkit::Children<Self>> =
            ::std::option::Option::Some(
                ::crumbkit::Children::field("copies")
                    .indent(4)
                    .spaced()
                    .help("the copies")
                    .lines(Self::copies)
                    .long_lines(|row: &Self| row.copies())
            );
    };
    assert!(output.contains(&expected.to_string()), "{output}");
}

#[test]
fn a_bare_children_key_writes_children_field_alone() {
    let output = expanded(quote! { struct Shelf { #[row(children)] lines: Vec<String> } });
    let expected = quote! { ::crumbkit::Children::field("lines") };
    assert!(output.contains(&expected.to_string()), "{output}");
    assert!(!output.contains("indent"), "{output}");
}

#[test]
fn a_whole_key_on_the_type_writes_whole_as_true() {
    let output = expanded(quote! {
        #[row(whole)]
        struct Shelf {
            #[row(column)]
            id: u32,
        }
    });
    let expected = quote! { const WHOLE: bool = true; };
    assert!(output.contains(&expected.to_string()), "{output}");
}

#[test]
fn a_type_without_whole_or_children_writes_neither_constant() {
    let output = expanded(quote! { struct Shelf { #[row(column)] id: u32 } });
    assert!(!output.contains("WHOLE"), "{output}");
    assert!(!output.contains("CHILDREN"), "{output}");
}

#[test]
fn a_problem_by_key_writes_problem_by_with_the_fields_serialized_name() {
    let output = expanded(quote! {
        #[serde(rename_all = "camelCase")]
        struct Drafted {
            #[row(column, problem_by = |row: &Self| row.would_change)]
            would_change: bool,
        }
    });
    let expected = quote! {
        ::std::option::Option::Some(::crumbkit::Problem::by("wouldChange", |row: &Self| row.would_change))
    };
    assert!(output.contains(&expected.to_string()), "{output}");
}

#[test]
fn a_warning_by_key_writes_warning_by_with_the_fields_serialized_name() {
    let output = expanded(quote! {
        struct Drafted {
            #[row(column)]
            #[serde(rename = "state")]
            status: String,
            #[row(column, warning_by = check)]
            #[serde(rename = "wear")]
            damage: u32,
        }
    });
    let expected = quote! {
        ::std::option::Option::Some(::crumbkit::Warning::by("wear", check))
    };
    assert!(output.contains(&expected.to_string()), "{output}");
}
