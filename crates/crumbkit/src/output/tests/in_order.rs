//! Tests for how `finish_to` prints a batch whose document holds its
//! failures among its rows in input order (ADR-0016).

use std::fmt;
use std::process::ExitCode;

use serde_json::{Value, json};

use super::{Planet, finish_with, mercury, saturn};
use crate::Envelope;
use crate::{Batch, Call, Code, Document, Error, Failure, Recovery};

const NOT_FOUND: Code = Code::new("solar", "planets", "not_found");

/// A failure that names a planet the solar system lacks.
#[derive(Debug)]
struct Missing(&'static str);

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the solar system has no planet {}", self.0)
    }
}

impl std::error::Error for Missing {}

impl Failure for Missing {
    fn code(&self) -> &'static Code {
        &NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&["list"]))
    }
}

const SHOW: Document<Batch<Planet, Error>> =
    Document::new("solar", "planets-show", 1, "The test rows.").envelope(Envelope::Without);

/// Returns the batch of `Mercury Vulcan Saturn`, whose second key failed.
fn mercury_vulcan_saturn() -> Batch<Planet, Error> {
    let mut batch = Batch::new(&["show"], "names", &["Mercury", "Vulcan", "Saturn"]);
    batch.push(Ok(mercury()));
    batch.push(Err(Missing("Vulcan").into()));
    batch.push(Ok(saturn()));
    batch
}

#[test]
fn json_of_a_batch_in_input_order_prints_its_rows_and_failure_documents_in_the_order_of_their_keys()
{
    let finished = finish_with(
        &["--json"],
        SHOW.envelope(Envelope::WithoutInOrder),
        mercury_vulcan_saturn(),
    );

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(
        finished.out,
        concat!(
            r#"[{"name":"Mercury","moons":0,"ringed":false,"au":0.39},"#,
            r#"{"schema":"crumbkit/failure/v1","key":"Vulcan","index":1,"code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","recovery":{"kind":"run","call":"tool list"}},"#,
            r#"{"name":"Saturn","moons":274,"ringed":true,"au":9.54}]"#,
            "\n"
        )
    );
    assert_eq!(finished.err, "");
}

#[test]
fn fields_select_the_fields_of_the_rows_and_leave_each_failure_document_whole() {
    let finished = finish_with(
        &["--json", "--fields", "name"],
        SHOW.envelope(Envelope::WithoutInOrder),
        mercury_vulcan_saturn(),
    );

    let array: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(array[0], json!({"name": "Mercury"}));
    assert_eq!(array[1]["schema"], "crumbkit/failure/v1");
    assert_eq!(array[1]["message"], "the solar system has no planet Vulcan");
    assert_eq!(array[2], json!({"name": "Saturn"}));
}

#[test]
fn the_text_of_a_batch_in_input_order_is_the_text_of_the_same_batch_without_it() {
    let ordered = finish_with(
        &[],
        SHOW.envelope(Envelope::WithoutInOrder),
        mercury_vulcan_saturn(),
    );
    let apart = finish_with(&[], SHOW, mercury_vulcan_saturn());

    assert_eq!(
        (ordered.code, ordered.out, ordered.err),
        (apart.code, apart.out, apart.err)
    );
}
