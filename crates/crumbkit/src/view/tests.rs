use crumbkit_core::{Failure, Recovery};

use super::{Selection, Table, View};
use crate::ViewFlags;

fn view(flags: ViewFlags) -> View {
    View::try_from(flags).unwrap()
}

fn refused(flags: ViewFlags) -> (String, String) {
    let refusal = View::try_from(flags).unwrap_err();
    let Recovery::Hint(hint) = refusal.recovery() else {
        panic!("the pairing refusal carries a hint");
    };
    (refusal.to_string(), hint)
}

#[test]
fn no_flags_select_the_compact_text_table() {
    assert_eq!(view(ViewFlags::new()), View::Text(Table::Compact));
}

#[test]
fn long_alone_selects_the_long_text_table() {
    assert_eq!(view(ViewFlags::new().long(true)), View::Text(Table::Long));
}

#[test]
fn fields_alone_select_the_named_text_columns() {
    let flags = ViewFlags::new().fields(vec!["name".to_owned(), "year".to_owned()]);
    assert_eq!(
        view(flags),
        View::Text(Table::Named(vec!["name".to_owned(), "year".to_owned()]))
    );
}

#[test]
fn json_alone_selects_the_default_columns() {
    assert_eq!(
        view(ViewFlags::new().json(true)),
        View::Json(Selection::Columns)
    );
}

#[test]
fn json_with_long_selects_the_long_columns() {
    let flags = ViewFlags::new().json(true).long(true);
    assert_eq!(view(flags), View::Json(Selection::Long));
}

#[test]
fn json_with_full_selects_every_field() {
    let flags = ViewFlags::new().json(true).full(true);
    assert_eq!(view(flags), View::Json(Selection::Full));
}

#[test]
fn json_with_long_and_full_selects_every_field() {
    let flags = ViewFlags::new().json(true).long(true).full(true);
    assert_eq!(view(flags), View::Json(Selection::Full));
}

#[test]
fn json_with_fields_selects_the_named_fields() {
    let flags = ViewFlags::new().json(true).fields(vec!["name".to_owned()]);
    assert_eq!(
        view(flags),
        View::Json(Selection::Named(vec!["name".to_owned()]))
    );
}

#[test]
fn fields_with_long_are_refused() {
    let (message, hint) = refused(ViewFlags::new().fields(vec!["name".to_owned()]).long(true));
    assert_eq!(
        message,
        "--fields prints only the fields it names, so it cannot be used with --long"
    );
    assert_eq!(hint, "drop --long, and name the long columns in --fields");
}

#[test]
fn fields_with_full_are_refused() {
    let flags = ViewFlags::new()
        .json(true)
        .fields(vec!["name".to_owned()])
        .full(true);
    let (message, hint) = refused(flags);
    assert_eq!(
        message,
        "--fields prints only the fields it names, so it cannot be used with --full"
    );
    assert_eq!(
        hint,
        "drop --full to print only the named fields, or drop --fields to print every field"
    );
}

#[test]
fn full_without_json_is_refused() {
    let (message, hint) = refused(ViewFlags::new().full(true));
    assert_eq!(
        message,
        "--full prints every field of every row in JSON, so it needs --json"
    );
    assert_eq!(hint, "add --json, or drop --full");
}

#[test]
fn fields_with_long_are_refused_before_fields_with_full() {
    let flags = ViewFlags::new()
        .fields(vec!["name".to_owned()])
        .long(true)
        .full(true);
    let (message, _) = refused(flags);
    assert!(message.ends_with("with --long"), "{message}");
}

#[test]
fn the_view_reports_json_long_full_and_the_names() {
    let text = View::Text(Table::Long);
    assert!(!text.is_json());
    assert!(text.is_long());
    assert!(!text.is_full());
    assert_eq!(text.names(), None);
    assert_eq!(text.fields(), Vec::<String>::new());

    let full = View::Json(Selection::Full);
    assert!(full.is_json());
    assert!(!full.is_long());
    assert!(full.is_full());

    let named = View::Json(Selection::Named(vec!["name".to_owned()]));
    assert_eq!(named.names(), Some(["name".to_owned()].as_slice()));
    assert_eq!(named.fields(), ["name".to_owned()].as_slice());
}
