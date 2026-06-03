use super::*;

fn branch_document(flow_id: Uuid) -> Value {
    let if_condition = json!({
        "operator": "or",
        "conditions": [
            {
                "kind": "rule",
                "left": ["node-start", "query"],
                "comparator": "contains",
                "right": { "kind": "constant", "value": "vip" }
            },
            {
                "operator": "and",
                "conditions": [{
                    "kind": "rule",
                    "left": ["node-start", "tier"],
                    "comparator": "equals",
                    "right": { "kind": "selector", "selector": ["node-start", "expected_tier"] }
                }]
            }
        ]
    });

    json!({
        "schemaVersion": "1flowbase.flow/v2",
        "meta": { "flowId": flow_id.to_string(), "name": "Branches", "description": "", "tags": [] },
        "graph": {
            "nodes": [
                {
                    "id": "node-start",
                    "type": "start",
                    "alias": "Start",
                    "description": "",
                    "containerId": null,
                    "position": { "x": 0, "y": 0 },
                    "configVersion": 1,
                    "config": {},
                    "bindings": {},
                    "outputs": []
                },
                {
                    "id": "node-if",
                    "type": "if_else",
                    "alias": "If / Else",
                    "description": "",
                    "containerId": null,
                    "position": { "x": 240, "y": 0 },
                    "configVersion": 1,
                    "config": {},
                    "bindings": {
                        "branches": {
                            "kind": "if_else_branches",
                            "value": {
                                "branches": [
                                    {
                                        "id": "if",
                                        "kind": "if",
                                        "title": "If",
                                        "sourceHandle": "if",
                                        "condition": if_condition
                                    },
                                    {
                                        "id": "else",
                                        "kind": "else",
                                        "title": "Else",
                                        "sourceHandle": "else"
                                    }
                                ]
                            }
                        }
                    },
                    "outputs": []
                },
                {
                    "id": "node-answer-if",
                    "type": "answer",
                    "alias": "Answer If",
                    "description": "",
                    "containerId": null,
                    "position": { "x": 480, "y": 0 },
                    "configVersion": 1,
                    "config": {},
                    "bindings": {
                        "answer_template": { "kind": "templated_text", "value": "if" }
                    },
                    "outputs": [{ "key": "answer", "title": "回答", "valueType": "string" }]
                },
                {
                    "id": "node-answer-else",
                    "type": "answer",
                    "alias": "Answer Else",
                    "description": "",
                    "containerId": null,
                    "position": { "x": 480, "y": 160 },
                    "configVersion": 1,
                    "config": {},
                    "bindings": {
                        "answer_template": { "kind": "templated_text", "value": "else" }
                    },
                    "outputs": [{ "key": "answer", "title": "回答", "valueType": "string" }]
                }
            ],
            "edges": [
                {
                    "id": "edge-start-if",
                    "source": "node-start",
                    "target": "node-if",
                    "sourceHandle": null,
                    "targetHandle": null,
                    "containerId": null,
                    "points": []
                },
                {
                    "id": "edge-if-answer",
                    "source": "node-if",
                    "target": "node-answer-if",
                    "sourceHandle": "if",
                    "targetHandle": null,
                    "containerId": null,
                    "points": []
                },
                {
                    "id": "edge-else-answer",
                    "source": "node-if",
                    "target": "node-answer-else",
                    "sourceHandle": "else",
                    "targetHandle": null,
                    "containerId": null,
                    "points": []
                }
            ]
        },
        "editor": { "viewport": { "x": 0, "y": 0, "zoom": 1 }, "annotations": [], "activeContainerPath": [] }
    })
}

