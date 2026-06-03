use super::*;

#[test]
fn compile_if_else_branches_preserves_source_handles_and_condition_selectors() {
    let flow_id = Uuid::nil();
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
    let document = json!({
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
    });

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
