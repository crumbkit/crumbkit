use super::__row_columns;

#[test]
fn the_columns_helper_concatenates_lists_in_order() {
    const JOINED: [&str; 4] = __row_columns(&[&["path"], &["a", "b"], &[], &["size"]]);
    assert_eq!(JOINED, ["path", "a", "b", "size"]);
}

#[test]
#[should_panic(expected = "fewer names")]
fn the_columns_helper_panics_when_the_length_is_larger_than_the_names() {
    let _ = __row_columns::<3>(&[&["path"], &["a"]]);
}

#[test]
#[should_panic(expected = "more names")]
fn the_columns_helper_panics_when_the_length_is_smaller_than_the_names() {
    let _ = __row_columns::<1>(&[&["path"], &["a"]]);
}
