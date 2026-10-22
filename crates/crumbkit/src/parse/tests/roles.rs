//! Tests for the role that help, the version and a picked run's echo carry
//! to the sink (ADR-0128).

use std::ffi::OsString;

use clap::Command;

use super::super::page::print_page;
use super::super::pick::echo;
use crate::sink::role::{Message, Recorded, Role};

/// Returns the error that clap stops with for `tool -h`, which holds the
/// help page.
fn help_request() -> clap::Error {
    Command::new("tool")
        .about("Counts planets.")
        .try_get_matches_from(["tool", "-h"])
        .unwrap_err()
}

#[test]
fn a_help_page_delivers_as_the_result() {
    let mut recorded = Recorded::default();

    let code = print_page(&mut recorded, &help_request(), false);

    assert_eq!(code, 0);
    assert_eq!(recorded.roles(), [Role::Result]);
    assert!(
        recorded.texts()[0].1.starts_with("Counts planets.\n"),
        "{:?}",
        recorded.texts()
    );
}

#[test]
fn a_help_page_prints_plain_when_the_sink_wants_no_styles_for_the_result() {
    let mut recorded = Recorded::default();

    print_page(&mut recorded, &help_request(), false);

    assert!(
        !recorded.texts()[0].1.contains('\u{1b}'),
        "{:?}",
        recorded.texts()
    );
}

/// clap writes a page's styles only with its `color` feature, which
/// crumbkit's `color` feature turns on.
#[cfg(feature = "color")]
#[test]
fn a_help_page_keeps_clap_styles_when_the_sink_wants_styles_for_the_result() {
    let mut recorded = Recorded {
        styled: true,
        ..Recorded::default()
    };

    print_page(&mut recorded, &help_request(), false);

    assert!(
        recorded.texts()[0].1.contains('\u{1b}'),
        "{:?}",
        recorded.texts()
    );
}

#[test]
fn the_echo_of_a_picked_run_delivers_as_an_echo_message() {
    let mut recorded = Recorded::default();
    let words = [OsString::from("books"), OsString::from("show")];

    echo(&mut recorded, &Command::new("shelf"), &words);

    assert_eq!(
        recorded.texts(),
        [(
            Role::Message(Message::Echo),
            "Running: shelf books show\n".to_owned()
        )]
    );
}
