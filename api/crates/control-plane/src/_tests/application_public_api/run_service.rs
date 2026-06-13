use control_plane::application_public_api::{
    api_keys::{ApplicationApiKeyService, CreateApplicationApiKeyCommand},
    mapping::{
        ApplicationApiMappingConfig, ApplicationApiMappingInput, ApplicationApiMappingOutput,
    },
    native::{
        CreateNativeRunCommand, NativeProtocolRequestKind, NativeRunRequest,
        NativeRunValidationError,
    },
    publications::{ApplicationPublicationService, PublishApplicationCommand},
    run_service::{ApplicationPublishedRunControlRepository, ApplicationPublishedRunService},
    ApplicationPublicApiTestHarness,
};
use control_plane::ports::{
    ApplicationEnvironmentVariableInput, ApplicationRepository, FlowRepository,
    ReplaceApplicationEnvironmentVariablesInput,
};
use serde_json::json;
use uuid::Uuid;

fn actor_user_id() -> Uuid {
    Uuid::from_u128(0x11111111111111111111111111111111)
}

fn native_request(response_mode: &str, idempotency_key: Option<&str>) -> NativeRunRequest {
    let execution = idempotency_key
        .map(|key| json!({ "idempotency_key": key }))
        .unwrap_or_else(|| json!({}));
    serde_json::from_value(json!({
        "query": "Summarize the incident",
        "model": "public-model/pass-through",
        "inputs": {
            "priority": "high"
        },
        "conversation": {
            "id": "conversation-1",
            "user": "customer-1"
        },
        "response_mode": response_mode,
        "execution": execution,
        "metadata": {
            "trace_id": "trace-1",
            "request_id": "req-1"
        },
        "compatibility_mode": "native-v1"
    }))
    .unwrap()
}

fn anthropic_request(query: &str) -> NativeRunRequest {
    let mut request: NativeRunRequest = serde_json::from_value(json!({
        "query": query,
        "model": "public-model/pass-through",
        "conversation": {
            "id": "3e7058c2-3120-4222-bb14-c99ec85e1c0f",
            "user": "user_31fb5a_account__session_3e7058c2-3120-4222-bb14-c99ec85e1c0f"
        },
        "response_mode": "streaming",
        "compatibility_mode": "anthropic-messages-v1"
    }))
    .unwrap();
    request.protocol_compatibility_mode = Some("anthropic-messages-v1".to_string());
    request
}

fn anthropic_subagent_request(query: &str) -> NativeRunRequest {
    let mut request = anthropic_request(query);
    request.system = Some(
        "x-anthropic-billing-header: cc_version=2.1.165; cc_entrypoint=cli; cch=007d6; cc_is_subagent=true;\n\nYou are Claude Code."
            .to_string(),
    );
    request
}

fn anthropic_builtin_agent_request(query: &str) -> NativeRunRequest {
    let mut request = anthropic_request(query);
    request.system = Some(
        "x-anthropic-billing-header: cc_version=2.1.141; cc_entrypoint=cli; cch=04e8f;\n\n\
You are Claude Code, Anthropic's official CLI for Claude.\n\n\
You are a file search specialist for Claude Code, Anthropic's official CLI for Claude.\n\n\
Notes:\n\
- Agent threads always have their cwd reset between bash calls, as a result please only use absolute file paths.\n\
- Do NOT Write report/summary/findings/analysis .md files. Return findings directly as your final assistant message — the parent agent reads your text output, not files you create."
            .to_string(),
    );
    request
}

fn anthropic_tool_result_continuation_request(query: &str) -> NativeRunRequest {
    let mut request = anthropic_request(query);
    request.protocol_request_kind =
        Some(NativeProtocolRequestKind::AnthropicToolResultContinuation);
    request
}

fn anthropic_claude_code_control_request(query: &str, control_kind: &str) -> NativeRunRequest {
    let mut request = anthropic_request(query);
    request.inputs = serde_json::from_value(json!({
        "compatibility": {
            "claude_code_control": control_kind
        }
    }))
    .unwrap();
    request
}

fn anthropic_away_summary_request() -> NativeRunRequest {
    anthropic_request(
        "The user stepped away and is coming back. Write exactly 1-3 short sentences. Start by stating the high-level task — what they are building or debugging, not implementation details. Next: the concrete next step. Skip status reports and commit recaps.",
    )
}

