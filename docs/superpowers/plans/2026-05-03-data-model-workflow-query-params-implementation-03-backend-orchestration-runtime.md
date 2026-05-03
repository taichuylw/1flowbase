# Data Model Query Params 03 Backend Orchestration Runtime Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Update the index plan after each completed task.

**Goal:** Compile and resolve `data_model_query` bindings in the orchestration runtime.

**Architecture:** The compiler filters Data Model bindings by action before compiling selector dependencies. The binding runtime resolves `data_model_query` into the standard list query object consumed by control-plane Data Model runtime.

**Tech Stack:** Rust 2021, `serde_json`, `anyhow`, orchestration-runtime tests.

---

## Files

- Modify: `api/crates/orchestration-runtime/src/compiler.rs`
- Modify: `api/crates/orchestration-runtime/src/binding_runtime.rs`
- Create: `api/crates/orchestration-runtime/src/_tests/binding_runtime_tests.rs`
- Modify: `api/crates/orchestration-runtime/src/_tests/mod.rs`
- Test: `api/crates/orchestration-runtime/src/_tests/compiler_tests.rs`

### Task 1: Compiler Active Bindings And Query Selectors

- [x] **Step 1: Add failing compiler tests**

Append to `api/crates/orchestration-runtime/src/_tests/compiler_tests.rs`:

```rust
#[test]
fn compile_data_model_query_extracts_selector_dependencies() {
    let flow_id = Uuid::now_v7();
    let mut document = sample_document(flow_id);
    document["graph"]["nodes"][1] = json!({
        "id": "node-data-model",
        "type": "data_model",
        "alias": "Orders",
        "description": "",
        "containerId": null,
        "position": { "x": 240, "y": 0 },
        "configVersion": 1,
        "config": { "data_model_code": "orders", "action": "list" },
        "bindings": {
            "query": {
                "kind": "data_model_query",
                "value": {
                    "filters": [
                        {
                            "field_code": "status",
                            "operator": "eq",
                            "value": { "kind": "selector", "selector": ["node-start", "query"] }
                        }
                    ],
                    "sorts": [],
                    "expand_relations": [],
                    "page": { "kind": "constant", "value": 1 },
                    "page_size": { "kind": "selector", "selector": ["node-start", "page_size"] }
                }
            }
        },
        "outputs": [
            { "key": "records", "title": "记录列表", "valueType": "array" },
            { "key": "total", "title": "记录总数", "valueType": "number" }
        ]
    });

    let plan = FlowCompiler::compile(flow_id, "draft-1", &document, &compile_context()).unwrap();

    assert_eq!(
        plan.nodes["node-data-model"].bindings["query"].selector_paths,
        vec![
            vec!["node-start".to_string(), "query".to_string()],
            vec!["node-start".to_string(), "page_size".to_string()]
        ]
    );
}

#[test]
fn compile_data_model_filters_inactive_bindings_by_action() {
    let flow_id = Uuid::now_v7();
    let mut document = sample_document(flow_id);
    document["graph"]["nodes"][1] = json!({
        "id": "node-data-model",
        "type": "data_model",
        "alias": "Orders",
        "description": "",
        "containerId": null,
        "position": { "x": 240, "y": 0 },
        "configVersion": 1,
        "config": { "data_model_code": "orders", "action": "create" },
        "bindings": {
            "query": {
                "kind": "data_model_query",
                "value": {
                    "filters": [
                        {
                            "field_code": "status",
                            "operator": "eq",
                            "value": { "kind": "selector", "selector": ["missing-node", "answer"] }
                        }
                    ],
                    "sorts": [],
                    "expand_relations": [],
                    "page": { "kind": "constant", "value": 1 },
                    "page_size": { "kind": "constant", "value": 20 }
                }
            },
            "payload": {
                "kind": "named_bindings",
                "value": [{ "name": "title", "selector": ["node-start", "query"] }]
            }
        },
        "outputs": [{ "key": "record", "title": "记录", "valueType": "json" }]
    });

    let plan = FlowCompiler::compile(flow_id, "draft-1", &document, &compile_context()).unwrap();

    assert!(!plan.nodes["node-data-model"].bindings.contains_key("query"));
    assert!(plan.nodes["node-data-model"].bindings.contains_key("payload"));
}
```

