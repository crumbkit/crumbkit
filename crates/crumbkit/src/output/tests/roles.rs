//! Tests for the role that each piece of a finished result carries to the
//! sink (ADR-0128).

use clap::CommandFactory as _;

use super::{PLANET_LISTING, PLANETS, Tool, mercury, parse_against, saturn};
use crate::output::respond::respond;
use crate::sink::role::{Message, Recorded, Role};
use crate::{Document, Error, Listing, Output, Rows};

/// Responds with `value` into `recorded` as `finish_to` does, and returns
/// the exit code.
fn responded<T: Rows>(
    recorded: &mut Recorded,
    mut output: Output,
    document: Document<T>,
    value: T,
) -> u8 {
    let (spelling, document) = output.named(document);
    let result: Result<T, Error> = Ok(value);
    respond(recorded, &mut output, spelling, document, result).0
}

#[test]
fn a_table_delivers_as_the_result_and_its_footer_as_a_note_after_it() {
    let mut recorded = Recorded::default();
    let output = parse_against(Tool::command(), &[]);
    let listing = Listing::new(vec![mercury(), saturn()], 8);

    responded(&mut recorded, output, PLANET_LISTING, listing);

    assert_eq!(
        recorded.roles(),
        [Role::Result, Role::Message(Message::Note)]
    );
    assert!(
        recorded.texts()[1].1.starts_with("Showing 2 of 8"),
        "{:?}",
        recorded.texts()
    );
}

#[test]
fn the_dry_run_line_delivers_as_a_dry_run_message_after_the_result() {
    let mut recorded = Recorded::default();
    let mut output = parse_against(Tool::command(), &[]);
    output.dry_run = true;

    responded(&mut recorded, output, PLANETS, vec![mercury()]);

    assert_eq!(
        recorded.texts(),
        [
            (Role::Result, "NAME     MOONS\nMercury  0\n".to_owned()),
            (
                Role::Message(Message::DryRun),
                "This was a dry run, so nothing changed.\n".to_owned()
            ),
        ]
    );
}

#[test]
fn a_text_table_fits_the_width_that_the_sink_gives_the_result() {
    let mut recorded = Recorded {
        width: Some(12),
        ..Recorded::default()
    };
    let output = parse_against(Tool::command(), &[]);

    responded(&mut recorded, output, PLANETS, vec![mercury()]);

    assert_eq!(
        recorded.texts(),
        [(Role::Result, "NAME   MOONS\nMerc…  0\n".to_owned())]
    );
}

#[test]
fn a_text_table_prints_unfitted_when_the_sink_gives_the_result_no_width() {
    let mut recorded = Recorded::default();
    let output = parse_against(Tool::command(), &[]);

    responded(&mut recorded, output, PLANETS, vec![mercury()]);

    assert_eq!(
        recorded.texts(),
        [(Role::Result, "NAME     MOONS\nMercury  0\n".to_owned())]
    );
}
