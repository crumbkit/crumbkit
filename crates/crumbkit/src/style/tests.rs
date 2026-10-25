//! Tests for the styles of a table's cells.

use super::Style;

#[test]
fn each_style_starts_with_its_own_escape_code_and_ends_with_the_reset() {
    let rendered = |style: Style| {
        let ansi = style.ansi();
        format!("{}{}", ansi.render(), ansi.render_reset())
    };

    assert_eq!(rendered(Style::Emphasis), "\u{1b}[1m\u{1b}[0m");
    assert_eq!(rendered(Style::Dim), "\u{1b}[2m\u{1b}[0m");
    assert_eq!(rendered(Style::Success), "\u{1b}[32m\u{1b}[0m");
    assert_eq!(rendered(Style::Error), "\u{1b}[31m\u{1b}[0m");
}
