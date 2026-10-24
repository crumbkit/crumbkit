use super::{Pages, reading};

#[test]
fn a_page_count_reads_with_its_unit_in_text_and_as_a_number_in_json() {
    assert_eq!(Pages(432).to_string(), "432 pages");
    assert_eq!(serde_json::to_string(&Pages(432)).unwrap(), "432");
}

#[test]
fn the_reading_time_is_two_minutes_a_page() {
    assert_eq!(
        serde_json::to_string(&reading(432)).unwrap(),
        r#""PT14H24M""#
    );
}
