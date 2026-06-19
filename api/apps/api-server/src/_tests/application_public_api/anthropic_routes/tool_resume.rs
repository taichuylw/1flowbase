use super::*;

#[tokio::test]
async fn anthropic_messages_routes_hidden_system_reminder_tool_result_to_callback_resume() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Hidden Reminder Tool Result App").await;
    let before = flow_run_count(state.as_ref()).await;
    let callback_task_id = uuid::Uuid::from_u128(0xffffffffffffffffffffffffffffffff);
    let tool_use_id = encode_anthropic_callback_tool_use_id(callback_task_id, "toolu_read");

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {
                    "role": "assistant",
                    "content": [{
                        "type": "tool_use",
                        "id": tool_use_id,
                        "name": "Read",
                        "input": {}
                    }]
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": "Found 3 files"
                        },
                        {
                            "type": "text",
                            "text": "<system-reminder>Claude Code internal reminder</system-reminder>"
                        }
                    ]
                }
            ]
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["type"], json!("callback_task"));
    assert_eq!(flow_run_count(state.as_ref()).await, before);
}

#[tokio::test]
async fn anthropic_messages_routes_latest_message_only_tool_result_to_callback_resume() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Latest Tool Result App").await;
    let before = flow_run_count(state.as_ref()).await;
    let callback_task_id = uuid::Uuid::from_u128(0xffffffffffffffffffffffffffffffff);
    let tool_use_id = encode_anthropic_callback_tool_use_id(callback_task_id, "toolu_read");

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": "Found 3 files"
                    }]
                }
            ]
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["type"], json!("callback_task"));
    assert_eq!(flow_run_count(state.as_ref()).await, before);
}

#[tokio::test]
async fn anthropic_messages_rejects_orphan_tool_result_without_creating_run() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Orphan Tool Result App").await;
    let before = flow_run_count(state.as_ref()).await;

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {
                    "role": "assistant",
                    "content": [{
                        "type": "tool_use",
                        "id": "toolu_read",
                        "name": "Read",
                        "input": {}
                    }]
                },
                {
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": "toolu_read",
                        "content": "plain Anthropic tool result"
                    }]
                }
            ]
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["type"], json!("tool_result_only_orphan"));
    assert_eq!(flow_run_count(state.as_ref()).await, before);
}

#[tokio::test]
async fn anthropic_messages_matches_plain_tool_result_to_same_conversation_pending_callback() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Plain Tool Result Resume App").await;
    let before = flow_run_count(state.as_ref()).await;
    let session_id = "claude-code-session-plain-tool".to_string();
    let metadata = json!({
        "expand_id": "claude-code-user"
    });

    let first = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token.clone()),
        vec![("x-claude-code-session-id", session_id.clone())],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {"role": "user", "content": "uploads/image-1.png 这部分代码在哪里？"}
            ],
            "metadata": metadata
        }),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_payload = response_json(first).await;
    let run_id = Uuid::parse_str(
        first_payload["id"]
            .as_str()
            .expect("anthropic response id")
            .strip_prefix("msg_")
            .expect("anthropic response id should include msg_ prefix"),
    )
    .unwrap();
    seed_pending_anthropic_llm_callback(state.as_ref(), run_id, "toolu_read").await;

    let response = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        vec![("x-claude-code-session-id", session_id)],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {
                    "role": "assistant",
                    "content": [{
                        "type": "tool_use",
                        "id": "toolu_read",
                        "name": "Grep",
                        "input": {"pattern": "image-1.png"}
                    }]
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": "toolu_read",
                            "content": "No files found"
                        },
                        {
                            "type": "text",
                            "text": "<system-reminder>Claude Code internal reminder</system-reminder>"
                        }
                    ]
                }
            ],
            "metadata": metadata
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(flow_run_count(state.as_ref()).await, before + 1);
    let event_types = sqlx::query_scalar::<_, String>(
        "select event_type from flow_run_events where flow_run_id = $1 order by sequence asc",
    )
    .bind(run_id)
    .fetch_all(state.store.pool())
    .await
    .unwrap();
    assert!(
        event_types.contains(&"public_run_resume_requested".to_string()),
        "plain tool_result should route to callback resume, not a new run: {event_types:?}"
    );
}

