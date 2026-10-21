use clap::builder::StyledStr;
#[cfg(feature = "color")]
use clap::builder::styling::Style;
use clap::builder::styling::Styles;
use clap::{Arg, Command};

use super::text;

/// Returns clap's refusal of `args` against a tool with a `--limit` that
/// takes a number from 1 to 100 and a `--sort` that has a similar name.
fn refused(args: &[&str]) -> StyledStr {
    Command::new("tool")
        .arg(
            Arg::new("limit")
                .long("limit")
                .value_name("N")
                .value_parser(clap::value_parser!(u8).range(1..=100)),
        )
        .arg(Arg::new("sort").long("sort").value_name("KEY"))
        .try_get_matches_from(args)
        .unwrap_err()
        .render()
}

/// The text form that crumbkit composes for the refusal of
/// `tool --limit 101`.
const LIMIT: &str = "\
error: invalid value '101' for '--limit <N>': 101 is not in 1..=100
  help:  run `tool -h`
  code:  tool::cli::usage
";

/// The text form that crumbkit composes for the refusal of `tool --sor`.
#[cfg(feature = "color")]
const SIMILAR: &str = "\
error: unexpected argument '--sor' found; a similar argument exists: '--sort'
  usage: tool [OPTIONS]
  help:  run `tool -h`
  code:  tool::cli::usage
";

/// The text form that crumbkit composes for the refusal of `tool` without
/// its required `<NAME>`.
#[cfg(feature = "color")]
const MISSING: &str = "\
error: the following required arguments were not provided: <NAME>
  usage: tool <NAME>
  help:  run `tool -h`
  code:  tool::cli::usage
";

/// Returns clap's refusal of `tool` without its required `<NAME>`.
#[cfg(feature = "color")]
fn missing() -> StyledStr {
    Command::new("tool")
        .arg(Arg::new("name").value_name("NAME").required(true))
        .try_get_matches_from(["tool"])
        .unwrap_err()
        .render()
}

/// Returns `text` without its ANSI escape sequences, each of which starts
/// with `ESC [` and ends with the letter `m` in clap's styles.
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

#[cfg(feature = "color")]
#[test]
fn a_refused_value_takes_clap_s_invalid_style_and_its_argument_clap_s_literal_style() {
    let palette = Styles::styled();
    let (e, i, l) = (
        palette.get_error(),
        palette.get_invalid(),
        palette.get_literal(),
    );

    let styled = text(LIMIT, e, Some(&refused(&["tool", "--limit", "101"])));

    let expected = format!(
        "{e}error:{e:#} invalid value '{i}101{i:#}' for '{l}--limit <N>{l:#}': 101 is not in 1..=100\n  help:  run `tool -h`\n  code:  tool::cli::usage\n"
    );
    assert_eq!(styled, expected);
}

#[cfg(feature = "color")]
#[test]
fn a_suggestion_takes_clap_s_valid_style() {
    let palette = Styles::styled();
    let (e, i, v) = (
        palette.get_error(),
        palette.get_invalid(),
        palette.get_valid(),
    );

    let styled = text(SIMILAR, e, Some(&refused(&["tool", "--sor"])));

    let first = styled.lines().next().unwrap();
    assert_eq!(
        first,
        format!(
            "{e}error:{e:#} unexpected argument '{i}--sor{i:#}' found; a similar argument exists: '{v}--sort{v:#}'"
        )
    );
}

#[cfg(feature = "color")]
#[test]
fn a_usage_line_takes_the_styles_of_clap_s_usage_and_its_label_stays_plain() {
    let palette = Styles::styled();
    let (e, v, l) = (
        palette.get_error(),
        palette.get_valid(),
        palette.get_literal(),
    );

    let styled = text(MISSING, e, Some(&missing()));

    let expected = format!(
        "{e}error:{e:#} the following required arguments were not provided: {v}<NAME>{v:#}\n  usage: {l}tool{l:#} <NAME>\n  help:  run `tool -h`\n  code:  tool::cli::usage\n"
    );
    assert_eq!(styled, expected);
}

#[cfg(feature = "color")]
#[test]
fn a_refusal_without_clap_s_error_styles_only_its_error_label() {
    let palette = Styles::styled();
    let error = palette.get_error();
    let plain = "error: --fields names an unknown field 'radius'\n  help:  use the valid fields: name\n  code:  tool::cli::usage\n";

    let styled = text(plain, error, None);

    assert_eq!(
        styled,
        format!(
            "{error}error:{error:#} --fields names an unknown field 'radius'\n  help:  use the valid fields: name\n  code:  tool::cli::usage\n"
        )
    );
}

#[cfg(feature = "color")]
#[test]
fn a_styled_piece_that_crumbkit_s_text_does_not_hold_is_left_out() {
    let palette = Styles::styled();
    let error = palette.get_error();
    let plain = "error: unexpected argument '--other' found\n  code:  tool::cli::usage\n";

    let styled = text(plain, error, Some(&refused(&["tool", "--sor"])));

    assert_eq!(
        strip(&styled),
        plain,
        "the styled text keeps the plain bytes"
    );
    assert!(
        !styled.contains("--other\u{1b}"),
        "a piece that clap did not style stays plain: {styled:?}"
    );
}

#[cfg(feature = "color")]
#[test]
fn a_styled_refusal_without_its_escape_codes_is_the_plain_refusal() {
    let palette = Styles::styled();
    let error = palette.get_error();
    for (plain, args) in [
        (LIMIT, &["tool", "--limit", "101"][..]),
        (SIMILAR, &["tool", "--sor"]),
    ] {
        let styled = text(plain, error, Some(&refused(args)));

        assert!(styled.contains("\u{1b}["), "{args:?} has no styles");
        assert_eq!(strip(&styled), plain, "{args:?}");
    }
    let styled = text(MISSING, error, Some(&missing()));
    assert_eq!(strip(&styled), MISSING);
}

#[cfg(feature = "color")]
#[test]
fn a_plain_style_writes_no_escape_codes() {
    let plain = Style::new();

    assert_eq!(text(LIMIT, &plain, None), LIMIT);
}

#[cfg(not(feature = "color"))]
#[test]
fn without_the_color_feature_a_refusal_stays_plain() {
    let palette = Styles::styled();

    let printed = text(
        LIMIT,
        palette.get_error(),
        Some(&refused(&["tool", "--limit", "101"])),
    );

    assert_eq!(printed, LIMIT);
    assert_eq!(strip(&printed), LIMIT);
}