fn anthropic_compact_resume_request() -> NativeRunRequest {
    anthropic_request(
        "This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.\n\nSummary:\n- user asked where uploads/image-1.png is implemented\n\nContinue the conversation from where it left off without asking the user any further questions. Resume directly — do not acknowledge the summary, do not recap what was happening, do not preface with \"I'll continue\" or similar. Pick up the last task as if the break never happened.",
    )
}

fn native_request_with_model_parameters(
    model: &str,
    model_parameters: serde_json::Value,
) -> NativeRunRequest {
    serde_json::from_value(json!({
        "query": "Summarize the incident",
        "model": model,
        "inputs": {
            "priority": "high"
        },
        "execution": {
            "model_parameters": model_parameters
        }
    }))
    .unwrap()
}

fn published_mapping() -> ApplicationApiMappingConfig {
    ApplicationApiMappingConfig {
        input: ApplicationApiMappingInput {
            query_target: "node-start.query".into(),
            model_target: None,
            inputs_target: Some("node-start".into()),
            history_target: None,
            attachments_target: None,
        },
        output: ApplicationApiMappingOutput::default(),
    }
}

async fn issue_key(harness: &ApplicationPublicApiTestHarness, application_id: Uuid) -> String {
    ApplicationApiKeyService::new(harness.repository())
        .create_api_key(CreateApplicationApiKeyCommand {
            actor_user_id: actor_user_id(),
            application_id,
            name: "Native runner".into(),
            expires_at: None,
        })
        .await
        .unwrap()
        .token
}

async fn save_start_model_catalog(
    repository: &control_plane::application_public_api::ApplicationPublicApiTestRepository,
    application: &domain::ApplicationRecord,
) {
    let editor_state = repository
        .get_or_create_editor_state(application.workspace_id, application.id, actor_user_id())
        .await
        .unwrap();
    let mut document = editor_state.draft.document;
    let start_node = document["graph"]["nodes"]
        .as_array_mut()
        .expect("nodes array")
        .iter_mut()
        .find(|node| node["type"] == "start")
        .expect("default document has a start node");
    start_node["config"]["model_list"] = json!([
        {
            "id": "gpt-5.4",
            "name": "GPT-5.4",
            "max_output_tokens": 32000,
            "capabilities": {
                "reasoning": true
            },
            "reasoning": {
                "default_effort": "medium",
                "supported_efforts": ["low", "medium", "high"]
            }
        },
        {
            "id": "plain-model",
            "name": "Plain model"
        }
    ]);
    FlowRepository::save_draft(
        repository,
        application.workspace_id,
        application.id,
        actor_user_id(),
        document,
        domain::FlowChangeKind::Logical,
        "Configure published model catalog",
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn start_native_run_creates_published_api_flow_run_from_frozen_publication() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Published Native App");
    let token = issue_key(&harness, application.id).await;
    let publication = ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let result = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: native_request("streaming", None),
        })
        .await
        .unwrap();
    let flow_run = repository
        .get_flow_run(application.id, result.id)
        .await
        .unwrap()
        .expect("published flow run should be durable");

    assert_eq!(flow_run.run_mode, domain::FlowRunMode::PublishedApiRun);
    assert_eq!(flow_run.created_by, actor_user_id());
    assert_eq!(flow_run.flow_id, publication.flow_id);
    assert_eq!(
        flow_run.compiled_plan_id,
        Some(publication.compiled_plan_id)
    );
    assert_eq!(
        flow_run.flow_schema_version,
        publication.flow_schema_version
    );
    assert_eq!(flow_run.document_hash, publication.document_hash);
    assert_eq!(flow_run.publication_version_id, Some(publication.id));
    assert_eq!(flow_run.title, "Summarize the incident");
    assert_eq!(flow_run.external_user.as_deref(), Some("customer-1"));
    assert_eq!(
        flow_run.external_conversation_id.as_deref(),
        Some("conversation-1")
    );
    assert_eq!(flow_run.external_trace_id.as_deref(), Some("trace-1"));
    assert!(flow_run.compatibility_mode.is_none());
    assert_eq!(
        flow_run.input_payload,
        json!({
            "env": {},
            "node-start": {
                "query": "Summarize the incident",
                "priority": "high"
            }
        })
    );
    assert_eq!(result.metadata["model"], json!("public-model/pass-through"));
}

