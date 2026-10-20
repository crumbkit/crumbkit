//! Tests for the role that each piece of a printed failure carries to the
//! sink (ADR-0128).

use clap::builder::StyledStr;

use super::super::{Printing, print};
use super::{Missing, missing};
use crate::refusal::Refusal;
use crate::sink::role::{Message, Recorded, Role};
use crate::{FieldValue, Recovery, Spelling};

/// Prints `failure` into a recording sink in the form that `json` chooses.
fn printed(failure: &Missing, json: bool) -> Recorded {
    let mut recorded = Recorded::default();
    let command = &mut clap::Command::new("tool");
    print(
        &mut recorded,
        json,
        Spelling::COLONS,
        Printing::default(),
        failure,
        command,
    );
    recorded
}

/// Prints `refusal` into a recording sink in the form that `json` chooses.
fn refused(refusal: &Refusal, json: bool) -> Recorded {
    let mut recorded = Recorded::default();
    let command = &mut clap::Command::new("tool");
    refusal.print(
        &mut recorded,
        json,
        Spelling::COLONS,
        Printing::default(),
        command,
    );
    recorded
}

/// Returns the refusal of a bad `--fields` value.
fn bad_fields() -> Refusal {
    Refusal::flag_value(
        "--fields names an unknown field 'radius'".to_owned(),
        "use the valid fields: name".to_owned(),
        "--fields",
        Some("radius".to_owned()),
    )
}

#[test]
fn a_failure_in_json_form_delivers_as_the_result() {
    let recorded = printed(&missing(Recovery::Retry), true);

    assert_eq!(recorded.roles(), [Role::Result]);
    assert!(
        recorded.texts()[0].1.starts_with(r#"{"schema":"#),
        "{:?}",
        recorded.texts()
    );
}

#[test]
fn a_failure_in_text_form_delivers_as_a_failure_message() {
    let recorded = printed(&missing(Recovery::Retry), false);

    assert_eq!(recorded.roles(), [Role::Message(Message::Failure)]);
    assert!(
        recorded.texts()[0].1.starts_with("error: "),
        "{:?}",
        recorded.texts()
    );
}

#[test]
fn a_debug_warning_delivers_before_the_failure_it_warns_about() {
    let mut failure = missing(Recovery::Retry);
    failure.fields = vec![("Bad-Name", FieldValue::from("x"))];

    let recorded = printed(&failure, false);

    assert_eq!(
        recorded.roles(),
        [
            Role::Message(Message::DebugWarning),
            Role::Message(Message::Failure)
        ]
    );
    assert!(
        recorded.texts()[0].1.starts_with("warning: "),
        "{:?}",
        recorded.texts()
    );
}

#[test]
fn a_refusal_in_text_form_delivers_as_a_refusal_message() {
    let recorded = refused(&bad_fields(), false);

    assert_eq!(recorded.roles(), [Role::Message(Message::Refusal)]);
}

#[test]
fn a_refusal_in_json_form_delivers_as_the_result() {
    let recorded = refused(&bad_fields(), true);

    assert_eq!(recorded.roles(), [Role::Result]);
}

#[test]
fn a_refusals_group_page_delivers_as_refusal_help_in_place_of_its_lines() {
    let page = StyledStr::from("Usage: tool planets <COMMAND>\n");
    let refusal = bad_fields().shown_with(Vec::new(), Some(page));

    let recorded = refused(&refusal, false);

    assert_eq!(
        recorded.texts(),
        [(
            Role::Message(Message::RefusalHelp),
            "Usage: tool planets <COMMAND>\n".to_owned()
        )]
    );
}

/// clap writes a page's styles only with its `color` feature, which
/// crumbkit's `color` feature turns on.
#[cfg(feature = "color")]
#[test]
fn a_refusals_group_page_prints_plain_although_the_result_is_styled() {
    let mut page = StyledStr::new();
    write_bold(&mut page, "Usage:");
    let refusal = bad_fields().shown_with(Vec::new(), Some(page));
    let mut recorded = Recorded {
        styled: true,
        ..Recorded::default()
    };

    let command = &mut clap::Command::new("tool");
    refusal.print(
        &mut recorded,
        false,
        Spelling::COLONS,
        Printing::default(),
        command,
    );

    assert_eq!(recorded.texts()[0].1, "Usage:");
}

/// Appends `text` to `page` in bold, as clap writes a heading.
#[cfg(feature = "color")]
fn write_bold(page: &mut StyledStr, text: &str) {
    use std::fmt::Write as _;
    let bold = clap::builder::styling::Style::new().bold();
    let _ = write!(page, "{bold}{text}{bold:#}");
}
