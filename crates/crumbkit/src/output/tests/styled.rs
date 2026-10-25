//! Tests for the styles that a row type gives its cells, which a text table
//! carries only when the sink asks for styles (ADR-0150).

use clap::CommandFactory as _;
use schemars::JsonSchema;
use serde::Serialize;

use std::process::ExitCode;

use super::{Finished, Tool, built_with};
use crate::help::Guide;
use crate::{Document, Error, Listing, Output, Row, Rows, Sink, Style, finish_to};

/// A service and its state, whose state cell is green when it runs, red
/// when it failed and dim when it stopped.
#[derive(Serialize, JsonSchema)]
struct Service {
    /// The service's name.
    name: String,
    /// The service's state.
    state: &'static str,
}

impl Row for Service {
    const COLUMNS: &'static [&'static str] = &["name", "state"];

    fn cell_style(&self, column: &str) -> Option<Style> {
        match (column, self.state) {
            ("name", _) => Some(Style::Emphasis),
            ("state", "running") => Some(Style::Success),
            ("state", "failed") => Some(Style::Error),
            ("state", "stopped") => Some(Style::Dim),
            _ => None,
        }
    }
}

/// The same service as [`Service`], with no styles.
#[derive(Serialize, JsonSchema)]
struct PlainService {
    /// The service's name.
    name: String,
    /// The service's state.
    state: &'static str,
}

impl Row for PlainService {
    const COLUMNS: &'static [&'static str] = &["name", "state"];
}

const SERVICES: Document<Vec<Service>> = Document::new("services", "services", 1, "The test rows.");
const PLAIN_SERVICES: Document<Vec<PlainService>> =
    Document::new("services", "plain-services", 1, "The test rows.");
const SERVICE_LISTING: Document<Listing<Service>> =
    Document::new("services", "service-listing", 1, "The test rows.");

/// The names and states of three services.
const STATES: [(&str, &str); 3] = [
    ("web", "running"),
    ("queue-worker", "failed"),
    ("cache", "stopped"),
];

/// Returns the three services of [`STATES`], with styles.
fn services() -> Vec<Service> {
    STATES
        .iter()
        .map(|&(name, state)| Service {
            name: name.to_owned(),
            state,
        })
        .collect()
}

/// Returns the three services of [`STATES`], without styles.
fn plain_services() -> Vec<PlainService> {
    STATES
        .iter()
        .map(|&(name, state)| PlainService {
            name: name.to_owned(),
            state,
        })
        .collect()
}

/// Returns the output of `tool list` with `flags` and `guide`.
fn built(guide: &Guide, flags: &[&str]) -> Output {
    let args: Vec<&str> = ["tool", "list"].iter().chain(flags).copied().collect();
    built_with(Tool::command(), guide, &args)
}

