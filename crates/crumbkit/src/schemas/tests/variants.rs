//! Tests for the compact schema of an enum row whose variants lack a column
//! or the children field (ADR-0132).

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::conforms::conforms;
use crate::schemas::document_schema;
use crate::{Document, Envelope, Listing, Row};

/// A tenant that is listed in full with its plan and applications, or in
/// short with its name alone.
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[schemars(deny_unknown_fields)]
enum Tenant {
    /// A tenant with every field.
    Full {
        /// The tenant's name.
        name: String,
        /// The tenant's plan.
        plan: String,
        /// The applications that the tenant runs.
        applications: Vec<String>,
    },
    /// A tenant with its name alone.
    Short {
        /// The tenant's name.
        name: String,
    },
}

impl Row for Tenant {
    const COLUMNS: &'static [&'static str] = &["kind", "name", "plan"];
    const CHILDREN: Option<&'static str> = Some("applications");
}

const TENANTS: Document<Listing<Tenant>> =
    Document::new("ops", "tenants-list", 1, "One page of the tenants.");

/// Returns the variants of `Tenant` in the compact schema of `TENANTS`.
fn variants(schema: &Value) -> Vec<Value> {
    schema["$defs"]["Tenant"]["oneOf"]
        .as_array()
        .unwrap()
        .clone()
}

/// Returns the compact JSON of `tenant`: each column and the children
/// field, with null for a field that the tenant lacks, as `--json` prints
/// it.
fn compact(tenant: &Tenant) -> Value {
    let whole = serde_json::to_value(tenant).unwrap();
    let row: Map<String, Value> = Tenant::COLUMNS
        .iter()
        .chain(Tenant::CHILDREN.iter())
        .map(|&name| {
            (
                name.to_owned(),
                whole.get(name).cloned().unwrap_or(Value::Null),
            )
        })
        .collect();
    Value::Object(row)
}

#[test]
fn a_variant_without_a_column_s_property_admits_null_for_that_column_in_the_compact_schema() {
    let schema = document_schema(TENANTS).to_value();
    let short = &variants(&schema)[1];

    assert_eq!(short["properties"]["plan"], json!({"type": "null"}));
    assert_eq!(short["additionalProperties"], json!(false));
    assert_eq!(short["required"], json!(["kind", "name"]));
}

#[test]
fn a_variant_without_the_children_field_s_property_admits_null_for_it_and_a_short_row_matches() {
    let schema = document_schema(TENANTS).to_value();
    let short = &variants(&schema)[1];
    let row = compact(&Tenant::Short {
        name: "acme".to_owned(),
    });

    assert_eq!(short["properties"]["applications"], json!({"type": "null"}));
    assert_eq!(row["applications"], Value::Null);
    assert_eq!(row["plan"], Value::Null);
    assert_eq!(conforms(&schema["$defs"]["Tenant"], &row), Ok(()));
}

#[test]
fn a_variant_that_has_the_property_keeps_its_schema() {
    let schema = document_schema(TENANTS).to_value();
    let whole = document_schema(TENANTS.envelope(Envelope::Without)).to_value();
    let full = &variants(&schema)[0];
    let whole_full = &whole["$defs"]["Tenant"]["oneOf"][0];
    let row = compact(&Tenant::Full {
        name: "acme".to_owned(),
        plan: "team".to_owned(),
        applications: vec!["billing".to_owned()],
    });

    assert_eq!(full["properties"], whole_full["properties"]);
    assert_eq!(
        full["required"],
        json!(["kind", "name", "plan", "applications"])
    );
    assert_eq!(conforms(&schema["$defs"]["Tenant"], &row), Ok(()));
}

#[test]
fn the_enum_s_own_schema_without_properties_gains_no_property() {
    let schema = document_schema(TENANTS).to_value();
    let tenant = schema["$defs"]["Tenant"].as_object().unwrap();

    assert_eq!(tenant.get("properties"), None);
}

/// How a project pays, which a project flattens into its own fields.
#[derive(Serialize, JsonSchema)]
#[serde(tag = "plan", rename_all = "lowercase")]
enum Billing {
    /// A project that pays by invoice.
    Paid {
        /// The project's invoice number.
        invoice: String,
    },
    /// A project that pays nothing.
    Free {
        /// The seats that the project may use.
        seats: u32,
    },
}

/// A project whose schema has its own properties beside the `oneOf` of its
/// billing.
#[derive(Serialize, JsonSchema)]
struct Project {
    /// The project's name.
    name: String,
    #[serde(flatten)]
    billing: Billing,
}

impl Row for Project {
    const COLUMNS: &'static [&'static str] = &["name", "plan", "invoice"];
}

#[test]
fn a_column_that_a_member_of_the_row_s_one_of_declares_gains_no_null_property_beside_it() {
    const PROJECTS: Document<Listing<Project>> =
        Document::new("ops", "projects-list", 1, "One page of the projects.");
    let projects = [
        Project {
            name: "web".to_owned(),
            billing: Billing::Paid {
                invoice: "A-1".to_owned(),
            },
        },
        Project {
            name: "docs".to_owned(),
            billing: Billing::Free { seats: 3 },
        },
    ];

    let schema = document_schema(PROJECTS).to_value();
    let project = &schema["$defs"]["Project"];

    assert_eq!(project["properties"].get("plan"), None);
    assert_eq!(project["properties"].get("invoice"), None);
    for project_row in &projects {
        let whole = serde_json::to_value(project_row).unwrap();
        let row: Map<String, Value> = Project::COLUMNS
            .iter()
            .map(|&name| {
                (
                    name.to_owned(),
                    whole.get(name).cloned().unwrap_or(Value::Null),
                )
            })
            .collect();
        assert_eq!(conforms(project, &Value::Object(row)), Ok(()));
    }
}