#[tokio::test]
async fn start_native_run_freezes_valid_external_reasoning_parameters_for_runtime() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Published Native Reasoning App");
    let token = issue_key(&harness, application.id).await;
    save_start_model_catalog(&repository, &application).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let result = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: serde_json::from_value(json!({
                "query": "Summarize the incident",
                "model": "gpt-5.4",
                "inputs": {
                    "priority": "high"
                },
                "execution": {
                    "model_parameters": {
                        "reasoning": {
                            "enabled": true,
                            "effort": "high",
                            "budget_tokens": 4096
                        }
                    }
                }
            }))
            .unwrap(),
        })
        .await
        .unwrap();
    let flow_run = repository
        .get_flow_run(application.id, result.id)
        .await
        .unwrap()
        .expect("published flow run should be durable");

    assert_eq!(
        flow_run.input_payload["sys"]["model_parameters"],
        json!({
            "reasoning": {
                "enabled": true,
                "effort": "high",
                "budget_tokens": 4096
            }
        })
    );
    assert_eq!(
        flow_run.input_payload["node-start"]["reasoning_effort"],
        json!("high")
    );
    assert!(flow_run.input_payload["sys"]
        .get("reasoning_effort")
        .is_none());
}

#[tokio::test]
async fn start_native_run_rejects_context_window_as_runtime_model_parameter() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Published Native Context App");
    let token = issue_key(&harness, application.id).await;
    save_start_model_catalog(&repository, &application).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let result = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: native_request_with_model_parameters(
                "gpt-5.4",
                json!({
                    "context_window": 128000
                }),
            ),
        })
        .await;

    assert_eq!(
        result,
        Err(NativeRunValidationError::InvalidModelParameters(
            "execution.model_parameters"
        ))
    );
}

#[tokio::test]
async fn start_native_run_rejects_external_reasoning_for_unknown_model() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Published Native Unknown App");
    let token = issue_key(&harness, application.id).await;
    save_start_model_catalog(&repository, &application).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let result = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: native_request_with_model_parameters(
                "missing-model",
                json!({
                    "reasoning": {
                        "enabled": true,
                        "effort": "high"
                    }
                }),
            ),
        })
        .await;

    assert_eq!(
        result,
        Err(NativeRunValidationError::InvalidModelParameters("model"))
    );
}

#[tokio::test]
async fn start_native_run_rejects_external_reasoning_for_unsupported_model() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Published Native Plain App");
    let token = issue_key(&harness, application.id).await;
    save_start_model_catalog(&repository, &application).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let result = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: native_request_with_model_parameters(
                "plain-model",
                json!({
                    "reasoning": {
                        "enabled": true,
                        "effort": "high"
                    }
                }),
            ),
        })
        .await;

    assert_eq!(
        result,
        Err(NativeRunValidationError::InvalidModelParameters(
            "execution.model_parameters.reasoning"
        ))
    );
}

#[tokio::test]
async fn start_native_run_rejects_unsupported_reasoning_effort() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Published Native Effort App");
    let token = issue_key(&harness, application.id).await;
    save_start_model_catalog(&repository, &application).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let result = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: native_request_with_model_parameters(
                "gpt-5.4",
                json!({
                    "reasoning": {
                        "enabled": true,
                        "effort": "xhigh"
                    }
                }),
            ),
        })
        .await;

    assert_eq!(
        result,
        Err(NativeRunValidationError::InvalidModelParameters(
            "execution.model_parameters.reasoning.effort"
        ))
    );
}

#[tokio::test]
async fn start_native_run_rejects_reasoning_budget_over_model_output_limit() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Published Native Budget App");
    let token = issue_key(&harness, application.id).await;
    save_start_model_catalog(&repository, &application).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let result = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: native_request_with_model_parameters(
                "gpt-5.4",
                json!({
                    "reasoning": {
                        "enabled": true,
                        "effort": "high",
                        "budget_tokens": 32001
                    }
                }),
            ),
        })
        .await;

    assert_eq!(
        result,
        Err(NativeRunValidationError::InvalidModelParameters(
            "execution.model_parameters.reasoning.budget_tokens"
        ))
    );
}

