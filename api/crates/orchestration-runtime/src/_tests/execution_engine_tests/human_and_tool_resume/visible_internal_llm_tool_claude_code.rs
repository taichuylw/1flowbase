use super::visible_internal_llm_tool_fixtures::*;
use super::*;

#[tokio::test]
async fn visible_internal_llm_tool_is_hidden_for_claude_code_control_runs() {
    let (invoker, captured_inputs) = sequential_tool_invoker(vec![final_llm_response("summary")]);
    let plan = visible_internal_llm_tool_plan();

    let outcome = start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": "Your task is to create a detailed summary of the conversation so far",
                "compatibility": {
                    "claude_code_control": "compact_summary"
                },
                "tools": [
                    {
                        "name": "Bash",
                        "description": "Run a shell command",
                        "input_schema": {
                            "type": "object",
                            "properties": {
                                "command": { "type": "string" }
                            },
                            "required": ["command"]
                        }
                    }
                ]
            }
        }),
        &invoker,
    )
    .await
    .unwrap();

    assert!(
        matches!(outcome.stop_reason, ExecutionStopReason::Completed),
        "expected completed control run, got {:?}",
        outcome.stop_reason
    );
    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 1);
    assert!(
        captured[0].tools.is_empty(),
        "control runs must not expose client or visible internal LLM tools"
    );
}

#[tokio::test]
async fn visible_internal_llm_tool_stays_available_for_claude_code_compact_resume() {
    let (invoker, captured_inputs) = sequential_tool_invoker(vec![final_llm_response("resume")]);
    let plan = visible_internal_llm_tool_plan();

    let outcome = start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": "This session is being continued from a previous conversation that ran out of context.\n\nIf you need specific details from before compaction, use the summary.",
                "compatibility": {
                    "claude_code_control": "compact_resume"
                },
                "tools": [
                    {
                        "name": "Bash",
                        "description": "Run a shell command",
                        "input_schema": {
                            "type": "object",
                            "properties": {
                                "command": { "type": "string" }
                            },
                            "required": ["command"]
                        }
                    }
                ]
            }
        }),
        &invoker,
    )
    .await
    .unwrap();

    assert!(
        matches!(outcome.stop_reason, ExecutionStopReason::Completed),
        "expected completed compact resume run, got {:?}",
        outcome.stop_reason
    );
    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    let tool_names = captured[0]
        .tools
        .iter()
        .filter_map(|tool| tool["function"]["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(tool_names, vec!["Bash", "inspect_visible_context"]);
}
