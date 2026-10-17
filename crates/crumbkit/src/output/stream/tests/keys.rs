//! Tests for a stream whose document has the tool's own envelope keys
//! (ADR-0059).

use super::{SIGHTINGS, robin, run};
use crate::Envelope;

#[test]
fn each_line_of_a_stream_with_its_own_keys_holds_the_id_and_one_row_under_the_declared_keys() {
    let document = SIGHTINGS.envelope(Envelope::Keys {
        type_key: "$type",
        data_key: "entry",
    });

    let streamed = run(&["--json"], document, &[robin(), robin()], None, Ok(()));

    let line = r#"{"$type":"birds/sightings-watch/v1","entry":{"number":1,"bird":"robin"}}"#;
    assert_eq!(streamed.out, format!("{line}\n{line}\n"));
}