#[test]
fn compile_if_else_branches_preserves_source_handles_and_condition_selectors() {
    let flow_id = Uuid::nil();
    let document = branch_document(flow_id);

    let plan = FlowCompiler::compile(flow_id, "draft-branches", &document, &compile_context())
        .expect("if_else branch document should compile");

    assert_eq!(
        plan.edges
            .iter()
            .map(|edge| (edge.edge_id.as_str(), edge.source_handle.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            ("edge-start-if", None),
            ("edge-if-answer", Some("if")),
            ("edge-else-answer", Some("else")),
        ]
    );
    assert_eq!(
        plan.nodes["node-if"].bindings["branches"].selector_paths,
        vec![
            vec!["node-start".to_string(), "query".to_string()],
            vec!["node-start".to_string(), "tier".to_string()],
            vec!["node-start".to_string(), "expected_tier".to_string()],
        ]
    );
}

#[test]
fn compile_if_else_rejects_unknown_source_handles() {
    let flow_id = Uuid::nil();
    let mut document = branch_document(flow_id);
    document["graph"]["edges"][1]["sourceHandle"] = json!("stale-branch");

    let error = FlowCompiler::compile(flow_id, "draft-branches", &document, &compile_context())
        .expect_err("stale if_else source handle should fail compile");

    assert!(error
        .to_string()
        .contains("edge edge-if-answer references unknown if_else sourceHandle stale-branch"));
}

#[test]
fn compile_if_else_rejects_missing_else_branch() {
    let flow_id = Uuid::nil();
    let mut document = branch_document(flow_id);
    document["graph"]["nodes"][1]["bindings"]["branches"]["value"]["branches"] =
        json!([document["graph"]["nodes"][1]["bindings"]["branches"]["value"]["branches"][0]]);
    document["graph"]["edges"] =
        json!([document["graph"]["edges"][0], document["graph"]["edges"][1],]);

    let error = FlowCompiler::compile(flow_id, "draft-branches", &document, &compile_context())
        .expect_err("if_else branch document without else fallback should fail compile");

    assert!(error
        .to_string()
        .contains("if_else node node-if must include an else branch"));
}

#[test]
fn compile_if_else_rejects_duplicate_source_handles() {
    let flow_id = Uuid::nil();
    let mut document = branch_document(flow_id);
    document["graph"]["nodes"][1]["bindings"]["branches"]["value"]["branches"][1]["sourceHandle"] =
        json!("if");
    document["graph"]["edges"][2]["sourceHandle"] = json!("if");

    let error = FlowCompiler::compile(flow_id, "draft-branches", &document, &compile_context())
        .expect_err("if_else branch document with duplicate source handles should fail compile");

    assert!(error
        .to_string()
        .contains("if_else node node-if duplicate branch sourceHandle if"));
}

#[test]
fn compile_if_else_rejects_missing_branch_condition() {
    let flow_id = Uuid::nil();
    let mut document = branch_document(flow_id);
    document["graph"]["nodes"][1]["bindings"]["branches"]["value"]["branches"][0]
        .as_object_mut()
        .expect("branch should be an object")
        .remove("condition");

    let error = FlowCompiler::compile(flow_id, "draft-branches", &document, &compile_context())
        .expect_err("if_else branch without condition should fail compile");

    assert!(error
        .to_string()
        .contains("if_else node node-if branch if must include a complete condition"));
}

#[test]
fn compile_if_else_rejects_empty_branch_condition() {
    let flow_id = Uuid::nil();
    let mut document = branch_document(flow_id);
    document["graph"]["nodes"][1]["bindings"]["branches"]["value"]["branches"][0]["condition"] = json!({
        "operator": "and",
        "conditions": []
    });

    let error = FlowCompiler::compile(flow_id, "draft-branches", &document, &compile_context())
        .expect_err("if_else branch with empty condition should fail compile");

    assert!(error
        .to_string()
        .contains("if_else node node-if branch if must include a complete condition"));
}

#[test]
fn compile_if_else_rejects_invalid_branch_contract_without_outgoing_edges() {
    let flow_id = Uuid::nil();
    let mut document = branch_document(flow_id);
    document["graph"]["edges"] = json!([document["graph"]["edges"][0]]);
    document["graph"]["nodes"][1]["bindings"]["branches"]["value"]["branches"][0]["condition"] = json!({
        "operator": "and",
        "conditions": []
    });

    let error = FlowCompiler::compile(flow_id, "draft-branches", &document, &compile_context())
        .expect_err("reachable if_else node with invalid branches should fail compile");

    assert!(error
        .to_string()
        .contains("if_else node node-if branch if must include a complete condition"));
}

#[test]
fn compile_if_else_rejects_multiple_else_branches() {
    let flow_id = Uuid::nil();
    let mut document = branch_document(flow_id);
    document["graph"]["nodes"][1]["bindings"]["branches"]["value"]["branches"]
        .as_array_mut()
        .expect("branches should be an array")
        .push(json!({
            "id": "else-2",
            "kind": "else",
            "title": "Else 2",
            "sourceHandle": "else-2"
        }));

    let error = FlowCompiler::compile(flow_id, "draft-branches", &document, &compile_context())
        .expect_err("if_else branch document with multiple else branches should fail compile");

    assert!(error
        .to_string()
        .contains("if_else node node-if must include only one else branch"));
}
