//! Tests for a styled sink: `parse_to` writes help with clap's styles to a
//! sink that asks for them, and plain text to every other sink.

use std::process::ExitCode;

use super::{Stop, parse_to, tool};
use crate::Sink;
use crate::help::Guide;

/// Runs `parse_to` over `tool()` with `args`, which ask for help or the
/// version, into a sink that asks for styles when `styled` is set, and
/// returns what it printed on standard output.
fn page(args: &[&str], styled: bool) -> String {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err).styled(styled),
        tool(),
        &Guide::new(),
        args,
    );
    assert_eq!(
        result.err().as_ref().map(Stop::code),
        Some(ExitCode::SUCCESS)
    );
    assert_eq!(err, b"");
    String::from_utf8(out).unwrap()
}

#[test]
fn a_sink_asks_for_no_styles_until_its_builder_asks_for_them() {
    let sink = Sink::new(Vec::new(), Vec::new());
    assert!(!sink.is_styled());

    let sink = sink.styled(true);
    assert!(sink.is_styled());

    let sink = sink.styled(false);
    assert!(!sink.is_styled());
}

#[test]
fn an_unstyled_sink_receives_help_and_the_version_without_escape_codes() {
    for args in [
        &["tool", "-h"][..],
        &["tool", "planets", "show", "--help"],
        &["tool", "--version"],
    ] {
        let page = page(args, false);
        assert!(
            !page.contains('\u{1b}'),
            "{args:?} printed escapes:\n{page}"
        );
    }
}

#[cfg(feature = "color")]
#[test]
fn a_styled_sink_receives_short_and_long_help_with_clap_s_styles() {
    for args in [&["tool", "-h"][..], &["tool", "planets", "show", "--help"]] {
        let styled = page(args, true);
        let plain = page(args, false);

        assert!(
            styled.contains("\u{1b}["),
            "{args:?} printed no styles:\n{styled}"
        );
        assert_eq!(strip(&styled), plain);
    }
}

#[cfg(feature = "color")]
#[test]
fn a_styled_sink_receives_the_bytes_that_clap_s_error_print_writes_at_a_terminal() {
    let error = tool()
        .disable_help_subcommand(true)
        .try_get_matches_from(["tool", "planets", "show", "-h"])
        .unwrap_err();
    let clap = error.render().ansi().to_string();

    let styled = page(&["tool", "planets", "show", "-h"], true);

    // crumbkit adds its lines after clap's page, so the page that clap
    // renders alone starts the same way up to its options.
    let options = clap.find("Options:").unwrap();
    assert_eq!(&styled[..options], &clap[..options]);
}

#[cfg(not(feature = "color"))]
#[test]
fn without_the_color_feature_a_styled_sink_receives_plain_help() {
    for args in [&["tool", "-h"][..], &["tool", "planets", "show", "--help"]] {
        assert_eq!(page(args, true), page(args, false));
    }
}

#[test]
fn a_styled_sink_receives_the_version_as_clap_renders_it() {
    assert_eq!(page(&["tool", "--version"], true), "tool 1.2.3\n");
    assert_eq!(page(&["tool", "-V"], true), page(&["tool", "-V"], false));
}

#[test]
fn a_styled_sink_receives_a_topic_as_plain_text() {
    let args = &["tool", "help", "exit-codes"];
    let styled = page(args, true);

    assert!(
        !styled.contains('\u{1b}'),
        "{args:?} printed escapes:\n{styled}"
    );
    assert_eq!(styled, page(args, false));
}

#[test]
fn a_styled_sink_receives_the_help_word_as_it_receives_the_help_flag() {
    assert_eq!(
        page(&["tool", "help"], true),
        page(&["tool", "--help"], true)
    );
}

/// Returns `text` without its ANSI escape sequences, each of which starts
/// with `ESC [` and ends with the letter `m` in clap's styles.
#[cfg(feature = "color")]
fn strip(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("\u{1b}[") {
        plain.push_str(&rest[..start]);
        let end = rest[start..].find('m').unwrap();
        rest = &rest[start + end + 1..];
    }
    plain.push_str(rest);
    plain
}