#[tokio::test]
async fn start_native_run_freezes_application_environment_variables() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Published Native Env App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    ApplicationRepository::replace_application_environment_variables(
        &repository,
        &ReplaceApplicationEnvironmentVariablesInput {
            actor_user_id: actor_user_id(),
            workspace_id: application.workspace_id,
            application_id: application.id,
            variables: vec![ApplicationEnvironmentVariableInput {
                name: "ApiBaseUrl".into(),
                value_type: "string".into(),
                value: json!("https://api.at-start.example.com"),
                description: "Native API base URL".into(),
            }],
        },
    )
    .await
    .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let result = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: native_request("streaming", None),
        })
        .await
        .unwrap();
    ApplicationRepository::replace_application_environment_variables(
        &repository,
        &ReplaceApplicationEnvironmentVariablesInput {
            actor_user_id: actor_user_id(),
            workspace_id: application.workspace_id,
            application_id: application.id,
            variables: vec![ApplicationEnvironmentVariableInput {
                name: "ApiBaseUrl".into(),
                value_type: "string".into(),
                value: json!("https://api.changed.example.com"),
                description: "Changed Native API base URL".into(),
            }],
        },
    )
    .await
    .unwrap();
    let flow_run = repository
        .get_flow_run(application.id, result.id)
        .await
        .unwrap()
        .expect("published flow run should be durable");

    assert_eq!(
        flow_run.input_payload["env"]["ApiBaseUrl"],
        json!("https://api.at-start.example.com")
    );
}

#[tokio::test]
async fn start_native_run_uses_expand_id_and_truncates_title() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Expanded Native User App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());
    let long_query = "Q".repeat(300);
    let expected_title = "Q".repeat(255);

    let result = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: serde_json::from_value(json!({
                "query": long_query,
                "model": "public-model/pass-through",
                "inputs": {
                    "priority": "high"
                },
                "expand_id": "customer-alias-1",
                "response_mode": "blocking",
                "execution": {},
                "metadata": {
                    "trace_id": "trace-1"
                }
            }))
            .unwrap(),
        })
        .await
        .unwrap();
    let flow_run = repository
        .get_flow_run(application.id, result.id)
        .await
        .unwrap()
        .expect("published flow run should be durable");

    assert_eq!(flow_run.external_user.as_deref(), Some("customer-alias-1"));
    assert!(flow_run
        .external_conversation_id
        .as_deref()
        .is_some_and(|value| value.starts_with("conv_")));
    assert_eq!(flow_run.title, expected_title);
    assert_eq!(result.metadata["expand_id"], json!("customer-alias-1"));
    assert!(result.metadata.get("user_id").is_none());
}

#[tokio::test]
async fn start_native_run_replays_existing_run_for_same_idempotency_key() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Idempotent Native App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let first = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: native_request("blocking", Some("idem-1")),
        })
        .await
        .unwrap();
    let second = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: native_request("blocking", Some("idem-1")),
        })
        .await
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(repository.flow_run_count(), 1);
}

#[tokio::test]
async fn start_native_run_rejects_same_idempotency_key_with_different_request() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Idempotent Native App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());
    service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: native_request("blocking", Some("idem-conflict")),
        })
        .await
        .unwrap();
    let mut changed_request = native_request("blocking", Some("idem-conflict"));
    changed_request.query = "Summarize a different incident".to_string();

    let error = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: changed_request,
        })
        .await
        .unwrap_err();

    assert_eq!(error, NativeRunValidationError::IdempotencyConflict);
    assert_eq!(repository.flow_run_count(), 1);
}

#[tokio::test]
async fn start_anthropic_run_cancels_previous_waiting_callback_in_same_conversation() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Anthropic Session App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let first = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: anthropic_request("hi"),
        })
        .await
        .unwrap();
    let callback_task = repository.seed_pending_callback_task(first.id);

    let second = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: anthropic_request("new message"),
        })
        .await
        .unwrap();

    assert_ne!(first.id, second.id);
    let first_run = repository
        .get_flow_run(application.id, first.id)
        .await
        .unwrap()
        .expect("first run should remain durable");
    let callback_task = repository
        .get_published_callback_task(callback_task.id)
        .await
        .unwrap()
        .expect("callback task should remain durable");
    assert_eq!(first_run.status, domain::FlowRunStatus::Cancelled);
    assert_eq!(callback_task.status, domain::CallbackTaskStatus::Cancelled);
    let first_run_events = repository.run_event_types(first.id);
    assert!(first_run_events.contains(&"public_run_cancelled".to_string()));
    assert!(first_run_events.contains(&"public_run_callback_cancelled".to_string()));
}

