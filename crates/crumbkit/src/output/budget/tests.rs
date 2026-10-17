//! Tests for the variable that lowers the budget, the buffer that holds a
//! result that fits and the search for the rows that fit.

use std::ffi::OsStr;
use std::io::Write;

use super::{Over, OverBudget, buffered, largest, lowered, write_cut_line};

#[test]
fn the_budget_variable_lowers_the_budget_and_never_raises_it() {
    assert_eq!(lowered(1_000, Some(OsStr::new("200"))), 200);
    assert_eq!(lowered(1_000, Some(OsStr::new("5000"))), 1_000);
}

#[test]
fn an_unset_empty_zero_or_wordy_budget_variable_leaves_the_budget_unchanged() {
    for value in [
        None,
        Some(""),
        Some("0"),
        Some("lots"),
        Some("-5"),
        Some("1e3"),
    ] {
        assert_eq!(lowered(1_000, value.map(OsStr::new)), 1_000, "{value:?}");
    }
}

#[test]
fn the_buffer_returns_every_byte_of_a_result_that_fits_its_budget() {
    let bytes = buffered(10, |buffer| buffer.write_all(b"0123456789")).unwrap();

    assert_eq!(bytes.as_deref(), Some(&b"0123456789"[..]));
}

#[test]
fn the_buffer_returns_nothing_and_stops_the_writer_when_a_result_passes_its_budget() {
    let mut writes = 0;
    let bytes = buffered(10, |buffer| {
        for _ in 0..5 {
            writes += 1;
            buffer.write_all(b"0123")?;
        }
        Ok(())
    })
    .unwrap();

    assert_eq!(bytes, None);
    assert_eq!(writes, 3);
}

#[test]
fn the_search_finds_the_largest_count_that_fits() {
    let fits = |limit: usize| move |count: usize| Ok(count * 10 <= limit);

    assert_eq!(largest(0, 9, fits(55)).unwrap(), 5);
    assert_eq!(largest(0, 9, fits(1_000)).unwrap(), 9);
    assert_eq!(largest(0, 9, fits(0)).unwrap(), 0);
}

#[test]
fn the_search_returns_its_lowest_count_when_no_count_fits() {
    assert_eq!(largest(1, 9, |_| Ok(false)).unwrap(), 1);
}

#[test]
fn the_cut_line_of_a_listing_names_limit_and_of_a_list_only_fields() {
    let mut listing = Vec::new();
    let mut list = Vec::new();

    write_cut_line(&mut listing, 3, Some(50), Some("books"), 400, true).unwrap();
    write_cut_line(&mut list, 3, Some(7), None, 400, false).unwrap();

    assert_eq!(
        String::from_utf8(listing).unwrap(),
        "Showing 3 of 50 books, because output stops at 400 bytes. Ask for less with --limit or --fields.\n"
    );
    assert_eq!(
        String::from_utf8(list).unwrap(),
        "Showing 3 of 7, because output stops at 400 bytes. Ask for less with --fields.\n"
    );
}

#[test]
fn a_budget_of_1_is_named_as_1_byte_in_the_cut_line_and_in_each_over_budget_message() {
    let mut line = Vec::new();
    write_cut_line(&mut line, 0, Some(5), Some("books"), 1, true).unwrap();
    let message = |over: Over| {
        let code = &crate::codes::OUTPUT_OVER_BUDGET;
        OverBudget {
            code,
            budget: 1,
            over,
        }
        .to_string()
    };
    let batch = Over::Batch {
        left_out: vec!["b".to_owned()],
        from: Some(1),
        unlisted: 0,
        fetch: None,
    };

    assert_eq!(
        String::from_utf8(line).unwrap(),
        "Showing 0 of 5 books, because output stops at 1 byte. Ask for less with --limit or --fields.\n"
    );
    assert_eq!(
        message(batch),
        "1 more answer did not fit in the output budget of 1 byte"
    );
    assert_eq!(
        message(Over::Whole),
        "the result is larger than the output budget of 1 byte"
    );
    assert_eq!(
        message(Over::Stream),
        "the stream stopped at the output budget of 1 byte"
    );
}
