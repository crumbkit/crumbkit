//! Tests for the text line facet of a row type.

use schemars::JsonSchema;
use serde::Serialize;

use crate::{Line, Row};

/// A link that prints as `<POINTER> -> <TARGET>` when it has a target.
#[derive(Serialize, JsonSchema)]
struct Link {
    /// The name that points.
    pointer: &'static str,
    /// What the pointer points at, when it points at anything.
    target: Option<&'static str>,
}

impl Link {
    /// Returns the link's line, or `None` when it has no target.
    fn line(&self) -> Option<String> {
        let target = self.target?;
        Some(format!("{} -> {target}", self.pointer))
    }
}

impl Row for Link {
    const COLUMNS: &'static [&'static str] = &["pointer", "target"];
    const LINE: Option<Line<Self>> = Some(Line::new("<POINTER> -> <TARGET>", Self::line));
}

#[test]
fn a_line_returns_the_text_that_its_function_builds() {
    let link = Link {
        pointer: "latest",
        target: Some("v2"),
    };
    let text = Link::LINE.unwrap().text(&link);
    assert_eq!(text.as_deref(), Some("latest -> v2"));
}

#[test]
fn a_line_returns_its_form() {
    const LINE: Line<Link> = Line::new("<POINTER>", |_| None);
    assert_eq!(LINE.get_form(), "<POINTER>");
}

#[test]
fn a_line_prints_its_form_in_debug() {
    assert_eq!(
        format!("{:?}", Link::LINE.unwrap()),
        "Line { form: \"<POINTER> -> <TARGET>\", .. }"
    );
}
