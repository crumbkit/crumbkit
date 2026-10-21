//! Tests for a styled sink: `parse_to` writes help with clap's styles to a
//! sink that asks for them, and plain text to every other sink. A refusal
//! carries clap's styles to a sink whose error writer wants them
//! (ADR-0131).

use std::process::ExitCode;

use super::{Stop, Unparsed, parse_to, tool, try_parse_to};
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

/// Runs `parse_to` over `tool()` with `args`, which clap refuses, into a
/// sink whose error writer wants styles when `styled` is set, and returns
/// what it printed on standard error. It checks that nothing went to
/// standard output.
fn refusal(args: &[&str], styled: bool) -> String {
    let (out, err) = refusal_streams(args, styled);
    assert_eq!(out, "", "{args:?}");
    err
}

/// Runs `parse_to` as [`refusal`] does, and returns what it printed on
/// standard output and on standard error.
fn refusal_streams(args: &[&str], styled: bool) -> (String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err).styled_errors(styled),
        tool(),
        &Guide::new(),
        args,
    );
    assert_eq!(
        result.err().as_ref().map(Stop::code),
        Some(ExitCode::from(2)),
        "{args:?}"
    );
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// Command lines that `tool()` refuses: a missing argument, an unknown
/// flag, a flag without its value, a mistyped subcommand, and a group
/// without its subcommand, which prints the group's page.
const REFUSED: [&[&str]; 5] = [
    &["tool", "planets", "show"],
    &["tool", "planets", "show", "mars", "--srt", "name"],
    &["tool", "planets", "show", "mars", "--sort"],
    &["tool", "planet"],
    &["tool", "planets"],
];

#[cfg(feature = "color")]
#[test]
fn a_refusal_is_styled_on_a_sink_whose_error_writer_wants_styles() {
    let palette = clap::builder::styling::Styles::styled();
    let error = palette.get_error();

    let styled = refusal(&["tool", "planets", "show", "mars", "--srt", "name"], true);

    assert!(
        styled.starts_with(&format!("{error}error:{error:#} ")),
        "{styled:?}"
    );
}

#[cfg(feature = "color")]
#[test]
fn a_refusals_group_page_keeps_clap_s_styles_on_a_sink_whose_error_writer_wants_styles() {
    let palette = clap::builder::styling::Styles::styled();
    let (usage, literal) = (palette.get_usage(), palette.get_literal());

    let styled = refusal(&["tool", "planets"], true);

    let heading = format!("{usage}Usage:{usage:#} {literal}tool planets{literal:#}");
    assert!(styled.contains(&heading), "{styled:?}");
    assert_eq!(strip(&styled), refusal(&["tool", "planets"], false));
}

#[cfg(feature = "color")]
#[test]
fn a_styled_refusal_without_its_escape_codes_is_the_plain_refusal() {
    for args in REFUSED {
        let styled = refusal(args, true);

        assert!(styled.contains("\u{1b}["), "{args:?} has no styles");
        assert_eq!(strip(&styled), refusal(args, false), "{args:?}");
    }
}

#[test]
fn a_refusal_is_plain_on_a_sink_from_sink_new_even_when_its_result_is_styled() {
    for args in REFUSED {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let _stop = parse_to::<(), _, _, _, _>(
            &mut Sink::new(&mut out, &mut err).styled(true),
            tool(),
            &Guide::new(),
            args,
        );

        let err = String::from_utf8(err).unwrap();
        assert!(!err.contains('\u{1b}'), "{args:?} printed escapes:\n{err}");
        assert_eq!(err, refusal(args, false), "{args:?}");
    }
}

#[cfg(not(feature = "color"))]
#[test]
fn without_the_color_feature_a_refusal_is_plain_on_a_sink_whose_error_writer_wants_styles() {
    for args in REFUSED {
        assert_eq!(refusal(args, true), refusal(args, false), "{args:?}");
    }
}

#[test]
fn a_refusal_in_json_form_carries_no_styles_on_a_sink_whose_error_writer_wants_styles() {
    let args = &["tool", "--json", "planets", "show", "mars", "--srt", "name"];

    let (out, err) = refusal_streams(args, true);

    assert_eq!(err, "");
    assert!(out.starts_with('{'), "{out:?}");
    assert_eq!((out, err), refusal_streams(args, false));
}

#[test]
fn a_refusal_that_try_parse_returns_prints_nothing_and_holds_no_escape_codes() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let unparsed = try_parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err).styled_errors(true),
        tool(),
        &Guide::new(),
        ["tool", "planets", "show", "mars", "--srt", "name"],
    );

    let Err(Unparsed::Refused(refused)) = unparsed else {
        unreachable!("the command line is refused");
    };
    assert_eq!((out, err), (Vec::new(), Vec::new()));
    assert!(
        !refused.message().contains('\u{1b}'),
        "{:?}",
        refused.message()
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