- [x] **Step 2: Confirm failure**

Run:

```bash
cargo test -p orchestration-runtime compile_data_model -- --test-threads=1
```

Expected: FAIL because `data_model_query` is unsupported and inactive Data Model bindings are still compiled.

- [x] **Step 3: Filter active bindings**

In `api/crates/orchestration-runtime/src/compiler.rs`, replace the binding compile block in `compile_node` with:

```rust
    let raw_bindings = node
        .get("bindings")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("node {node_id} missing bindings"))?;
    let active_bindings = active_binding_values(&node_type, &config, raw_bindings);
    let bindings = compile_bindings(&active_bindings)
        .with_context(|| format!("failed to compile bindings for node {node_id}"))?;
```

Add helpers near `compile_bindings`:

```rust
fn active_binding_values(
    node_type: &str,
    config: &Value,
    binding_values: &serde_json::Map<String, Value>,
) -> BTreeMap<String, Value> {
    if node_type != "data_model" {
        return binding_values.iter().map(|(key, value)| (key.clone(), value.clone())).collect();
    }

    let active_keys = active_data_model_binding_keys(config);

    binding_values
        .iter()
        .filter(|(key, _)| active_keys.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn active_data_model_binding_keys(config: &Value) -> &'static [&'static str] {
    match config.get("action").and_then(Value::as_str).unwrap_or("list") {
        "get" => &["record_id"],
        "create" => &["payload"],
        "update" => &["record_id", "payload"],
        "delete" => &["record_id"],
        "list" | _ => &["query"],
    }
}
```

- [x] **Step 4: Extract query selectors**

Add this branch to `extract_selector_paths`:

```rust
        "data_model_query" => extract_data_model_query_selector_paths(raw_value),
```

Add helpers below `extract_selector_paths`:

```rust
fn extract_data_model_query_selector_paths(raw_value: &Value) -> Result<Vec<Vec<String>>> {
    let object = raw_value
        .as_object()
        .ok_or_else(|| anyhow!("data_model_query value must be an object"))?;
    let mut selectors = Vec::new();

    if let Some(filters) = object.get("filters") {
        for filter in filters
            .as_array()
            .ok_or_else(|| anyhow!("data_model_query filters must be an array"))?
        {
            if let Some(value) = filter.get("value") {
                push_query_value_selector(value, &mut selectors)?;
            }
        }
    }

    if let Some(page) = object.get("page") {
        push_query_value_selector(page, &mut selectors)?;
    }
    if let Some(page_size) = object.get("page_size") {
        push_query_value_selector(page_size, &mut selectors)?;
    }

    Ok(selectors)
}

fn push_query_value_selector(value: &Value, selectors: &mut Vec<Vec<String>>) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("data_model_query value input must be an object"))?;

    if object.get("kind").and_then(Value::as_str) == Some("selector") {
        selectors.push(selector_path(object.get("selector").unwrap_or(&Value::Null))?);
    }

    Ok(())
}
```

- [x] **Step 5: Verify compiler**

Run:

```bash
cargo test -p orchestration-runtime compile_data_model -- --test-threads=1
```

Expected: PASS.

### Task 2: Binding Runtime Query Resolution

- [x] **Step 1: Add failing binding runtime tests**

Create `api/crates/orchestration-runtime/src/_tests/binding_runtime_tests.rs`:

