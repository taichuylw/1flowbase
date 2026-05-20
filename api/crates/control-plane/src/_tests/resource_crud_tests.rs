use std::collections::HashMap;

use control_plane::resource_crud::{
    filter_by_tk_values, filter_resource_records, matches_resource_filter, parse_resource_filter,
    ResourceBatchSelection, ResourceCrudDescriptor, ResourceFilterTarget,
};
use serde_json::json;

struct TestResource {
    fields: HashMap<&'static str, String>,
}

impl TestResource {
    fn new(fields: &[(&'static str, &str)]) -> Self {
        Self {
            fields: fields
                .iter()
                .map(|(key, value)| (*key, (*value).to_string()))
                .collect(),
        }
    }
}

impl ResourceFilterTarget for TestResource {
    fn field_value(&self, field: &str) -> Option<String> {
        self.fields.get(field).cloned()
    }
}

#[test]
fn resource_filter_matches_nocobase_style_field_operators() {
    let resource = TestResource::new(&[
        ("code", "customer_profiles"),
        ("title", "Customer Profiles"),
        ("status", "published"),
    ]);
    let filter = json!({
        "$or": [
            { "code": { "$includes": "customer" } },
            { "title.$includes": "customer" }
        ],
        "status": "published"
    });

    assert!(matches_resource_filter(&resource, &filter).unwrap());
}

#[test]
fn resource_filter_rejects_unknown_operator() {
    let resource = TestResource::new(&[("code", "orders")]);
    let error = matches_resource_filter(
        &resource,
        &json!({
            "code": { "$startsWith": "ord" }
        }),
    )
    .unwrap_err();

    assert!(error.to_string().contains("filter"));
}

#[test]
fn parse_resource_filter_accepts_empty_and_json_object() {
    assert!(parse_resource_filter(None).unwrap().is_none());
    assert!(parse_resource_filter(Some("")).unwrap().is_none());
    assert_eq!(
        parse_resource_filter(Some(r#"{"status":"published"}"#)).unwrap(),
        Some(json!({ "status": "published" }))
    );
}

#[test]
fn filter_by_tk_accepts_single_or_many_keys() {
    assert_eq!(
        filter_by_tk_values(json!("model-1")).unwrap(),
        vec!["model-1".to_string()]
    );
    assert_eq!(
        filter_by_tk_values(json!(["model-1", "model-2"])).unwrap(),
        vec!["model-1".to_string(), "model-2".to_string()]
    );
}

#[test]
fn filter_resource_records_returns_matching_records() {
    let resources = vec![
        TestResource::new(&[("code", "orders")]),
        TestResource::new(&[("code", "customer_profiles")]),
    ];

    let filtered = filter_resource_records(
        resources,
        Some(&json!({ "code": { "$includes": "customer" } })),
    )
    .unwrap();

    assert_eq!(filtered.len(), 1);
    assert_eq!(
        filtered[0].field_value("code").as_deref(),
        Some("customer_profiles")
    );
}

#[test]
fn resource_crud_descriptor_filters_records() {
    let descriptor = ResourceCrudDescriptor::new("test_resource", "id");
    let resources = vec![
        TestResource::new(&[("id", "1"), ("code", "orders")]),
        TestResource::new(&[("id", "2"), ("code", "customer_profiles")]),
    ];

    let filtered = descriptor
        .filter_records(
            resources,
            Some(&json!({ "code": { "$includes": "customer" } })),
        )
        .unwrap();

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].field_value("id").as_deref(), Some("2"));
}

#[test]
fn resource_crud_descriptor_selects_batch_ids_by_filter_by_tk() {
    let descriptor = ResourceCrudDescriptor::new("test_resource", "id");
    let selected = descriptor
        .select_batch_ids::<TestResource, String, _, _>(
            Vec::new(),
            ResourceBatchSelection::new(Some(json!(["id-1", "id-2"])), None),
            Ok,
            |resource| resource.field_value("id").unwrap_or_default(),
        )
        .unwrap();

    assert_eq!(selected, vec!["id-1".to_string(), "id-2".to_string()]);
}

#[test]
fn resource_crud_descriptor_selects_batch_ids_by_filter() {
    let descriptor = ResourceCrudDescriptor::new("test_resource", "id");
    let resources = vec![
        TestResource::new(&[("id", "1"), ("code", "orders")]),
        TestResource::new(&[("id", "2"), ("code", "customer_profiles")]),
    ];

    let selected = descriptor
        .select_batch_ids(
            resources,
            ResourceBatchSelection::new(None, Some(json!({ "code.$includes": "customer" }))),
            Ok,
            |resource| resource.field_value("id").unwrap_or_default(),
        )
        .unwrap();

    assert_eq!(selected, vec!["2".to_string()]);
}
