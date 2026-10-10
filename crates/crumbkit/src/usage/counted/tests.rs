use std::io::Write as _;

use super::Counted;

#[test]
fn the_count_is_the_number_of_bytes_the_inner_writer_took() {
    let mut out = Vec::new();
    let mut counted = Counted::new(&mut out);

    write!(counted, "one").unwrap();
    writeln!(counted, " two").unwrap();

    assert_eq!(counted.bytes(), 8);
    assert_eq!(out, b"one two\n");
}
