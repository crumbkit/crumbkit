//! Tests for the run time check.

use std::time::Duration;

use super::{Timing, measure};

/// Returns `ms` milliseconds.
fn ms(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

#[test]
fn a_median_of_exactly_30_ms_passes() {
    let timing = measure("books list", &[ms(1), ms(30), ms(90)]);

    assert_eq!(timing.median, ms(30));
    assert!(timing.passes());
}

#[test]
fn a_median_just_over_30_ms_fails() {
    let timing = measure(
        "books list",
        &[ms(1), ms(30) + Duration::from_nanos(1), ms(90)],
    );

    assert!(!timing.passes());
}

#[test]
fn a_median_of_exactly_10_ms_is_within_the_budget() {
    assert!(!measure("books list", &[ms(10)]).over_budget());
}

#[test]
fn a_median_just_over_10_ms_is_over_the_budget_and_still_passes() {
    let timing = measure("books list", &[ms(10) + Duration::from_nanos(1)]);

    assert!(timing.over_budget());
    assert!(timing.passes());
}

#[test]
fn the_median_does_not_depend_on_the_order_of_the_runs() {
    assert_eq!(
        measure("books list", &[ms(9), ms(2), ms(5)]),
        measure("books list", &[ms(2), ms(5), ms(9)])
    );
}

#[test]
fn the_median_of_an_even_count_is_the_mean_of_the_middle_two() {
    assert_eq!(
        measure("books list", &[ms(8), ms(1), ms(3), ms(4)]).median,
        Duration::from_micros(3500)
    );
}

#[test]
fn the_95th_percentile_is_the_nearest_rank() {
    let runs: Vec<Duration> = (1..=200).map(ms).collect();

    assert_eq!(measure("books list", &runs).p95, ms(190));
    assert_eq!(measure("books list", &[ms(1), ms(2)]).p95, ms(2));
    assert_eq!(measure("books list", &[ms(4)]).p95, ms(4));
}

#[test]
fn no_runs_at_all_fails() {
    let timing = measure("books list", &[]);

    assert_eq!(timing.runs, 0);
    assert!(!timing.passes());
}

#[test]
fn a_timing_prints_the_command_the_median_the_95th_percentile_and_the_limits() {
    let timing = Timing {
        command: "books list".to_owned(),
        runs: 200,
        median: Duration::from_micros(3_240),
        p95: Duration::from_micros(4_010),
    };

    assert_eq!(
        timing.to_string(),
        "shelf books list: 3.2 ms median and 4.0 ms 95th percentile wall time of 200 runs, budget 10 ms, limit 30 ms"
    );
}

#[test]
fn a_timing_over_the_budget_says_so() {
    let timing = Timing {
        command: "books list".to_owned(),
        runs: 200,
        median: ms(12),
        p95: ms(15),
    };

    assert_eq!(
        timing.to_string(),
        "shelf books list: 12.0 ms median and 15.0 ms 95th percentile wall time of 200 runs, over the budget of 10 ms, limit 30 ms"
    );
}

#[test]
fn a_timing_without_runs_says_there_is_nothing_to_measure() {
    assert_eq!(
        measure("books list", &[]).to_string(),
        "shelf books list: no runs to measure, limit 30 ms"
    );
}