#[tokio::test]
async fn start_anthropic_subagent_run_keeps_parent_waiting_callback_alive() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Anthropic Subagent App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let parent = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: anthropic_request("uploads\\test-01.png 找一下这幅图相关代码"),
        })
        .await
        .unwrap();
    let callback_task = repository.seed_pending_callback_task(parent.id);

    let subagent = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: anthropic_subagent_request("Find nav bar code"),
        })
        .await
        .unwrap();

    assert_ne!(parent.id, subagent.id);
    let parent_run = repository
        .get_flow_run(application.id, parent.id)
        .await
        .unwrap()
        .expect("parent run should remain durable");
    let callback_task = repository
        .get_published_callback_task(callback_task.id)
        .await
        .unwrap()
        .expect("callback task should remain durable");
    assert_eq!(parent_run.status, domain::FlowRunStatus::WaitingCallback);
    assert_eq!(callback_task.status, domain::CallbackTaskStatus::Pending);
    let parent_run_events = repository.run_event_types(parent.id);
    assert!(!parent_run_events.contains(&"public_run_cancelled".to_string()));
    assert!(!parent_run_events.contains(&"public_run_callback_cancelled".to_string()));
}

#[tokio::test]
async fn start_anthropic_builtin_agent_run_keeps_parent_waiting_callback_alive() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Anthropic Builtin Agent App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let parent = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: anthropic_request("uploads/image-1.png 这部分代码在哪里？"),
        })
        .await
        .unwrap();
    let callback_task = repository.seed_pending_callback_task(parent.id);

    let agent = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: anthropic_builtin_agent_request(
                "在 /home/taichu/git/1flowbase 项目中，找到工作台页面相关的前端代码。",
            ),
        })
        .await
        .unwrap();

    assert_ne!(parent.id, agent.id);
    let parent_run = repository
        .get_flow_run(application.id, parent.id)
        .await
        .unwrap()
        .expect("parent run should remain durable");
    let callback_task = repository
        .get_published_callback_task(callback_task.id)
        .await
        .unwrap()
        .expect("callback task should remain durable");
    assert_eq!(parent_run.status, domain::FlowRunStatus::WaitingCallback);
    assert_eq!(callback_task.status, domain::CallbackTaskStatus::Pending);
    let parent_run_events = repository.run_event_types(parent.id);
    assert!(!parent_run_events.contains(&"public_run_cancelled".to_string()));
    assert!(!parent_run_events.contains(&"public_run_callback_cancelled".to_string()));
}

#[tokio::test]
async fn start_anthropic_tool_result_continuation_keeps_parent_waiting_callback_alive() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application =
        harness.seed_application(actor_user_id(), "Anthropic Tool Result Continuation App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let parent = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: anthropic_request("uploads\\test-01.png 找一下这幅图相关代码"),
        })
        .await
        .unwrap();
    let callback_task = repository.seed_pending_callback_task(parent.id);

    let continuation = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: anthropic_tool_result_continuation_request(
                "-rw-r--r-- 1 Lw 197121 17907 Jun 12 15:25 uploads/test-01.png",
            ),
        })
        .await
        .unwrap();

    assert_ne!(parent.id, continuation.id);
    let parent_run = repository
        .get_flow_run(application.id, parent.id)
        .await
        .unwrap()
        .expect("parent run should remain durable");
    let callback_task = repository
        .get_published_callback_task(callback_task.id)
        .await
        .unwrap()
        .expect("callback task should remain durable");
    assert_eq!(parent_run.status, domain::FlowRunStatus::WaitingCallback);
    assert_eq!(callback_task.status, domain::CallbackTaskStatus::Pending);
    let parent_run_events = repository.run_event_types(parent.id);
    assert!(!parent_run_events.contains(&"public_run_cancelled".to_string()));
    assert!(!parent_run_events.contains(&"public_run_callback_cancelled".to_string()));
}

