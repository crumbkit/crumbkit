use proc_macro2::TokenStream;
use quote::quote;

use super::refusal;

#[test]
fn each_refused_warning_use_is_a_compile_error_that_names_the_field() {
    let cases: Vec<(TokenStream, &str)> = vec![
        (
            quote! {
                struct Check {
                    #[row(column, warning)] wear: Option<String>,
                    #[row(column, warning)] tear: Option<String>,
                }
            },
            "the fields `wear` and `tear` both give `warning`, and a row has at most one warning field",
        ),
        (
            quote! { struct Check { #[row(long, warning)] wear: Option<String> } },
            "the field `wear` gives `warning` without `column`, and the warning field is one of the compact columns",
        ),
        (
            quote! { struct Check { #[row(column, problem, warning)] wear: Option<String> } },
            "the field `wear` gives both `problem` and `warning`, and a row that reports a problem counts only as a problem",
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(refusal(input), expected);
    }
}

#[test]
fn the_derive_refuses_problem_and_warning_on_one_field() {
    let message = refusal(quote! {
        struct Book {
            #[row(column, problem, warning)]
            damage: Option<String>,
        }
    });
    assert!(
        message.contains("gives both `problem` and `warning`"),
        "{message}"
    );
}

#[test]
fn each_refused_use_is_a_compile_error_that_names_the_field_or_the_type() {
    let dropped = |key: &str| {
        let except = if key == "flatten" {
            " but `flatten`"
        } else {
            ""
        };
        format!(
            "the field `notes` is marked #[serde({key})], so the JSON holds no field of that name, and it takes no #[row(...)] key{except}"
        )
    };
    let keys = "Its keys are `column`, `long`, `uncut`, `problem`, `problem_by`, `warning`, `warning_by`, `children` and `flatten` on a field, and `crate`, `whole` and `remote` on the type";
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
                    #[row(column, problem)] damage: Option<String>,
                }
            },
            "the fields `error` and `damage` both give `problem`, and a row has at most one problem field".to_owned(),
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

#[test]
fn each_new_refused_use_is_a_compile_error_that_names_the_field_or_the_type() {
    let cases = [
        (
            quote! { struct Book { #[row(column, problem, problem_by = |row: &Self| row.id == 0)] id: u32 } },
            "the field `id` gives both `problem` and `problem_by`, and a field names how it decides once",
        ),
        (
            quote! { struct Book { #[row(column, warning, warning_by = |row: &Self| row.id == 0)] id: u32 } },
            "the field `id` gives both `warning` and `warning_by`, and a field names how it decides once",
        ),
        (
            quote! {
                struct Book {
                    #[row(column, problem)] error: Option<String>,
                    #[row(column, problem_by = |row: &Self| row.id == 0)] id: u32,
                }
            },
            "the fields `error` and `id` give `problem` and `problem_by`, and a row has at most one problem field",
        ),
        (
            quote! {
                struct Book {
                    #[row(column, warning_by = |row: &Self| row.id == 0)] id: u32,
                    #[row(column, warning_by = |row: &Self| row.id == 1)] other: u32,
                }
            },
            "the fields `id` and `other` both give `warning_by`, and a row has at most one warning field",
        ),
        (
            quote! { struct Book { #[row(long, problem_by = |row: &Self| row.id == 0)] id: u32 } },
            "the field `id` gives `problem_by` without `column`, and the problem field is one of the compact columns",
        ),
        (
            quote! { struct Book { #[row(long, warning_by = |row: &Self| row.id == 0)] id: u32 } },
            "the field `id` gives `warning_by` without `column`, and the warning field is one of the compact columns",
        ),
        (
            quote! {
                struct Book {
                    #[row(column, problem_by = |row: &Self| row.id == 0, warning)] id: u32,
                }
            },
            "the field `id` gives both `problem_by` and `warning`, and a row that reports a problem counts only as a problem",
        ),
        (
            quote! { struct Book { #[row(column, children)] lines: Vec<String> } },
            "the field `lines` gives `children` and `column`, and a field that holds the child lines is in no column",
        ),
        (
            quote! { struct Book { #[row(children, warning_by = |row: &Self| true)] lines: Vec<String> } },
            "the field `lines` gives `children` and `warning_by`, and a field that holds the child lines is in no column",
        ),
        (
            quote! {
                struct Book {
                    #[row(children)] lines: Vec<String>,
                    #[row(children)] notes: Vec<String>,
                }
            },
            "the fields `lines` and `notes` both give `children`, and a row has at most one children field",
        ),
        (
            quote! { struct Book { #[row(children(colour = 1))] lines: Vec<String> } },
            "the field `lines` gives the `children` option `colour`, and `children` has no such option. Its options are `indent`, `spaced`, `help`, `lines` and `long_lines`",
        ),
        (
            quote! { struct Book { #[row(children(indent = 2, indent = 4))] lines: Vec<String> } },
            "the field `lines` gives the `children` option `indent` twice",
        ),
        (
            quote! { struct Book { #[row(children)] #[row(children)] lines: Vec<String> } },
            "the field `lines` gives `children` twice",
        ),
        (
            quote! { #[row(whole, whole)] struct Book { id: u32 } },
            "the type gives `whole` twice",
        ),
        (
            quote! { struct Book { #[row(whole)] id: u32 } },
            "`whole` goes in the #[row(...)] attribute of the type",
        ),
        (
            quote! { #[row(children)] struct Book { id: u32 } },
            "`children` goes in the #[row(...)] attribute of a field",
        ),
    ];
    for (input, message) in cases {
        assert_eq!(refusal(input.clone()), message, "{input}");
    }
}

#[test]
fn each_refused_flatten_use_is_a_compile_error_that_names_the_field() {
    let cases = [
        (
            quote! { struct File { #[row(flatten)] stats: Stats } },
            "the field `stats` gives `flatten` without #[serde(flatten)], and the columns of its type are columns of the row only when serde flattens the field",
        ),
        (
            quote! { struct File { #[serde(flatten)] #[row(flatten, column)] stats: Stats } },
            "the field `stats` gives `flatten` and `column`, and a flattened field takes the columns of its type and no other key",
        ),
        (
            quote! { struct File { #[serde(flatten)] #[row(flatten, children)] stats: Stats } },
            "the field `stats` gives `flatten` and `children`, and a flattened field takes the columns of its type and no other key",
        ),
        (
            quote! { struct File<T> { #[serde(flatten)] #[row(flatten)] stats: T } },
            "the field `stats` gives `flatten` on a generic type, and the columns of a flattened type cannot depend on the type's parameters",
        ),
        (
            quote! { struct File { #[row(flatten)] #[row(flatten)] stats: Stats } },
            "the field `stats` gives `flatten` twice",
        ),
    ];
    for (input, message) in cases {
        assert_eq!(refusal(input.clone()), message, "{input}");
    }
}

#[test]
fn each_refused_remote_use_is_a_compile_error_that_names_the_type_or_the_field() {
    let cases = [
        (
            quote! { #[row(remote = Printed<Report>)] struct Mirror<T> { #[row(column)] name: T } },
            "the type `Mirror` is generic and gives `remote`, and a mirror of a foreign type has no type parameters",
        ),
        (
            quote! { #[row(remote = A, remote = B)] struct Mirror { #[row(column)] name: String } },
            "the type gives `remote` twice",
        ),
        (
            quote! { struct Mirror { #[row(remote = A)] name: String } },
            "`remote` goes in the #[row(...)] attribute of the type",
        ),
        (
            quote! { #[serde(transparent)] struct Mirror { #[row(column)] name: String } },
            "the type `Mirror` is marked #[serde(transparent)], so its JSON has no field names for #[derive(Row)] to list",
        ),
    ];
    for (input, message) in cases {
        assert_eq!(refusal(input.clone()), message, "{input}");
    }
}
