//! Tests for the styles of a refusal that `Refused::print_to` and
//! `Refused::print` print (ADR-0131).

use super::refused;
#[cfg(feature = "color")]
use super::{refused_by, tool};
use crate::Sink;
#[cfg(feature = "color")]
use crate::help::Guide;

/// Prints the refusal of `tool planets show mars --srt name` with
/// `print_to` into a sink whose error writer wants styles when `styled` is
/// set, and returns what it printed on standard error.
fn printed_to(styled: bool) -> String {
    let refused = refused(&["tool", "planets", "show", "mars", "--srt", "name"]);
    let mut err = Vec::new();
    let _stop = refused.print_to(&mut Sink::new(Vec::new(), &mut err).styled_errors(styled));
    String::from_utf8(err).unwrap()
}

/// clap writes styles only with its `color` feature, which crumbkit's
/// `color` feature turns on.
#[cfg(feature = "color")]
#[test]
fn print_to_writes_clap_s_styles_to_a_sink_whose_error_writer_wants_them() {
    let palette = clap::builder::styling::Styles::styled();
    let error = palette.get_error();

    let styled = printed_to(true);

    assert!(
        styled.starts_with(&format!("{error}error:{error:#} ")),
        "{styled:?}"
    );
}

#[test]
fn print_to_writes_plain_text_to_a_sink_whose_error_writer_wants_no_styles() {
    let plain = printed_to(false);

    assert!(plain.starts_with("error: "), "{plain:?}");
    assert!(!plain.contains('\u{1b}'), "{plain:?}");
}

#[cfg(feature = "color")]
#[test]
fn print_decides_error_styles_from_the_commands_colour_choice_and_the_guides_terminal_detection() {
    const OFF: Guide = Guide::new().without_terminal_detection("The test runs in a terminal.");
    let args = ["tool", "comets"];
    let detecting = refused(&args);
    let never = refused_by(tool().color(clap::ColorChoice::Never), &Guide::new(), &args);
    let off = refused_by(tool(), &OFF, &args);

    assert_eq!(
        detecting.output().refusal_colour(),
        (clap::ColorChoice::Auto, true)
    );
    assert_eq!(
        never.output().refusal_colour(),
        (clap::ColorChoice::Never, true)
    );
    assert_eq!(
        off.output().refusal_colour(),
        (clap::ColorChoice::Auto, false)
    );
}
