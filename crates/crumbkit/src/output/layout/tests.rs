use serde_json::{Value, json};

use super::{JsonLayout, indent};

/// Returns `value` as `indent` lays out its compact bytes.
fn indented(value: &Value) -> String {
    let compact = serde_json::to_vec(value).unwrap();
    let mut out = Vec::new();
    indent(&compact, &mut out);
    String::from_utf8(out).unwrap()
}

/// Asserts that `indent` writes `value` as `serde_json`'s pretty formatter
/// does.
fn assert_matches_pretty(value: &Value) {
    assert_eq!(
        indented(value),
        serde_json::to_string_pretty(value).unwrap()
    );
}

#[test]
fn indent_matches_the_pretty_formatter_for_nested_objects_and_arrays() {
    assert_matches_pretty(&json!({
        "schema": "solar/planets/v1",
        "data": [{"name": "Saturn", "moons": 274, "rings": [1, 2.5, -3e-7]}],
        "cut": {"budget": 200, "nested": {"deeper": [[1], [true, null]]}}
    }));
}

#[test]
fn indent_leaves_empty_objects_and_arrays_on_one_line() {
    assert_matches_pretty(&json!({"data": [], "fields": {}, "rows": [[], {}, [{}]]}));
    assert_matches_pretty(&json!([]));
    assert_matches_pretty(&json!({}));
}

#[test]
fn indent_leaves_the_bytes_inside_strings_unchanged() {
    assert_matches_pretty(&json!({
        "quote": "a \"quoted\" {word}, [list]: done",
        "backslash": "C:\\path\\",
        "both": "\\\"",
        "control": "tab\there\nline",
        "key, with: marks {}": "value"
    }));
}

#[test]
fn indent_leaves_non_ascii_text_unchanged() {
    assert_matches_pretty(&json!({"title": "Æsop’s Fables", "emoji": "📚", "日本": ["語"]}));
}

#[test]
fn indent_matches_the_pretty_formatter_for_bare_values() {
    for value in [json!(1), json!("text"), json!(null), json!(false)] {
        assert_matches_pretty(&value);
    }
}

#[test]
fn the_default_layout_is_one_line() {
    assert_eq!(JsonLayout::default(), JsonLayout::OneLine);
}