```rust
use std::collections::BTreeMap;

use orchestration_runtime::binding_runtime::resolve_node_inputs;
use orchestration_runtime::compiled_plan::{CompiledBinding, CompiledNode};
use serde_json::{json, Map, Value};

fn compiled_node(binding: CompiledBinding) -> CompiledNode {
    CompiledNode {
        node_id: "node-data-model".to_string(),
        node_type: "data_model".to_string(),
        alias: "Orders".to_string(),
        container_id: None,
        dependency_node_ids: Vec::new(),
        downstream_node_ids: Vec::new(),
        bindings: BTreeMap::from([("query".to_string(), binding)]),
        outputs: Vec::new(),
        config: json!({ "data_model_code": "orders", "action": "list" }),
        plugin_runtime: None,
        llm_runtime: None,
    }
}

#[test]
fn resolve_data_model_query_binding_with_constant_and_selector_values() {
    let node = compiled_node(CompiledBinding {
        kind: "data_model_query".to_string(),
        raw_value: json!({
            "filters": [
                { "field_code": "status", "operator": "eq", "value": { "kind": "constant", "value": "paid" } },
                { "field_code": "customer_id", "operator": "eq", "value": { "kind": "selector", "selector": ["node-start", "customer_id"] } }
            ],
            "sorts": [{ "field_code": "created_at", "direction": "desc" }],
            "expand_relations": ["customer"],
            "page": { "kind": "constant", "value": 1 },
            "page_size": { "kind": "selector", "selector": ["node-start", "page_size"] }
        }),
        selector_paths: vec![],
    });
    let variable_pool = Map::from_iter([(
        "node-start".to_string(),
        json!({ "customer_id": "customer-1", "page_size": 50 }),
    )]);

    let resolved = resolve_node_inputs(&node, &variable_pool).unwrap();

    assert_eq!(
        resolved["query"],
        json!({
            "filters": [
                { "field_code": "status", "operator": "eq", "value": "paid" },
                { "field_code": "customer_id", "operator": "eq", "value": "customer-1" }
            ],
            "sorts": [{ "field_code": "created_at", "direction": "desc" }],
            "expand_relations": ["customer"],
            "page": 1,
            "page_size": 50
        })
    );
}

#[test]
fn resolve_data_model_query_rejects_invalid_operator() {
    let node = compiled_node(CompiledBinding {
        kind: "data_model_query".to_string(),
        raw_value: json!({
            "filters": [
                { "field_code": "status", "operator": "contains", "value": { "kind": "constant", "value": "paid" } }
            ],
            "sorts": [],
            "expand_relations": [],
            "page": { "kind": "constant", "value": 1 },
            "page_size": { "kind": "constant", "value": 20 }
        }),
        selector_paths: vec![],
    });

    let error = resolve_node_inputs(&node, &Map::<String, Value>::new()).unwrap_err();

    assert!(error.to_string().contains("data_model list filter operator is unsupported"));
}
```

Add to `api/crates/orchestration-runtime/src/_tests/mod.rs`:

```rust
mod binding_runtime_tests;
```

- [x] **Step 2: Confirm failure**

Run:

```bash
cargo test -p orchestration-runtime data_model_query -- --test-threads=1
```

Expected: FAIL because binding runtime rejects the new kind.

- [x] **Step 3: Resolve `data_model_query`**

In `api/crates/orchestration-runtime/src/binding_runtime.rs`, add this match branch:

```rust
        "data_model_query" => resolve_data_model_query_binding(binding, variable_pool),
```

Add helpers:

