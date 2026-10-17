//! Tests for the view flags that a stream refuses because its document
//! cannot use them.

use std::process::ExitCode;

use super::{SIGHTINGS, robin, run};
use crate::Envelope;

#[test]
fn full_on_a_stream_without_the_envelope_is_refused_before_the_first_line() {
    let unwrapped = SIGHTINGS.envelope(Envelope::Without);

    let streamed = run(&["--json", "--full"], unwrapped, &[robin()], None, Ok(()));

    assert_eq!(streamed.code, ExitCode::from(2));
    assert!(
        streamed.out.contains(r#""fields":{"argument":"--full"}"#),
        "{}",
        streamed.out
    );
    assert!(!streamed.out.contains("robin"), "{}", streamed.out);
}