#[tokio::test]
async fn start_anthropic_claude_code_control_run_keeps_parent_waiting_callback_alive() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application =
        harness.seed_application(actor_user_id(), "Anthropic Claude Code Control App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let parent = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: anthropic_request("uploads/image-1.png 这部分代码在哪里？"),
        })
        .await
        .unwrap();
    let callback_task = repository.seed_pending_callback_task(parent.id);

    let control = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: anthropic_claude_code_control_request(
                "CRITICAL: Respond with TEXT ONLY. Do NOT call any tools.",
                "compact_summary",
            ),
        })
        .await
        .unwrap();

    assert_ne!(parent.id, control.id);
    let parent_run = repository
        .get_flow_run(application.id, parent.id)
        .await
        .unwrap()
        .expect("parent run should remain durable");
    let callback_task = repository
        .get_published_callback_task(callback_task.id)
        .await
        .unwrap()
        .expect("callback task should remain durable");
    assert_eq!(parent_run.status, domain::FlowRunStatus::WaitingCallback);
    assert_eq!(callback_task.status, domain::CallbackTaskStatus::Pending);
    let parent_run_events = repository.run_event_types(parent.id);
    assert!(!parent_run_events.contains(&"public_run_cancelled".to_string()));
    assert!(!parent_run_events.contains(&"public_run_callback_cancelled".to_string()));
}

#[tokio::test]
async fn start_anthropic_away_summary_run_keeps_parent_waiting_callback_alive() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Anthropic Away Summary App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let parent = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: anthropic_request("uploads/image-1.png 这部分代码在哪里？"),
        })
        .await
        .unwrap();
    let callback_task = repository.seed_pending_callback_task(parent.id);

    let control = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: anthropic_away_summary_request(),
        })
        .await
        .unwrap();

    assert_ne!(parent.id, control.id);
    let parent_run = repository
        .get_flow_run(application.id, parent.id)
        .await
        .unwrap()
        .expect("parent run should remain durable");
    let callback_task = repository
        .get_published_callback_task(callback_task.id)
        .await
        .unwrap()
        .expect("callback task should remain durable");
    assert_eq!(parent_run.status, domain::FlowRunStatus::WaitingCallback);
    assert_eq!(callback_task.status, domain::CallbackTaskStatus::Pending);
    let parent_run_events = repository.run_event_types(parent.id);
    assert!(!parent_run_events.contains(&"public_run_cancelled".to_string()));
    assert!(!parent_run_events.contains(&"public_run_callback_cancelled".to_string()));
}

#[tokio::test]
async fn start_anthropic_compact_resume_run_keeps_parent_waiting_callback_alive() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Anthropic Compact Resume App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let parent = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: anthropic_request("uploads/image-1.png 这部分代码在哪里？"),
        })
        .await
        .unwrap();
    let callback_task = repository.seed_pending_callback_task(parent.id);

    let control = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: anthropic_compact_resume_request(),
        })
        .await
        .unwrap();

    assert_ne!(parent.id, control.id);
    let parent_run = repository
        .get_flow_run(application.id, parent.id)
        .await
        .unwrap()
        .expect("parent run should remain durable");
    let callback_task = repository
        .get_published_callback_task(callback_task.id)
        .await
        .unwrap()
        .expect("callback task should remain durable");
    assert_eq!(parent_run.status, domain::FlowRunStatus::WaitingCallback);
    assert_eq!(callback_task.status, domain::CallbackTaskStatus::Pending);
    let parent_run_events = repository.run_event_types(parent.id);
    assert!(!parent_run_events.contains(&"public_run_cancelled".to_string()));
    assert!(!parent_run_events.contains(&"public_run_callback_cancelled".to_string()));
}

#[tokio::test]
async fn start_anthropic_compact_resume_run_cancels_previous_control_waiting_callback_only() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application =
        harness.seed_application(actor_user_id(), "Anthropic Compact Control Cleanup App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let parent = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: anthropic_request("uploads/image-1.png 这部分代码在哪里？"),
        })
        .await
        .unwrap();
    let parent_callback = repository.seed_pending_callback_task(parent.id);
    let old_control = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: anthropic_compact_resume_request(),
        })
        .await
        .unwrap();
    let old_control_callback = repository.seed_pending_callback_task(old_control.id);

    let next_control = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: anthropic_compact_resume_request(),
        })
        .await
        .unwrap();

    assert_ne!(old_control.id, next_control.id);
    let parent_run = repository
        .get_flow_run(application.id, parent.id)
        .await
        .unwrap()
        .expect("parent run should remain durable");
    let parent_callback = repository
        .get_published_callback_task(parent_callback.id)
        .await
        .unwrap()
        .expect("parent callback task should remain durable");
    assert_eq!(parent_run.status, domain::FlowRunStatus::WaitingCallback);
    assert_eq!(parent_callback.status, domain::CallbackTaskStatus::Pending);
    let old_control_run = repository
        .get_flow_run(application.id, old_control.id)
        .await
        .unwrap()
        .expect("old control run should remain durable");
    let old_control_callback = repository
        .get_published_callback_task(old_control_callback.id)
        .await
        .unwrap()
        .expect("old control callback task should remain durable");
    assert_eq!(old_control_run.status, domain::FlowRunStatus::Cancelled);
    assert_eq!(
        old_control_callback.status,
        domain::CallbackTaskStatus::Cancelled
    );
    let parent_run_events = repository.run_event_types(parent.id);
    assert!(!parent_run_events.contains(&"public_run_cancelled".to_string()));
    assert!(!parent_run_events.contains(&"public_run_callback_cancelled".to_string()));
    let old_control_events = repository.run_event_types(old_control.id);
    assert!(old_control_events.contains(&"public_run_cancelled".to_string()));
    assert!(old_control_events.contains(&"public_run_callback_cancelled".to_string()));
}

