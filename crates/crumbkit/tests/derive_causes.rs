//! The `causes` key of `#[derive(Failure)]`, through crumbkit's `Failure`
//! trait.

use std::fmt;

use crumbkit::{Code, Failure};

const INVALID: Code<'static> = Code::new("tool", "flows", "invalid");
const EMPTY: Code<'static> = Code::new("tool", "flows", "empty");

/// A failure that lists a reason for each invalid flow.
#[derive(Debug, crumbkit_derive::Failure)]
#[failure(crate = ::crumbkit)]
enum FlowsError {
    #[failure(
        code = INVALID,
        hint = "run `tool check`",
        causes = flows.iter().map(|flow| format!("the flow {flow} is invalid")).collect(),
    )]
    Invalid { flows: Vec<String> },
    #[failure(code = EMPTY, hint = "add a flow")]
    Empty,
}

impl fmt::Display for FlowsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("cannot publish flows")
    }
}

impl std::error::Error for FlowsError {}

/// A failure that wraps a [`FlowsError`].
#[derive(Debug, crumbkit_derive::Failure)]
#[failure(crate = ::crumbkit, forward = inner)]
struct Publish {
    inner: FlowsError,
}

impl fmt::Display for Publish {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("cannot publish")
    }
}

impl std::error::Error for Publish {}

#[test]
fn a_causes_key_returns_the_expressions_list_for_its_variant() {
    let failure = FlowsError::Invalid {
        flows: vec!["intake".to_owned(), "billing".to_owned()],
    };

    assert_eq!(
        failure.causes(),
        ["the flow intake is invalid", "the flow billing is invalid"]
    );
    assert_eq!(FlowsError::Empty.causes(), Vec::<String>::new());
}

#[test]
fn a_forwarding_variant_returns_the_causes_of_the_failure_it_wraps() {
    let failure = Publish {
        inner: FlowsError::Invalid {
            flows: vec!["intake".to_owned()],
        },
    };

    assert_eq!(failure.causes(), ["the flow intake is invalid"]);
}