/// Finishes `value` with `output` into a sink whose result is `styled`, as
/// `finish` styles it at a terminal that allows colour, and that gives the
/// result `width` columns.
fn finish_into<T: Rows>(
    output: Output,
    document: Document<T>,
    value: T,
    styled: bool,
    width: Option<usize>,
) -> Finished {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err).styled(styled).fitted(width);
    let result: Result<T, Error> = Ok(value);
    let code = finish_to(&mut sink, output, document, result);
    Finished {
        code: ExitCode::from(code),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Finishes the three styled services with `flags` into a sink whose result
/// is `styled`.
fn finish_services(flags: &[&str], styled: bool) -> Finished {
    finish_into(
        built(&Guide::new(), flags),
        SERVICES,
        services(),
        styled,
        None,
    )
}

/// Returns `text` without its ANSI escape sequences, each of which starts
/// with `ESC [` and ends with the letter `m`.
fn strip(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("\u{1b}[") {
        plain.push_str(&rest[..start]);
        let end = rest[start..]
            .find('m')
            .map_or(rest.len(), |end| start + end + 1);
        rest = &rest[end..];
    }
    plain.push_str(rest);
    plain
}

#[test]
fn a_styled_sink_wraps_each_styled_cell_in_its_escape_codes_and_pads_after_the_reset() {
    let finished = finish_services(&[], true);

    let expected = "\
NAME          STATE
\u{1b}[1mweb\u{1b}[0m           \u{1b}[32mrunning\u{1b}[0m
\u{1b}[1mqueue-worker\u{1b}[0m  \u{1b}[1m\u{1b}[31mfailed\u{1b}[0m
\u{1b}[1mcache\u{1b}[0m         \u{1b}[2mstopped\u{1b}[0m
";
    assert_eq!(finished.out, expected);
    assert_eq!(finished.err, "");
}

#[test]
fn a_sink_without_styles_prints_the_bytes_of_a_row_type_that_styles_no_cell() {
    let styled_type = finish_services(&[], false);
    let plain_type = finish_into(
        built(&Guide::new(), &[]),
        PLAIN_SERVICES,
        plain_services(),
        false,
        None,
    );

    assert_eq!(styled_type.out, plain_type.out);
    assert!(!styled_type.out.contains('\u{1b}'), "{}", styled_type.out);
}

#[test]
fn a_styled_table_without_its_escape_codes_is_the_plain_table_with_the_same_alignment() {
    for flags in [&[][..], &["--fields", "state,name"][..]] {
        let styled = finish_services(flags, true);
        let plain = finish_services(flags, false);

        assert_ne!(styled.out, plain.out, "{flags:?} printed no styles");
        assert_eq!(strip(&styled.out), plain.out, "{flags:?}");
    }
}

#[test]
fn json_with_a_styled_sink_prints_the_same_bytes_as_without_one() {
    let styled = finish_services(&["--json"], true);
    let plain = finish_services(&["--json"], false);

    assert_eq!(styled.out, plain.out);
    assert!(!styled.out.contains('\u{1b}'), "{}", styled.out);
}

#[test]
fn a_fitted_styled_table_cuts_the_text_inside_the_escape_codes() {
    let narrow = finish_into(
        built(&Guide::new(), &[]),
        SERVICES,
        services(),
        true,
        Some(16),
    );
    let plain = finish_into(
        built(&Guide::new(), &[]),
        SERVICES,
        services(),
        false,
        Some(16),
    );

    assert!(
        narrow.out.contains("\u{1b}[1mqueue-…\u{1b}[0m"),
        "{}",
        narrow.out
    );
    assert_eq!(strip(&narrow.out), plain.out);
}

#[test]
fn the_output_budget_shows_the_same_rows_with_and_without_styles() {
    let guide = Guide::new().output_budget(1024, "the tests print little");
    let listing = || {
        let rows: Vec<Service> = (0..60)
            .map(|number| Service {
                name: format!("service-{number:02}"),
                state: "running",
            })
            .collect();
        Listing::new(rows, 60).described("services")
    };

    let styled = finish_into(built(&guide, &[]), SERVICE_LISTING, listing(), true, None);
    let plain = finish_into(built(&guide, &[]), SERVICE_LISTING, listing(), false, None);

    assert_eq!(strip(&styled.out), plain.out);
    assert_eq!(styled.err, plain.err);
    assert!(
        plain.err.starts_with("Showing "),
        "the budget cut no row: {}",
        plain.err
    );
}

/// A check of a service, whose error field reports its problem.
#[derive(Serialize, JsonSchema)]
struct Check {
    /// The service's name.
    name: &'static str,
    /// The error that the check found, if any.
    error: Option<&'static str>,
    /// The service's state.
    state: &'static str,
}

impl Row for Check {
    const COLUMNS: &'static [&'static str] = &["name", "error", "state"];
    const PROBLEM: Option<&'static str> = Some("error");

    fn cell_style(&self, column: &str) -> Option<Style> {
        match (column, self.state) {
            ("name", _) => Some(Style::Emphasis),
            ("error", _) | ("state", "failed") => Some(Style::Error),
            ("state", "running") => Some(Style::Success),
            ("state", _) => Some(Style::Dim),
            _ => None,
        }
    }
}

const CHECKS: Document<Listing<Check>> = Document::new("checks", "checks", 1, "The test checks.");
const PROBLEM_CHECKS: Document<Listing<Check>> =
    Document::new("checks", "problem-checks", 1, "The test checks.").only_problems();

/// Returns a check of `name` in `state` that found `error`.
const fn check(name: &'static str, error: Option<&'static str>, state: &'static str) -> Check {
    Check { name, error, state }
}

#[test]
fn a_document_that_lists_only_problems_styles_each_shown_row_with_its_own_styles() {
    // The healthy row comes first, so styles taken from the unfiltered rows
    // would give the failed row the healthy row's green state.
    let checks = vec![
        check("web", None, "running"),
        check("queue", Some("exit 1"), "failed"),
    ];

    let finished = finish_into(
        built(&Guide::new(), &[]),
        PROBLEM_CHECKS,
        Listing::from_all(checks, None),
        true,
        None,
    );

    let expected = "\
NAME   ERROR   STATE
\u{1b}[1mqueue\u{1b}[0m  \u{1b}[1m\u{1b}[31mexit 1\u{1b}[0m  \u{1b}[1m\u{1b}[31mfailed\u{1b}[0m
";
    assert_eq!(finished.out, expected);
    assert_eq!(finished.code, ExitCode::from(1));
}

#[test]
fn a_column_left_out_for_being_empty_takes_no_style_from_the_columns_beside_it() {
    let checks = vec![
        check("web", None, "running"),
        check("cache", None, "stopped"),
    ];

    let finished = finish_into(
        built(&Guide::new(), &[]),
        CHECKS,
        Listing::from_all(checks, None),
        true,
        None,
    );

    let expected = "\
NAME   STATE
\u{1b}[1mweb\u{1b}[0m    \u{1b}[32mrunning\u{1b}[0m
\u{1b}[1mcache\u{1b}[0m  \u{1b}[2mstopped\u{1b}[0m
";
    assert_eq!(finished.out, expected);
}

#[test]
fn the_placeholder_of_a_missing_value_takes_the_style_of_its_cell() {
    let checks = vec![
        check("queue", Some("exit 1"), "failed"),
        check("web", None, "running"),
    ];

    let finished = finish_into(
        built(&Guide::new(), &[]),
        CHECKS,
        Listing::from_all(checks, None),
        true,
        None,
    );

    assert!(
        finished
            .out
            .contains("\u{1b}[1mweb\u{1b}[0m    \u{1b}[1m\u{1b}[31m-\u{1b}[0m"),
        "{}",
        finished.out
    );
}
