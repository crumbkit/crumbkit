//! Tests for the document that a leaf declares.

use super::{PLANETS, Planet};
use crate::Document;
use crate::help::Leaf;

#[test]
fn a_leaf_returns_the_document_it_declares_and_no_document_when_it_declares_none() {
    let leaf = Leaf::new(&["planets", "list"]).document(PLANETS);

    let document = leaf.get_document().unwrap().document();

    assert_eq!(document.to_string(), "tool/planets/v1");
    assert_eq!(document.description(), "The test rows.");
    assert!(Leaf::new(&["planets", "list"]).get_document().is_none());
}

#[test]
fn a_leaf_builds_the_schema_of_the_whole_document_it_declares() {
    let leaf = Leaf::new(&["planets", "list"]).document(PLANETS);

    assert_eq!(
        leaf.get_document().unwrap().document_schema(),
        crate::schemas::document_schema(PLANETS)
    );
}

#[test]
fn a_leaf_returns_the_shape_of_its_document_and_none_for_a_stream() {
    const STREAM: Document<crate::Stream<Planet>> =
        Document::new("tool", "planets-watch", 1, "Each planet.");

    let listed = Leaf::new(&["planets", "list"]).document(PLANETS);
    let streamed = Leaf::new(&["planets", "watch"]).stream(STREAM);

    assert_eq!(
        listed.get_document().unwrap().shape(),
        Some(crate::Shape::List)
    );
    assert_eq!(streamed.get_document().unwrap().shape(), None);
}
