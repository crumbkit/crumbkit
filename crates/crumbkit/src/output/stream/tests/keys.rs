//! Tests for a stream whose document has the tool's own envelope keys
//! (ADR-0059).

use super::{SIGHTINGS, robin, run};

#[test]
fn each_line_of_a_stream_with_its_own_keys_holds_the_type_and_one_row_under_the_declared_keys() {
    let document = SIGHTINGS.with_keys("$type", "entry").with_type("sighting");

    let streamed = run(&["--json"], document, &[robin(), robin()], None, Ok(()));

    let line = r#"{"$type":"sighting","entry":{"number":1,"bird":"robin"}}"#;
    assert_eq!(streamed.out, format!("{line}\n{line}\n"));
}
