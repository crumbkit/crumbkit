//! Tests of the function that receives a copy of an output stream.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{after, run_sh};
use crate::testing::sh;
use crate::{Cancel, Capture, RunError, RunOptions, run};

/// The bytes that a copy function has seen.
type Seen = Arc<Mutex<Vec<u8>>>;

/// Returns a copy function that appends each chunk to the returned buffer.
fn collecting() -> (Seen, impl Fn(&[u8]) + Send + Sync + 'static) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let copy = {
        let seen = Arc::clone(&seen);
        move |chunk: &[u8]| seen.lock().unwrap().extend_from_slice(chunk)
    };
    (seen, copy)
}

#[test]
fn a_copy_sees_the_whole_stream_while_the_capture_keeps_its_limit() {
    let (seen, copy) = collecting();
    let options = RunOptions::new().stdout(Capture::tail(3)).stdout_copy(copy);

    let finished = run_sh("printf abcdef", &options);

    assert_eq!(*seen.lock().unwrap(), b"abcdef");
    assert_eq!(finished.stdout().bytes(), b"def");
}

#[test]
fn a_copy_of_a_discarded_stream_still_sees_it_and_nothing_is_kept() {
    let (seen, copy) = collecting();
    let options = RunOptions::new().stderr_copy(copy);

    let finished = run_sh("printf oops >&2", &options);

    assert_eq!(*seen.lock().unwrap(), b"oops");
    assert_eq!(finished.stderr().bytes(), b"");
}

#[test]
fn a_copy_sees_output_while_the_child_still_runs() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let cancel = Cancel::new();
    let options = RunOptions::new()
        .cancel(&cancel)
        .stdout_copy(move |chunk| sender.send(chunk.to_vec()).unwrap());
    let running = std::thread::spawn(move || run_sh("echo first; sleep 30", &options));

    let first = receiver.recv_timeout(Duration::from_secs(5)).unwrap();

    assert!(!running.is_finished());
    assert_eq!(first, b"first\n");
    cancel.cancel();
    running.join().unwrap();
}

#[test]
fn a_copy_that_panics_lets_the_child_run_to_its_end_and_fails_the_run() {
    let path = std::env::temp_dir().join(format!("crumbkit-copy-panic-{}", std::process::id()));
    let options = RunOptions::new()
        .deadline(after(30.0))
        .stdout_copy(|_| panic!("the copy failed"));
    // The child writes more than a pipe holds. A reader that stopped at the
    // panic closes the pipe, SIGPIPE ends `head`, and `&&` skips the file.
    let script = format!("head -c 1000000 /dev/zero && echo $$ > {}", path.display());

    let refused = run(sh(&script), &options).unwrap_err();

    assert!(matches!(refused, RunError::Run(_)), "{refused:?}");
    assert!(path.exists());
    #[expect(
        clippy::disallowed_methods,
        reason = "the test removes the file that its child wrote"
    )]
    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_copy_of_an_inherited_stream_leaves_the_stream_the_tools_own() {
    let options = RunOptions::new()
        .stdout(Capture::inherit())
        .stderr(super::WHOLE)
        .stdout_copy(|_| panic!("the run gave a chunk of an inherited stream"));

    assert_eq!(super::inode_under(&options), super::own_inode());
}
