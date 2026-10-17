//! Tests for the envelope that a document prints its result in.

use super::super::{Document, Envelope};
use crate::Listing;

use super::Moon;

/// Every envelope, so a test can check a rule for each of them.
const EVERY_ENVELOPE: [Envelope; 5] = [
    Envelope::Crumbkit,
    Envelope::Keys {
        type_key: "$type",
        data_key: "entries",
    },
    Envelope::KeysInOrder {
        type_key: "$type",
        data_key: "entries",
    },
    Envelope::Without,
    Envelope::WithoutInOrder,
];

const ENTRIES: Document<Listing<Moon>> = Document::new("tool", "get-entries", 1, "The entries.")
    .envelope(Envelope::Keys {
        type_key: "$type",
        data_key: "entries",
    });

#[test]
fn a_new_document_has_crumbkits_envelope_with_schema_and_data() {
    let document: Document<()> = Document::new("shelf", "books-list", 1, "Books.");
    let envelope = document.get_envelope();

    assert_eq!(envelope, Envelope::Crumbkit);
    assert_eq!(envelope.type_key(), "schema");
    assert_eq!(envelope.data_key(), "data");
    assert!(envelope.wraps());
    assert!(!envelope.has_own_keys());
}

#[test]
fn declared_keys_replace_schema_and_data_and_untyped_keeps_them() {
    let envelope = ENTRIES.untyped().get_envelope();

    assert_eq!(envelope.type_key(), "$type");
    assert_eq!(envelope.data_key(), "entries");
    assert!(envelope.has_own_keys());
    assert!(envelope.wraps());
}

#[test]
fn only_the_two_envelopes_without_wrapping_print_the_result_alone() {
    for envelope in EVERY_ENVELOPE {
        let alone = matches!(envelope, Envelope::Without | Envelope::WithoutInOrder);

        assert_eq!(envelope.wraps(), !alone, "{envelope:?}");
    }
}

#[test]
fn only_the_two_in_order_envelopes_print_failures_in_order() {
    for envelope in EVERY_ENVELOPE {
        let in_order = matches!(
            envelope,
            Envelope::KeysInOrder { .. } | Envelope::WithoutInOrder
        );

        assert_eq!(envelope.failures_in_order(), in_order, "{envelope:?}");
    }
}

#[test]
fn a_document_without_its_envelope_keeps_its_id() {
    let document: Document<()> =
        Document::new("shelf", "books-list", 3, "Books.").envelope(Envelope::Without);

    assert_eq!(document.get_envelope(), Envelope::Without);
    assert_eq!(document.to_string(), "shelf/books-list/v3");
}

#[test]
#[should_panic(expected = "the two keys differ")]
fn a_type_key_equal_to_the_data_key_is_rejected() {
    let _: Document<()> = Document::new("tool", "get", 1, "Rows.").envelope(Envelope::Keys {
        type_key: "rows",
        data_key: "rows",
    });
}

#[test]
fn each_field_that_crumbkit_writes_beside_the_data_is_rejected_as_a_key() {
    for name in super::super::envelope::RESERVED {
        let declared = std::panic::catch_unwind(|| {
            Document::<()>::new("tool", "get", 1, "Rows.").envelope(Envelope::KeysInOrder {
                type_key: "$type",
                data_key: name,
            })
        });

        assert!(declared.is_err(), "the envelope accepted {name}");
    }
}

#[test]
#[should_panic(expected = "one non-empty line")]
fn an_empty_key_is_rejected() {
    let _: Document<()> = Document::new("tool", "get", 1, "Rows.").envelope(Envelope::Keys {
        type_key: "",
        data_key: "rows",
    });
}