#[tokio::test]
async fn start_native_run_does_not_trust_request_compatibility_mode_for_anthropic_cancellation() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Native Forged Compat App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository.clone());

    let first = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: anthropic_request("hi"),
        })
        .await
        .unwrap();
    let callback_task = repository.seed_pending_callback_task(first.id);

    let forged_native_request = serde_json::from_value(json!({
        "query": "Native caller should not own Anthropic cancellation policy",
        "model": "public-model/pass-through",
        "conversation": {
            "id": "3e7058c2-3120-4222-bb14-c99ec85e1c0f",
            "user": "user_31fb5a_account__session_3e7058c2-3120-4222-bb14-c99ec85e1c0f"
        },
        "response_mode": "blocking",
        "compatibility_mode": "anthropic-messages-v1",
        "execution": {
            "compatibility_mode": "anthropic-messages-v1"
        },
        "metadata": {
            "compatibility_mode": "anthropic-messages-v1"
        }
    }))
    .unwrap();
    let second = service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: forged_native_request,
        })
        .await
        .unwrap();

    assert_ne!(first.id, second.id);
    let first_run = repository
        .get_flow_run(application.id, first.id)
        .await
        .unwrap()
        .expect("first run should remain durable");
    let callback_task = repository
        .get_published_callback_task(callback_task.id)
        .await
        .unwrap()
        .expect("callback task should remain durable");
    assert_eq!(first_run.status, domain::FlowRunStatus::WaitingCallback);
    assert_eq!(callback_task.status, domain::CallbackTaskStatus::Pending);
    let first_run_events = repository.run_event_types(first.id);
    assert!(!first_run_events.contains(&"public_run_cancelled".to_string()));
    assert!(!first_run_events.contains(&"public_run_callback_cancelled".to_string()));
}

#[tokio::test]
async fn start_native_run_does_not_read_editor_state_after_publication() {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let application = harness.seed_application(actor_user_id(), "Frozen Native App");
    let token = issue_key(&harness, application.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: application.id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
    repository.reset_editor_state_read_count();
    let service = ApplicationPublishedRunService::new(repository.clone());

    service
        .start_native_run(CreateNativeRunCommand {
            bearer_token: token,
            request: native_request("streaming", None),
        })
        .await
        .unwrap();

    assert_eq!(repository.editor_state_read_count(), 0);
}

#[tokio::test]
async fn start_native_run_returns_application_not_published_for_unpublished_or_disabled_application(
) {
    let harness = ApplicationPublicApiTestHarness::new();
    let repository = harness.repository();
    let unpublished = harness.seed_application(actor_user_id(), "Unpublished App");
    let unpublished_token = issue_key(&harness, unpublished.id).await;
    let disabled = harness.seed_application(actor_user_id(), "Disabled App");
    let disabled_token = issue_key(&harness, disabled.id).await;
    ApplicationPublicationService::new(repository.clone())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: actor_user_id(),
            application_id: disabled.id,
            mapping: published_mapping(),
            api_enabled: false,
        })
        .await
        .unwrap();
    let service = ApplicationPublishedRunService::new(repository);

    for token in [unpublished_token, disabled_token] {
        let error = service
            .start_native_run(CreateNativeRunCommand {
                bearer_token: token,
                request: native_request("blocking", None),
            })
            .await
            .unwrap_err();

        assert_eq!(error, NativeRunValidationError::ApplicationNotPublished);
    }
}
