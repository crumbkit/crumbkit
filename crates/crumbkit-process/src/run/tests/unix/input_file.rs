//! Tests of a file or a pipe given as a child's input.

use std::fs::File;
use std::io::Write as _;
use std::os::fd::OwnedFd;
use std::sync::Arc;

use super::{kept, run_sh};
use crate::{Input, Outcome, Status};

#[test]
fn a_pipe_given_as_input_reaches_the_child() {
    let (reader, mut writer) = std::io::pipe().unwrap();
    writer.write_all(b"from the pipe\n").unwrap();
    drop(writer);
    let input = Input::File(Arc::new(File::from(OwnedFd::from(reader))));

    let finished = run_sh("cat", &kept().stdin(input));

    assert_eq!(finished.outcome(), Outcome::Ended(Status::Code(0)));
    assert_eq!(finished.stdout().bytes(), b"from the pipe\n");
}

#[test]
fn a_file_given_as_input_reaches_the_child_and_a_second_run_continues_from_its_position() {
    let path = std::env::temp_dir().join(format!("crumbkit-input-file-{}", std::process::id()));
    run_sh(&format!("printf content > {}", path.display()), &kept());
    let file = Arc::new(File::open(&path).unwrap());
    let options = kept().stdin(Input::File(Arc::clone(&file)));

    let first = run_sh("cat", &options);
    let second = run_sh("cat", &options);

    assert_eq!(first.stdout().bytes(), b"content");
    assert_eq!(second.stdout().bytes(), b"");
    #[expect(
        clippy::disallowed_methods,
        reason = "the test removes the file that its child wrote"
    )]
    std::fs::remove_file(path).unwrap();
}