#[tokio::test]
async fn anthropic_messages_resumes_embedded_encoded_tool_results_before_control_message() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Embedded Tool Result Control App").await;
    let before = flow_run_count(state.as_ref()).await;
    let session_id = "claude-code-session-embedded-tool-result".to_string();
    let metadata = json!({
        "expand_id": "claude-code-user"
    });

    let first = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token.clone()),
        vec![("x-claude-code-session-id", session_id.clone())],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {"role": "user", "content": "uploads/image-1.png 这部分代码在哪里？"}
            ],
            "metadata": metadata
        }),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_payload = response_json(first).await;
    let run_id = Uuid::parse_str(
        first_payload["id"]
            .as_str()
            .expect("anthropic response id")
            .strip_prefix("msg_")
            .expect("anthropic response id should include msg_ prefix"),
    )
    .unwrap();
    let callback_task = seed_pending_anthropic_llm_callback_with_tools(
        state.as_ref(),
        run_id,
        &["call_a", "call_b"],
    )
    .await;
    let encoded_a = encode_anthropic_callback_tool_use_id(callback_task.id, "call_a");
    let encoded_b = encode_anthropic_callback_tool_use_id(callback_task.id, "call_b");

    let response = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        vec![("x-claude-code-session-id", session_id)],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "tool_use",
                            "id": encoded_a,
                            "name": "Bash",
                            "input": {"command": "rg -l Navigation web/app/src"}
                        },
                        {
                            "type": "tool_use",
                            "id": encoded_b,
                            "name": "Bash",
                            "input": {"command": "rg -l AgentFlow web/app/src"}
                        }
                    ]
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": encoded_a,
                            "content": "web/app/src/app-shell/Navigation.tsx"
                        },
                        {
                            "type": "tool_result",
                            "tool_use_id": encoded_b,
                            "content": "web/app/src/features/applications/components/ApplicationCardGrid.tsx"
                        }
                    ]
                },
                {
                    "role": "user",
                    "content": "This session is being continued from a previous conversation that ran out of context. Continue the conversation from where it left off."
                }
            ],
            "metadata": metadata
        }),
    )
    .await;

    assert!(
        response.status().is_success() || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(flow_run_count(state.as_ref()).await, before + 1);
    let resume_payload: Value = sqlx::query_scalar(
        "select payload from flow_run_events where flow_run_id = $1 and event_type = 'public_run_resume_requested' order by sequence desc limit 1",
    )
    .bind(run_id)
    .fetch_one(state.store.pool())
    .await
    .unwrap();
    assert_eq!(
        resume_payload["response_payload"]["tool_results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool_result| tool_result["tool_call_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["call_a", "call_b"]
    );
}

#[tokio::test]
async fn anthropic_messages_routes_claude_code_plain_stdout_to_pending_callback_without_new_run() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Claude Code Stdout Resume App").await;
    let before = flow_run_count(state.as_ref()).await;
    let session_id = "claude-code-session-stdout".to_string();
    let metadata = json!({
        "expand_id": "claude-code-user"
    });

    let first = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token.clone()),
        vec![("x-claude-code-session-id", session_id.clone())],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {"role": "user", "content": "来 uploads\\test-01.png 找一下这幅图相关代码"}
            ],
            "metadata": metadata
        }),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_payload = response_json(first).await;
    let run_id = Uuid::parse_str(
        first_payload["id"]
            .as_str()
            .expect("anthropic response id")
            .strip_prefix("msg_")
            .expect("anthropic response id should include msg_ prefix"),
    )
    .unwrap();
    seed_pending_anthropic_llm_callback_with_tools(
        state.as_ref(),
        run_id,
        &["toolu_grep", "toolu_read"],
    )
    .await;

    let stdout = "No matches found\nFound 4 files\nweb\\app\\src\\routes\\route-config.ts\nweb\\app\\src\\app-shell\\Navigation.tsx";
    let response = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        vec![("x-claude-code-session-id", session_id)],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "tool_use",
                            "id": "toolu_grep",
                            "name": "Grep",
                            "input": {"pattern": "工作台|前台|子系统|工具"}
                        },
                        {
                            "type": "tool_use",
                            "id": "toolu_read",
                            "name": "Read",
                            "input": {"file_path": "web\\app\\src\\routes\\route-config.ts"}
                        }
                    ]
                },
                {"role": "user", "content": stdout}
            ],
            "metadata": metadata
        }),
    )
    .await;

    assert!(
        response.status().is_success() || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(flow_run_count(state.as_ref()).await, before + 1);
    let resume_payload: Value = sqlx::query_scalar(
        "select payload from flow_run_events where flow_run_id = $1 and event_type = 'public_run_resume_requested' order by sequence desc limit 1",
    )
    .bind(run_id)
    .fetch_one(state.store.pool())
    .await
    .unwrap();
    assert_eq!(
        resume_payload["response_payload"]["tool_results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool_result| tool_result["tool_call_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["toolu_grep", "toolu_read"]
    );
    assert_eq!(
        resume_payload["response_payload"]["tool_results"][0]["content"],
        json!(stdout)
    );
}

#[tokio::test]
async fn anthropic_messages_does_not_swallow_plain_user_question_as_stdout_continuation() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Plain User Question App").await;
    let before = flow_run_count(state.as_ref()).await;
    let session_id = "claude-code-session-user-question".to_string();
    let metadata = json!({
        "expand_id": "claude-code-user"
    });

    let first = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token.clone()),
        vec![("x-claude-code-session-id", session_id.clone())],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {"role": "user", "content": "uploads/image-1.png 这部分代码在哪里？"}
            ],
            "metadata": metadata
        }),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_payload = response_json(first).await;
    let run_id = Uuid::parse_str(
        first_payload["id"]
            .as_str()
            .expect("anthropic response id")
            .strip_prefix("msg_")
            .expect("anthropic response id should include msg_ prefix"),
    )
    .unwrap();
    seed_pending_anthropic_llm_callback(state.as_ref(), run_id, "toolu_grep").await;

    let response = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        vec![("x-claude-code-session-id", session_id)],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {"role": "user", "content": "这个结果不对，重新按 Navigation.tsx 查一下"}
            ],
            "metadata": metadata
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(flow_run_count(state.as_ref()).await, before + 2);
    let resume_count: i64 = sqlx::query_scalar(
        "select count(*) from flow_run_events where flow_run_id = $1 and event_type = 'public_run_resume_requested'",
    )
    .bind(run_id)
    .fetch_one(state.store.pool())
    .await
    .unwrap();
    assert_eq!(resume_count, 0);
}

