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

#[test]
fn a_flatten_key_splices_the_field_types_columns_at_the_fields_position() {
    let output = expanded(quote! {
        struct File {
            #[row(column)]
            path: String,
            #[serde(flatten)]
            #[row(flatten)]
            stats: Stats,
            #[row(column, uncut)]
            id: u32,
            #[row(long)]
            owner: String,
        }
    });
    let columns = quote! {
        const __COLUMNS: [&'static str; 2 + <Stats as ::crumbkit::Row>::COLUMNS.len()] =
            ::crumbkit::__row_columns(&[
                &["path"],
                <Stats as ::crumbkit::Row>::COLUMNS,
                &["id"]
            ]);
    };
    let long = quote! {
        const __LONG_COLUMNS: [&'static str; 1 + <Stats as ::crumbkit::Row>::LONG_COLUMNS.len()] =
            ::crumbkit::__row_columns(&[
                <Stats as ::crumbkit::Row>::LONG_COLUMNS,
                &["owner"]
            ]);
    };
    let uncut = quote! {
        const __UNCUT_COLUMNS: [&'static str; 1 + <Stats as ::crumbkit::Row>::UNCUT_COLUMNS.len()] =
            ::crumbkit::__row_columns(&[
                <Stats as ::crumbkit::Row>::UNCUT_COLUMNS,
                &["id"]
            ]);
    };
    for expected in [columns, long, uncut] {
        assert!(output.contains(&expected.to_string()), "{output}");
    }
    let uses = quote! {
        const COLUMNS: &'static [&'static str] = &__COLUMNS;
        const LONG_COLUMNS: &'static [&'static str] = &__LONG_COLUMNS;
        const UNCUT_COLUMNS: &'static [&'static str] = &__UNCUT_COLUMNS;
    };
    assert!(output.contains(&uses.to_string()), "{output}");
    assert!(output.starts_with("const _ : () = {"), "{output}");
}

#[test]
fn a_flatten_key_asserts_that_the_field_type_sets_no_problem_warning_or_children() {
    let output = expanded(quote! {
        struct File {
            #[serde(flatten)]
            #[row(flatten)]
            stats: Stats,
        }
    });
    for item in ["PROBLEM", "WARNING", "CHILDREN"] {
        let text = format!("< Stats as :: crumbkit :: Row > :: {item} . is_none ()");
        assert!(output.contains(&text), "{item}: {output}");
        assert!(
            output.contains(&format!(
                "the flattened type `Stats` sets {item}, and the row"
            )),
            "{item}: {output}"
        );
    }
}

#[test]
fn a_row_without_flatten_writes_no_helper_constants() {
    let output = expanded(quote! { struct File { #[row(column)] path: String } });
    assert!(!output.contains("__row_columns"), "{output}");
    assert!(!output.contains("const _"), "{output}");
}

#[test]
fn a_remote_key_writes_the_impl_for_the_named_type_and_uses_the_mirror() {
    let output = expanded(quote! {
        #[serde(rename_all = "camelCase", transparent)]
        #[row(remote = Printed<RunReport>)]
        struct RunReportRow {
            #[row(column)]
            run_name: String,
            #[row(long)]
            item_count: u32,
            hidden: bool,
        }
    });
    let expected = quote! {
        #[automatically_derived]
        impl ::crumbkit::Row for Printed<RunReport> {
            const COLUMNS: &'static [&'static str] = &["runName"];
            const LONG_COLUMNS: &'static [&'static str] = &["itemCount"];
            const UNCUT_COLUMNS: &'static [&'static str] = &[];
            const PROBLEM: ::std::option::Option<::crumbkit::Problem<Self>> =
                ::std::option::Option::None;
            const WARNING: ::std::option::Option<::crumbkit::Warning<Self>> =
                ::std::option::Option::None;
        }
        const _: fn(RunReportRow) -> RunReportRow = |row| RunReportRow {
            run_name: row.run_name,
            item_count: row.item_count,
            hidden: row.hidden
        };
    };
    assert_eq!(output, expected.to_string());
}
