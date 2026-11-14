//! Tests for the CI time check.

use super::{Report, measure};

#[test]
fn a_median_of_exactly_240_seconds_passes() {
    let report = measure(&[100, 240, 400]);

    assert_eq!(
        report,
        Report::Median {
            runs: 3,
            seconds: 240
        }
    );
    assert!(report.passes());
}

#[test]
fn a_median_of_241_seconds_fails() {
    let report = measure(&[100, 241, 400]);

    assert_eq!(
        report,
        Report::Median {
            runs: 3,
            seconds: 241
        }
    );
    assert!(!report.passes());
}

#[test]
fn the_median_does_not_depend_on_the_order_of_the_runs() {
    assert_eq!(measure(&[400, 100, 250]), measure(&[100, 250, 400]));
}

#[test]
fn the_median_of_an_even_count_is_the_mean_of_the_middle_two() {
    assert_eq!(
        measure(&[140, 59, 70, 43]),
        Report::Median {
            runs: 4,
            seconds: 65
        }
    );
}

#[test]
fn a_median_between_240_and_241_seconds_rounds_up_and_fails() {
    let report = measure(&[240, 241]);

    assert_eq!(
        report,
        Report::Median {
            runs: 2,
            seconds: 241
        }
    );
    assert!(!report.passes());
}

#[test]
fn no_runs_at_all_fails() {
    let report = measure(&[]);

    assert_eq!(report, Report::NoRuns);
    assert!(!report.passes());
}

#[test]
fn a_report_prints_the_workflow_the_median_the_run_count_and_the_limit() {
    let report = Report::Median {
        runs: 20,
        seconds: 65,
    };

    assert_eq!(
        report.to_string(),
        ".github/workflows/ci.yml: 65 seconds median wall time of 20 pull-request runs, limit 240"
    );
}

#[test]
fn a_report_without_runs_says_there_is_nothing_to_measure() {
    assert_eq!(
        Report::NoRuns.to_string(),
        ".github/workflows/ci.yml: no successful pull-request runs to measure, limit 240 seconds"
    );
}
