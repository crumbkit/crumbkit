//! Tests for the opt-out of one example.

use snapbox::assert::Action;

use super::{FILES, GUIDE, examples, run};
use crate::allow::Allow;
use crate::temp::TempDir;
use crate::testing::panic_message;

#[test]
fn an_example_opt_out_skips_only_that_example() {
    const ALLOW: &[Allow] = &[Allow::example(
        "-c",
        &["echo books"],
        "The output names the day.",
    )];
    let dir = TempDir::new("examples-one-opt-out").unwrap();

    run(&GUIDE, &examples(&dir).allow(ALLOW), Action::Overwrite);

    assert_eq!(dir.names(), [FILES[0], FILES[2]]);
}

#[test]
fn an_example_opt_out_that_names_no_example_of_its_leaf_fails_the_check() {
    const ALLOW: &[Allow] = &[Allow::example(
        "-c",
        &["echo titles"],
        "The output names the day.",
    )];
    let dir = TempDir::new("examples-one-opt-out-stale").unwrap();

    assert_eq!(
        panic_message(|| run(&GUIDE, &examples(&dir).allow(ALLOW), Action::Overwrite)).unwrap(),
        "the command tree breaks crumbkit's example rules:\n\
         the opt-out from `examples/run` for `sh -c 'echo titles'` names no example that the check runs, so it is stale"
    );
}