#[tokio::test]
async fn anthropic_messages_does_not_swallow_plain_user_question_with_stdout_markers() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Plain User Question Marker App").await;
    let before = flow_run_count(state.as_ref()).await;
    let session_id = "claude-code-session-user-question-marker".to_string();
    let metadata = json!({
        "expand_id": "claude-code-user"
    });

    let first = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token.clone()),
        vec![("x-claude-code-session-id", session_id.clone())],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {"role": "user", "content": "uploads/image-1.png 这部分代码在哪里？"}
            ],
            "metadata": metadata
        }),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_payload = response_json(first).await;
    let run_id = Uuid::parse_str(
        first_payload["id"]
            .as_str()
            .expect("anthropic response id")
            .strip_prefix("msg_")
            .expect("anthropic response id should include msg_ prefix"),
    )
    .unwrap();
    seed_pending_anthropic_llm_callback(state.as_ref(), run_id, "toolu_grep").await;

    let response = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        vec![("x-claude-code-session-id", session_id)],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {
                    "role": "user",
                    "content": "Found 不是工具输出，这里是我在追问：为什么没看到 Navigation.tsx？"
                }
            ],
            "metadata": metadata
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(flow_run_count(state.as_ref()).await, before + 2);
    let resume_count: i64 = sqlx::query_scalar(
        "select count(*) from flow_run_events where flow_run_id = $1 and event_type = 'public_run_resume_requested'",
    )
    .bind(run_id)
    .fetch_one(state.store.pool())
    .await
    .unwrap();
    assert_eq!(resume_count, 0);
}
