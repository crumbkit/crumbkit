//! Tests that the compact JSON of an enum row whose variants lack a column
//! or the children field matches the document schema (ADR-0132).

use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

use super::finish_with;
use crate::conforms::conforms;
use crate::schemas::document_schema;
use crate::{Document, Listing, Row};

/// An account that is listed in full with its owner and applications, or
/// in short with its id alone. Each variant refuses a field it does not
/// declare.
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[schemars(deny_unknown_fields)]
enum Account {
    /// An account with every field.
    Full {
        /// The account's id.
        id: u32,
        /// The account's owner.
        owner: String,
        /// The applications that the account runs.
        applications: Vec<String>,
    },
    /// An account with its id alone.
    Short {
        /// The account's id.
        id: u32,
    },
}

impl Row for Account {
    const COLUMNS: &'static [&'static str] = &["kind", "id", "owner"];
    const CHILDREN: Option<&'static str> = Some("applications");

    fn child_lines(&self) -> Vec<String> {
        match self {
            Self::Full { applications, .. } => applications.clone(),
            Self::Short { .. } => Vec::new(),
        }
    }
}

const ACCOUNTS: Document<Listing<Account>> =
    Document::new("ops", "accounts-list", 1, "One page of the accounts.");

/// Returns a listing of one full account and one short account.
fn full_and_short() -> Listing<Account> {
    let accounts = vec![
        Account::Full {
            id: 1,
            owner: "ada".to_owned(),
            applications: vec!["billing".to_owned()],
        },
        Account::Short { id: 2 },
    ];
    Listing::new(accounts, 2)
}

#[test]
fn the_compact_json_of_a_listing_with_a_full_and_a_short_variant_matches_the_document_schema() {
    let finished = finish_with(&["--json"], ACCOUNTS, full_and_short());

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    let document: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(document["data"][1]["owner"], Value::Null);
    assert_eq!(document["data"][1]["applications"], Value::Null);
    let schema = document_schema(ACCOUNTS).to_value();
    assert_eq!(conforms(&schema, &document), Ok(()));
}

#[test]
fn the_full_json_of_a_listing_with_a_full_and_a_short_variant_matches_the_document_schema() {
    let finished = finish_with(&["--json", "--full"], ACCOUNTS, full_and_short());

    let document: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(document["data"][1].get("applications"), None);
    let schema = document_schema(ACCOUNTS).to_value();
    assert_eq!(conforms(&schema, &document), Ok(()));
}