```rust
fn resolve_data_model_query_binding(
    binding: &CompiledBinding,
    variable_pool: &Map<String, Value>,
) -> Result<Value> {
    let object = binding
        .raw_value
        .as_object()
        .ok_or_else(|| anyhow!("data_model list query must be object"))?;
    let mut query = Map::new();

    query.insert("filters".to_string(), Value::Array(resolve_query_filters(object.get("filters"), variable_pool)?));
    query.insert("sorts".to_string(), Value::Array(resolve_query_sorts(object.get("sorts"))?));
    query.insert("expand_relations".to_string(), Value::Array(resolve_string_array(object.get("expand_relations"), "data_model list expand_relations")?));
    query.insert("page".to_string(), resolve_query_value(object.get("page"), variable_pool, Value::from(1))?);
    query.insert("page_size".to_string(), resolve_query_value(object.get("page_size"), variable_pool, Value::from(20))?);

    Ok(Value::Object(query))
}

fn resolve_query_filters(value: Option<&Value>, variable_pool: &Map<String, Value>) -> Result<Vec<Value>> {
    let Some(value) = value else { return Ok(Vec::new()); };
    let entries = value.as_array().ok_or_else(|| anyhow!("data_model list filters must be array"))?;
    let mut filters = Vec::with_capacity(entries.len());

    for entry in entries {
        let object = entry.as_object().ok_or_else(|| anyhow!("data_model list filter must be object"))?;
        let field_code = required_query_string(object, "field_code", "filter field_code")?;
        let operator = required_query_string(object, "operator", "filter operator")?;
        ensure_supported_filter_operator(&operator)?;
        let value = resolve_query_value(object.get("value"), variable_pool, Value::Null)?;

        filters.push(Value::Object(Map::from_iter([
            ("field_code".to_string(), Value::String(field_code)),
            ("operator".to_string(), Value::String(operator)),
            ("value".to_string(), value),
        ])));
    }

    Ok(filters)
}

fn resolve_query_sorts(value: Option<&Value>) -> Result<Vec<Value>> {
    let Some(value) = value else { return Ok(Vec::new()); };
    let entries = value.as_array().ok_or_else(|| anyhow!("data_model list sorts must be array"))?;
    let mut sorts = Vec::with_capacity(entries.len());

    for entry in entries {
        let object = entry.as_object().ok_or_else(|| anyhow!("data_model list sort must be object"))?;
        let field_code = required_query_string(object, "field_code", "sort field_code")?;
        let direction = object.get("direction").and_then(Value::as_str).unwrap_or("asc").to_ascii_lowercase();

        if !matches!(direction.as_str(), "asc" | "desc") {
            return Err(anyhow!("data_model list sort direction is unsupported"));
        }

        sorts.push(Value::Object(Map::from_iter([
            ("field_code".to_string(), Value::String(field_code)),
            ("direction".to_string(), Value::String(direction)),
        ])));
    }

    Ok(sorts)
}

fn resolve_string_array(value: Option<&Value>, label: &str) -> Result<Vec<Value>> {
    let Some(value) = value else { return Ok(Vec::new()); };

    value
        .as_array()
        .ok_or_else(|| anyhow!("{label} must be array"))?
        .iter()
        .map(|entry| entry.as_str().map(|text| Value::String(text.to_string())).ok_or_else(|| anyhow!("{label} item must be string")))
        .collect()
}

fn resolve_query_value(value: Option<&Value>, variable_pool: &Map<String, Value>, fallback: Value) -> Result<Value> {
    let Some(value) = value else { return Ok(fallback); };
    let object = value.as_object().ok_or_else(|| anyhow!("data_model query value must be object"))?;

    match object.get("kind").and_then(Value::as_str) {
        Some("constant") => Ok(object.get("value").cloned().unwrap_or(Value::Null)),
        Some("selector") => {
            let selector = object
                .get("selector")
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow!("data_model query selector must be array"))?
                .iter()
                .map(|segment| segment.as_str().map(str::to_string).ok_or_else(|| anyhow!("data_model query selector segment must be string")))
                .collect::<Result<Vec<_>>>()?;
            lookup_selector_value(variable_pool, &selector)
        }
        Some(other) => Err(anyhow!("data_model query value kind is unsupported: {other}")),
        None => Ok(fallback),
    }
}

fn required_query_string(object: &Map<String, Value>, key: &'static str, label: &'static str) -> Result<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("data_model list {label} is required"))
}

fn ensure_supported_filter_operator(operator: &str) -> Result<()> {
    match operator {
        "eq" | "ne" | "gt" | "gte" | "lt" | "lte" => Ok(()),
        _ => Err(anyhow!("data_model list filter operator is unsupported")),
    }
}
```

- [x] **Step 4: Verify**

Run:

```bash
cargo test -p orchestration-runtime compile_data_model data_model_query -- --test-threads=1
```

Expected: PASS.

- [x] **Step 5: Commit**

Run:

```bash
git add api/crates/orchestration-runtime/src/compiler.rs api/crates/orchestration-runtime/src/binding_runtime.rs api/crates/orchestration-runtime/src/_tests/compiler_tests.rs api/crates/orchestration-runtime/src/_tests/binding_runtime_tests.rs api/crates/orchestration-runtime/src/_tests/mod.rs
git commit -m "feat: compile and resolve data model query bindings"
```
