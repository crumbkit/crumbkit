use std::fs::{self, File};

use super::{random_bytes, read_byte_at};
use crate::usage::file::create_dir;
use crate::usage::testing::TempDir;

#[test]
fn two_reads_of_random_bytes_give_different_bytes() {
    let first: [u8; 16] = random_bytes().unwrap();
    let second: [u8; 16] = random_bytes().unwrap();

    assert_ne!(first, second);
}

#[test]
fn a_byte_read_at_an_offset_is_the_byte_at_that_offset() {
    let dir = TempDir::new("read-byte-at");
    create_dir(dir.path()).unwrap();
    let path = dir.path().join("bytes");
    #[expect(
        clippy::disallowed_methods,
        reason = "the test writes the file whose bytes it then reads"
    )]
    fs::write(&path, b"abc\n").unwrap();
    let file = File::open(&path).unwrap();

    assert_eq!(read_byte_at(&file, 1).unwrap(), b'b');
    assert_eq!(read_byte_at(&file, 3).unwrap(), b'\n');
}
