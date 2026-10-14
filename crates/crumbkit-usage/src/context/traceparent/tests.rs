use super::{Traceparent, parse};

/// The trace id of the W3C specification's examples.
const TRACE: &str = "4bf92f3577b34da6a3ce929d0e0e4736";
/// The parent id of the W3C specification's examples.
const PARENT: &str = "00f067aa0ba902b7";

#[test]
fn a_version_00_value_gives_its_trace_id_parent_id_and_flags() {
    let parent = parse(&format!("00-{TRACE}-{PARENT}-01")).unwrap();

    assert_eq!(
        parent,
        Traceparent {
            trace: [
                0x4b, 0xf9, 0x2f, 0x35, 0x77, 0xb3, 0x4d, 0xa6, 0xa3, 0xce, 0x92, 0x9d, 0x0e, 0x0e,
                0x47, 0x36
            ],
            parent: [0x00, 0xf0, 0x67, 0xaa, 0x0b, 0xa9, 0x02, 0xb7],
            flags: 0x01,
        }
    );
}

#[test]
fn a_value_outside_the_w3c_shape_gives_no_parent() {
    let zero_trace = "0".repeat(32);
    let zero_parent = "0".repeat(16);
    let refused = [
        String::new(),
        "s3cr3t".to_owned(),
        format!("00-{}-{PARENT}-01", TRACE.to_uppercase()),
        format!("00-{TRACE}-{}-01", PARENT.to_uppercase()),
        format!("0A-{TRACE}-{PARENT}-01"),
        format!("00-{TRACE}-{PARENT}-0A"),
        format!("00-{zero_trace}-{PARENT}-01"),
        format!("00-{TRACE}-{zero_parent}-01"),
        format!("ff-{TRACE}-{PARENT}-01"),
        format!("0-{TRACE}-{PARENT}-01"),
        format!("00-{TRACE}0-{PARENT}-01"),
        format!("00-{TRACE}-{PARENT}0-01"),
        format!("00-{TRACE}-{PARENT}-1"),
        format!("00-{TRACE}-{PARENT}-01-"),
        format!("00-{TRACE}-{PARENT}-01-extra"),
        format!("00-{TRACE}{PARENT}-01"),
        format!("00-{TRACE}-{PARENT}01"),
        format!("00_{TRACE}-{PARENT}-01"),
        format!("00-+bf92f3577b34da6a3ce929d0e0e4736-{PARENT}-01"),
        format!("00-{TRACE}-{PARENT}-é"),
        format!("01-{TRACE}-{PARENT}-01x"),
    ];

    for value in refused {
        assert_eq!(parse(&value), None, "{value}");
    }
}

#[test]
fn a_higher_version_gives_its_first_four_fields_and_ignores_the_rest() {
    let plain = parse(&format!("cc-{TRACE}-{PARENT}-03")).unwrap();
    let extended = parse(&format!("cc-{TRACE}-{PARENT}-03-what-the-future-holds")).unwrap();

    assert_eq!(plain, extended);
    assert_eq!(plain.flags, 0x03);
}
