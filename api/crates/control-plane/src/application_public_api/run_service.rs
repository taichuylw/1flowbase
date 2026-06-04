use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Map, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{
    api_keys::{ApplicationApiKeyActor, ApplicationApiKeyService},
    conversations::{
        ApplicationPublicConversationMessageRecord, ApplicationPublicConversationRepository,
        BindApplicationPublicConversationInput, ListApplicationPublicConversationMessagesInput,
    },
    model_catalog::{extract_agent_model_catalog_from_start_node, find_agent_model},
    native::{
        CreateNativeRunCommand, NativeInputMapper, NativeRunRequest, NativeRunResult,
        NativeRunStatus, NativeRunValidationError,
    },
    publications::ApplicationPublicationVersionRecord,
};
use crate::{
    audit::audit_log,
    flow_run_title::build_flow_run_title,
    ports::{
        ApiKeyRepository, ApplicationCompiledPlanRepository, ApplicationPublicationRepository,
        ApplicationRepository, AuthRepository, CacheStore, CreateFlowRunInput,
    },
    state_transition::ensure_flow_run_transition,
};

const APPLICATION_PUBLIC_CONVERSATION_HISTORY_LIMIT: i64 = 50;

pub struct ApplicationPublishedRunService<R> {
    repository: R,
    last_used_cache: Option<Arc<dyn CacheStore>>,
}

impl<R> ApplicationPublishedRunService<R>
where
    R: ApplicationRepository
        + ApiKeyRepository
        + AuthRepository
        + ApplicationPublicationRepository
        + ApplicationCompiledPlanRepository
        + ApplicationPublishedFlowRunRepository
        + ApplicationPublishedRunControlRepository
        + ApplicationPublicConversationRepository
        + Clone,
{
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            last_used_cache: None,
        }
    }

    pub fn with_last_used_cache(mut self, cache: Arc<dyn CacheStore>) -> Self {
        self.last_used_cache = Some(cache);
        self
    }

    pub async fn start_native_run(
        &self,
        command: CreateNativeRunCommand,
    ) -> std::result::Result<NativeRunResult, NativeRunValidationError> {
        let mut api_key_service = ApplicationApiKeyService::new(self.repository.clone());
        if let Some(cache) = &self.last_used_cache {
            api_key_service = api_key_service.with_last_used_cache(cache.clone());
        }
        let actor = api_key_service
            .authenticate_bearer_token(&command.bearer_token)
            .await
            .map_err(|_| NativeRunValidationError::NotAuthenticated)?;
        self.ensure_application_exists(&actor).await?;

        let publication = self.load_enabled_publication(&actor).await?;
        let request = self
            .bind_conversation(actor.application_id, actor.api_key_id, command.request)
            .await?;
        let external_model_parameters =
            validate_external_model_parameters(&request, &publication.document_snapshot)?;

        let compiled_plan = self
            .repository
            .get_application_compiled_plan(publication.compiled_plan_id)
            .await
            .map_err(|_| NativeRunValidationError::ApplicationNotPublished)?
            .ok_or(NativeRunValidationError::ApplicationNotPublished)?;

        let mapped = NativeInputMapper::map(&request, &publication.mapping_snapshot)
            .map_err(|_| NativeRunValidationError::InvalidMapping)?;
        let metadata = mapped.metadata;
        let idempotency_key = metadata
            .get("idempotency_key")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        if let Some(idempotency_key) = idempotency_key.as_deref() {
            if let Some(flow_run) = self
                .repository
                .find_published_flow_run_by_idempotency_key(
                    actor.application_id,
                    actor.api_key_id,
                    idempotency_key,
                )
                .await
                .map_err(|_| NativeRunValidationError::InvalidMapping)?
            {
                return Ok(native_result_from_flow_run(&flow_run, metadata));
            }
        }

        let environment_variables = self
            .repository
            .list_application_environment_variables(actor.workspace_id, actor.application_id)
            .await
            .map_err(|_| NativeRunValidationError::InvalidMapping)?;
        let started_at = OffsetDateTime::now_utc();
        let flow_run = self
            .repository
            .create_published_flow_run(&CreateFlowRunInput {
                actor_user_id: actor.creator_user_id,
                application_id: actor.application_id,
                flow_id: publication.flow_id,
                flow_draft_id: compiled_plan.draft_id,
                compiled_plan_id: publication.compiled_plan_id,
                debug_session_id: String::new(),
                flow_schema_version: publication.flow_schema_version.clone(),
                document_hash: publication.document_hash.clone(),
                run_mode: domain::FlowRunMode::PublishedApiRun,
                target_node_id: None,
                title: build_flow_run_title(request.title.as_deref(), &request.query),
                status: domain::FlowRunStatus::Queued,
                input_payload: freeze_run_input_environment(
                    mapped.node_input_payload,
                    &environment_variables,
                    external_model_parameters,
                    compiled_plan_start_node_id(&compiled_plan.plan).as_deref(),
                ),
                started_at,
                api_key_id: Some(actor.api_key_id),
                publication_version_id: Some(publication.id),
                external_user: metadata
                    .get("external_user")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                external_conversation_id: metadata
                    .get("external_conversation_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                external_trace_id: metadata
                    .get("external_trace_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                compatibility_mode: metadata
                    .get("compatibility_mode")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                idempotency_key,
            })
            .await
            .map_err(|_| NativeRunValidationError::InvalidMapping)?;

        self.append_started_audit(&actor, &publication, &flow_run, &metadata)
            .await;

        Ok(native_result_from_flow_run(&flow_run, metadata))
    }

    pub async fn cancel_published_run(
        &self,
        actor: &ApplicationApiKeyActor,
        flow_run: &domain::FlowRunRecord,
    ) -> std::result::Result<domain::FlowRunRecord, NativeRunValidationError> {
        ensure_flow_run_transition(
            flow_run.status,
            domain::FlowRunStatus::Cancelled,
            "cancel_published_api_run",
        )
        .map_err(|_| NativeRunValidationError::InvalidState)?;
        let cancelled = self
            .repository
            .cancel_published_flow_run(&CancelPublishedFlowRunInput {
                flow_run_id: flow_run.id,
                from_status: flow_run.status,
                output_payload: flow_run.output_payload.clone(),
                error_payload: flow_run.error_payload.clone(),
                finished_at: OffsetDateTime::now_utc(),
            })
            .await
            .map_err(|_| NativeRunValidationError::InvalidState)?
            .unwrap_or_else(|| flow_run.clone());

        self.append_cancelled_audit(actor, &cancelled).await;

        Ok(cancelled)
    }

    async fn ensure_application_exists(
        &self,
        actor: &ApplicationApiKeyActor,
    ) -> std::result::Result<(), NativeRunValidationError> {
        self.repository
            .get_application(actor.workspace_id, actor.application_id)
            .await
            .map_err(|_| NativeRunValidationError::ApplicationNotPublished)?
            .ok_or(NativeRunValidationError::ApplicationNotPublished)?;
        Ok(())
    }

    async fn load_enabled_publication(
        &self,
        actor: &ApplicationApiKeyActor,
    ) -> std::result::Result<ApplicationPublicationVersionRecord, NativeRunValidationError> {
        let publication = self
            .repository
            .load_active_application_publication(actor.application_id)
            .await
            .map_err(|_| NativeRunValidationError::ApplicationNotPublished)?;
        let Some(publication) = publication.filter(|publication| publication.api_enabled) else {
            self.append_denied_audit(actor, "application_not_published")
                .await;
            return Err(NativeRunValidationError::ApplicationNotPublished);
        };
        Ok(publication)
    }

    async fn append_denied_audit(&self, actor: &ApplicationApiKeyActor, reason: &str) {
        let _ = ApplicationRepository::append_audit_log(
            &self.repository,
            &audit_log(
                Some(actor.workspace_id),
                Some(actor.creator_user_id),
                "application_public_api_run",
                None,
                "application_public_api.run_denied",
                json!({
                    "api_key_id": actor.api_key_id,
                    "application_id": actor.application_id,
                    "creator_user_id": actor.creator_user_id,
                    "reason": reason,
                }),
            ),
        )
        .await;
    }

    async fn append_started_audit(
        &self,
        actor: &ApplicationApiKeyActor,
        publication: &ApplicationPublicationVersionRecord,
        flow_run: &domain::FlowRunRecord,
        metadata: &Value,
    ) {
        let admission =
            crate::orchestration_runtime::scheduler_admission::derive_scheduler_admission_metadata(
                flow_run,
            );
        let payload = json!({
            "api_key_id": actor.api_key_id,
            "application_id": actor.application_id,
            "publication_version_id": publication.id,
            "creator_user_id": actor.creator_user_id,
            "external_user": flow_run.external_user,
            "external_conversation_id": flow_run.external_conversation_id,
            "external_trace_id": flow_run.external_trace_id,
            "response_mode": metadata
                .get("request")
                .and_then(|request| request.get("response_mode"))
                .cloned()
                .unwrap_or(Value::Null),
            "compatibility_mode": flow_run.compatibility_mode,
            "admission": admission,
        });

        let _ = self
            .repository
            .append_published_run_event(&crate::ports::AppendRunEventInput {
                flow_run_id: flow_run.id,
                node_run_id: None,
                event_type: "public_run_started".to_string(),
                payload: payload.clone(),
            })
            .await;
        let _ = ApplicationRepository::append_audit_log(
            &self.repository,
            &audit_log(
                Some(actor.workspace_id),
                Some(actor.creator_user_id),
                "application_public_api_run",
                Some(flow_run.id),
                "application_public_api.run_started",
                payload,
            ),
        )
        .await;
    }

    async fn append_cancelled_audit(
        &self,
        actor: &ApplicationApiKeyActor,
        flow_run: &domain::FlowRunRecord,
    ) {
        let payload = json!({
            "api_key_id": actor.api_key_id,
            "application_id": actor.application_id,
            "publication_version_id": flow_run.publication_version_id,
            "creator_user_id": actor.creator_user_id,
            "external_user": flow_run.external_user,
            "external_conversation_id": flow_run.external_conversation_id,
            "external_trace_id": flow_run.external_trace_id,
            "compatibility_mode": flow_run.compatibility_mode,
            "reason": "manual_stop",
        });
        let _ = self
            .repository
            .append_published_run_event(&crate::ports::AppendRunEventInput {
                flow_run_id: flow_run.id,
                node_run_id: None,
                event_type: "public_run_cancelled".to_string(),
                payload: payload.clone(),
            })
            .await;
        let _ = ApplicationRepository::append_audit_log(
            &self.repository,
            &audit_log(
                Some(actor.workspace_id),
                Some(actor.creator_user_id),
                "application_public_api_run",
                Some(flow_run.id),
                "application_public_api.run_cancelled",
                payload,
            ),
        )
        .await;
    }

    async fn bind_conversation(
        &self,
        application_id: Uuid,
        api_key_id: Uuid,
        mut request: NativeRunRequest,
    ) -> std::result::Result<NativeRunRequest, NativeRunValidationError> {
        let Some(external_user) = request
            .expand_id
            .clone()
            .or_else(|| request.conversation.string("user"))
        else {
            return Ok(request);
        };
        request
            .conversation
            .insert_string("user", external_user.clone());
        let external_conversation_id = request
            .conversation
            .string("id")
            .unwrap_or_else(generate_external_conversation_id);
        request
            .conversation
            .insert_string("id", external_conversation_id.clone());
        self.repository
            .bind_application_public_conversation(&BindApplicationPublicConversationInput {
                application_id,
                api_key_id,
                external_user: external_user.clone(),
                external_conversation_id: external_conversation_id.clone(),
            })
            .await
            .map_err(|_| NativeRunValidationError::InvalidMapping)?;
        if request.history.is_empty() {
            let messages = self
                .repository
                .list_application_public_conversation_messages(
                    &ListApplicationPublicConversationMessagesInput {
                        application_id,
                        api_key_id,
                        external_user,
                        external_conversation_id,
                        limit: APPLICATION_PUBLIC_CONVERSATION_HISTORY_LIMIT,
                    },
                )
                .await
                .map_err(|_| NativeRunValidationError::InvalidMapping)?;
            request.history = application_public_conversation_messages_to_native_history(messages);
        }
        Ok(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApplicationPublicConversationTurn {
    user_content: String,
    assistant_parts: Vec<String>,
}

fn application_public_conversation_messages_to_native_history(
    messages: Vec<ApplicationPublicConversationMessageRecord>,
) -> Vec<Value> {
    let mut turns = Vec::<ApplicationPublicConversationTurn>::new();

    for message in messages {
        match message.role.as_str() {
            "user" => {
                if let Some(user_content) = normalize_conversation_user_content(&message.content) {
                    turns.push(ApplicationPublicConversationTurn {
                        user_content,
                        assistant_parts: Vec::new(),
                    });
                }
            }
            "assistant" => {
                let Some(turn) = turns.last_mut() else {
                    continue;
                };
                if let Some(assistant_content) =
                    normalize_conversation_assistant_content(&message.content)
                {
                    turn.assistant_parts.push(assistant_content);
                }
            }
            _ => {}
        }
    }

    let mut deduped = Vec::<ApplicationPublicConversationTurn>::new();
    for turn in turns {
        if deduped
            .last()
            .is_some_and(|last| last.user_content == turn.user_content)
        {
            if let Some(last) = deduped.last_mut() {
                *last = turn;
            }
            continue;
        }
        deduped.push(turn);
    }

    let mut history = Vec::new();
    for turn in deduped {
        history.push(json!({
            "role": "user",
            "content": turn.user_content,
        }));
        let assistant_content = turn
            .assistant_parts
            .into_iter()
            .filter_map(|content| trimmed_history_text(&content))
            .collect::<Vec<_>>()
            .join("\n\n");
        if let Some(assistant_content) = trimmed_history_text(&assistant_content) {
            history.push(json!({
                "role": "assistant",
                "content": assistant_content,
            }));
        }
    }

    history
}

fn normalize_conversation_user_content(content: &str) -> Option<String> {
    trimmed_history_text(&strip_tag_blocks(content, "system-reminder"))
}

fn normalize_conversation_assistant_content(content: &str) -> Option<String> {
    let without_thinking = strip_tag_blocks(content, "think");
    let without_tool_calls = strip_tag_blocks(&without_thinking, "tool_call");
    let visible_content =
        content_after_beautified_marker(&without_tool_calls).unwrap_or(without_tool_calls.as_str());
    trimmed_history_text(visible_content)
}

fn strip_tag_blocks(content: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut output = content.to_string();

    while let Some(start) = output.find(&open) {
        let search_start = start + open.len();
        let Some(end) = output[search_start..].find(&close) else {
            break;
        };
        let end = search_start + end + close.len();
        output.replace_range(start..end, "");
    }

    output
}

fn content_after_beautified_marker(content: &str) -> Option<&str> {
    let marker = "下面是美化后内容";
    let marker_start = content.find(marker)?;
    Some(
        content[marker_start + marker.len()..].trim_start_matches(|value: char| {
            value.is_whitespace() || value == '-' || value == '—'
        }),
    )
}

fn trimmed_history_text(content: &str) -> Option<String> {
    let trimmed = content.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[async_trait]
pub trait ApplicationPublishedFlowRunRepository: Send + Sync {
    async fn create_published_flow_run(
        &self,
        input: &CreateFlowRunInput,
    ) -> Result<domain::FlowRunRecord>;

    async fn find_published_flow_run_by_idempotency_key(
        &self,
        application_id: Uuid,
        api_key_id: Uuid,
        idempotency_key: &str,
    ) -> Result<Option<domain::FlowRunRecord>>;

    async fn append_published_run_event(
        &self,
        input: &crate::ports::AppendRunEventInput,
    ) -> Result<domain::RunEventRecord>;
}

#[derive(Debug, Clone)]
pub struct CancelPublishedFlowRunInput {
    pub flow_run_id: Uuid,
    pub from_status: domain::FlowRunStatus,
    pub output_payload: Value,
    pub error_payload: Option<Value>,
    pub finished_at: OffsetDateTime,
}

#[async_trait]
pub trait ApplicationPublishedRunControlRepository: Send + Sync {
    async fn get_published_flow_run(
        &self,
        flow_run_id: Uuid,
    ) -> Result<Option<domain::FlowRunRecord>>;

    async fn cancel_published_flow_run(
        &self,
        input: &CancelPublishedFlowRunInput,
    ) -> Result<Option<domain::FlowRunRecord>>;

    async fn cancel_published_pending_callback_tasks_for_run(
        &self,
        flow_run_id: Uuid,
        completed_at: OffsetDateTime,
    ) -> Result<Vec<domain::CallbackTaskRecord>>;

    async fn get_published_callback_task(
        &self,
        callback_task_id: Uuid,
    ) -> Result<Option<domain::CallbackTaskRecord>>;

    async fn get_published_run_detail(
        &self,
        application_id: Uuid,
        flow_run_id: Uuid,
    ) -> Result<Option<domain::ApplicationRunDetail>>;
}

pub fn native_result_from_flow_run(
    flow_run: &domain::FlowRunRecord,
    metadata: Value,
) -> NativeRunResult {
    let error = flow_run.error_payload.as_ref().map(|payload| {
        let message = payload
            .get("message")
            .or_else(|| payload.get("error"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| payload.to_string());
        super::native::NativeError {
            code: payload
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("runtime_error")
                .to_string(),
            message,
            details: payload.clone(),
        }
    });
    NativeRunResult {
        id: flow_run.id,
        application_id: flow_run.application_id,
        api_key_id: flow_run.api_key_id.unwrap_or_default(),
        publication_version_id: flow_run.publication_version_id.unwrap_or_default(),
        status: native_status(flow_run.status),
        node_input_payload: flow_run.input_payload.clone(),
        metadata,
        answer: extract_answer(&flow_run.output_payload),
        required_action: None,
        tool_calls: extract_tool_calls(&flow_run.output_payload),
        usage: extract_usage(&flow_run.output_payload),
        error,
        created_at: flow_run.created_at,
    }
}

pub fn native_result_from_run_detail(
    detail: &domain::ApplicationRunDetail,
    metadata: Value,
) -> NativeRunResult {
    let mut result = native_result_from_flow_run(&detail.flow_run, metadata);
    if result.usage.is_none() {
        result.usage = aggregate_node_usage(&detail.node_runs);
    }
    if let Some(task) = latest_pending_callback_task(&detail.callback_tasks) {
        result.required_action = Some(native_required_action_from_callback_task(task));
        if task.callback_kind == "llm_tool_calls" {
            result.tool_calls = task
                .request_payload
                .get("tool_calls")
                .filter(|value| value.is_array())
                .cloned();
        }
    }
    result
}

fn latest_pending_callback_task(
    tasks: &[domain::CallbackTaskRecord],
) -> Option<&domain::CallbackTaskRecord> {
    tasks
        .iter()
        .rev()
        .find(|task| task.status == domain::CallbackTaskStatus::Pending)
}

fn native_required_action_from_callback_task(
    task: &domain::CallbackTaskRecord,
) -> super::native::NativeRequiredAction {
    let action_type = if task.callback_kind == "llm_tool_calls" {
        "submit_tool_outputs"
    } else {
        "callback"
    };
    super::native::NativeRequiredAction {
        action_type: action_type.to_string(),
        payload: json!({
            "callback_task_id": task.id,
            "callback_kind": task.callback_kind,
            "flow_run_id": task.flow_run_id,
            "node_run_id": task.node_run_id,
            "request_payload": task.request_payload,
            "tool_calls": task
                .request_payload
                .get("tool_calls")
                .cloned()
                .unwrap_or(Value::Null),
        }),
    }
}

fn extract_answer(output_payload: &Value) -> Option<String> {
    output_payload
        .get("answer")
        .or_else(|| output_payload.get("text"))
        .or_else(|| output_payload.get("output"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn extract_tool_calls(output_payload: &Value) -> Option<Value> {
    output_payload
        .get("tool_calls")
        .filter(|value| value.is_array())
        .cloned()
}

fn extract_usage(output_payload: &Value) -> Option<super::native::NativeUsage> {
    let usage = output_payload.get("usage")?;
    usage_from_payload(usage)
}

fn aggregate_node_usage(node_runs: &[domain::NodeRunRecord]) -> Option<super::native::NativeUsage> {
    let mut aggregate = super::native::NativeUsage::default();
    let mut saw_usage = false;

    for node_run in node_runs {
        let usage = node_run
            .metrics_payload
            .get("usage")
            .and_then(usage_from_payload)
            .or_else(|| {
                node_run
                    .output_payload
                    .get("usage")
                    .and_then(usage_from_payload)
            });
        let Some(usage) = usage else {
            continue;
        };
        saw_usage = true;
        merge_usage(&mut aggregate, usage_with_total(usage));
    }

    saw_usage.then_some(aggregate)
}

fn usage_from_payload(usage: &Value) -> Option<super::native::NativeUsage> {
    let native_usage = super::native::NativeUsage {
        prompt_tokens: usage_number(usage, &["prompt_tokens", "input_tokens"]),
        completion_tokens: usage_number(usage, &["completion_tokens", "output_tokens"]),
        total_tokens: usage_number(usage, &["total_tokens"]),
        reasoning_tokens: usage_number(usage, &["reasoning_tokens"]),
        input_cache_hit_tokens: usage_number(usage, &["input_cache_hit_tokens"]),
        input_cache_miss_tokens: usage_number(usage, &["input_cache_miss_tokens"]),
        cache_read_tokens: usage_number(usage, &["cache_read_tokens", "cache_read_input_tokens"]),
        cache_write_tokens: usage_number(
            usage,
            &["cache_write_tokens", "cache_creation_input_tokens"],
        ),
    };

    native_usage_has_any_tokens(&native_usage).then_some(native_usage)
}

fn usage_number(usage: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| usage.get(*key).and_then(Value::as_u64))
}

fn usage_with_total(mut usage: super::native::NativeUsage) -> super::native::NativeUsage {
    if usage.total_tokens.is_none() {
        usage.total_tokens = match (usage.prompt_tokens, usage.completion_tokens) {
            (Some(prompt_tokens), Some(completion_tokens)) => {
                Some(prompt_tokens + completion_tokens)
            }
            _ => None,
        };
    }
    usage
}

fn native_usage_has_any_tokens(usage: &super::native::NativeUsage) -> bool {
    usage.prompt_tokens.is_some()
        || usage.completion_tokens.is_some()
        || usage.total_tokens.is_some()
        || usage.reasoning_tokens.is_some()
        || usage.input_cache_hit_tokens.is_some()
        || usage.input_cache_miss_tokens.is_some()
        || usage.cache_read_tokens.is_some()
        || usage.cache_write_tokens.is_some()
}

fn merge_usage(target: &mut super::native::NativeUsage, delta: super::native::NativeUsage) {
    add_usage_tokens(&mut target.prompt_tokens, delta.prompt_tokens);
    add_usage_tokens(&mut target.completion_tokens, delta.completion_tokens);
    add_usage_tokens(&mut target.total_tokens, delta.total_tokens);
    add_usage_tokens(&mut target.reasoning_tokens, delta.reasoning_tokens);
    add_usage_tokens(
        &mut target.input_cache_hit_tokens,
        delta.input_cache_hit_tokens,
    );
    add_usage_tokens(
        &mut target.input_cache_miss_tokens,
        delta.input_cache_miss_tokens,
    );
    add_usage_tokens(&mut target.cache_read_tokens, delta.cache_read_tokens);
    add_usage_tokens(&mut target.cache_write_tokens, delta.cache_write_tokens);
}

fn add_usage_tokens(target: &mut Option<u64>, delta: Option<u64>) {
    if let Some(delta) = delta {
        *target = Some(target.unwrap_or_default() + delta);
    }
}

fn generate_external_conversation_id() -> String {
    format!("conv_{}", Uuid::now_v7().simple())
}

fn freeze_run_input_environment(
    input_payload: Value,
    variables: &[domain::ApplicationEnvironmentVariable],
    external_model_parameters: Option<Value>,
    start_node_id: Option<&str>,
) -> Value {
    let mut payload = input_payload.as_object().cloned().unwrap_or_default();
    payload.insert(
        "env".to_string(),
        Value::Object(application_environment_variable_payload(variables)),
    );
    if let Some(model_parameters) = external_model_parameters {
        let mut sys = payload
            .remove("sys")
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        let reasoning_effort = external_reasoning_effort(&model_parameters).unwrap_or_default();
        sys.insert("model_parameters".to_string(), model_parameters);
        insert_start_reasoning_effort(&mut payload, start_node_id, reasoning_effort);
        payload.insert("sys".to_string(), Value::Object(sys));
    }
    Value::Object(payload)
}

fn compiled_plan_start_node_id(plan: &Value) -> Option<String> {
    plan.get("nodes")
        .and_then(Value::as_object)?
        .iter()
        .find_map(|(node_id, node)| {
            (node.get("node_type").and_then(Value::as_str) == Some("start"))
                .then(|| node_id.clone())
        })
}

fn insert_start_reasoning_effort(
    payload: &mut Map<String, Value>,
    start_node_id: Option<&str>,
    reasoning_effort: String,
) {
    let start_node_id = start_node_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("node-start");
    let start_payload = payload
        .entry(start_node_id.to_string())
        .or_insert_with(|| Value::Object(Map::new()));

    if !start_payload.is_object() {
        *start_payload = Value::Object(Map::new());
    }
    if let Some(start_payload) = start_payload.as_object_mut() {
        start_payload.insert(
            "reasoning_effort".to_string(),
            Value::String(reasoning_effort),
        );
    }
}

fn external_reasoning_effort(model_parameters: &Value) -> Option<String> {
    model_parameters
        .get("reasoning")
        .and_then(|value| value.get("effort"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn validate_external_model_parameters(
    request: &NativeRunRequest,
    document_snapshot: &Value,
) -> std::result::Result<Option<Value>, NativeRunValidationError> {
    let Some(model_parameters) = request.execution.get("model_parameters") else {
        return Ok(None);
    };
    let model_parameters =
        model_parameters
            .as_object()
            .ok_or(NativeRunValidationError::InvalidModelParameters(
                "execution.model_parameters",
            ))?;
    for key in model_parameters.keys() {
        if key != "reasoning" {
            return Err(NativeRunValidationError::InvalidModelParameters(
                "execution.model_parameters",
            ));
        }
    }
    let Some(reasoning) = model_parameters.get("reasoning") else {
        return Ok(Some(json!({})));
    };
    let reasoning =
        reasoning
            .as_object()
            .ok_or(NativeRunValidationError::InvalidModelParameters(
                "execution.model_parameters.reasoning",
            ))?;
    for key in reasoning.keys() {
        if !matches!(key.as_str(), "enabled" | "effort" | "budget_tokens") {
            return Err(NativeRunValidationError::InvalidModelParameters(
                "execution.model_parameters.reasoning",
            ));
        }
    }

    let enabled = reasoning
        .get("enabled")
        .map(|value| {
            value
                .as_bool()
                .ok_or(NativeRunValidationError::InvalidModelParameters(
                    "execution.model_parameters.reasoning.enabled",
                ))
        })
        .transpose()?
        .unwrap_or(true);
    let effort = reasoning
        .get("effort")
        .map(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .ok_or(NativeRunValidationError::InvalidModelParameters(
                    "execution.model_parameters.reasoning.effort",
                ))
        })
        .transpose()?;
    if let Some(effort) = effort.as_deref() {
        if !is_known_reasoning_effort(effort) {
            return Err(NativeRunValidationError::InvalidModelParameters(
                "execution.model_parameters.reasoning.effort",
            ));
        }
    }
    let budget_tokens = reasoning
        .get("budget_tokens")
        .map(|value| {
            value.as_u64().filter(|value| *value > 0).ok_or(
                NativeRunValidationError::InvalidModelParameters(
                    "execution.model_parameters.reasoning.budget_tokens",
                ),
            )
        })
        .transpose()?;

    if enabled || effort.is_some() || budget_tokens.is_some() {
        let model_id = request
            .model
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(NativeRunValidationError::InvalidModelParameters("model"))?;
        let models = extract_agent_model_catalog_from_start_node(document_snapshot);
        let model = find_agent_model(&models, model_id)
            .ok_or(NativeRunValidationError::InvalidModelParameters("model"))?;
        let supports_reasoning = model.capabilities.reasoning
            || model.reasoning.as_ref().is_some_and(|reasoning| {
                reasoning.default_effort.is_some() || !reasoning.supported_efforts.is_empty()
            });
        if !supports_reasoning {
            return Err(NativeRunValidationError::InvalidModelParameters(
                "execution.model_parameters.reasoning",
            ));
        }
        if let Some(effort) = effort.as_deref() {
            if let Some(reasoning) = model.reasoning.as_ref() {
                if !reasoning.supported_efforts.is_empty()
                    && !reasoning
                        .supported_efforts
                        .iter()
                        .any(|supported| supported == effort)
                {
                    return Err(NativeRunValidationError::InvalidModelParameters(
                        "execution.model_parameters.reasoning.effort",
                    ));
                }
            }
        }
        if let (Some(budget_tokens), Some(max_output_tokens)) =
            (budget_tokens, model.max_output_tokens)
        {
            if budget_tokens > max_output_tokens {
                return Err(NativeRunValidationError::InvalidModelParameters(
                    "execution.model_parameters.reasoning.budget_tokens",
                ));
            }
        }
    }

    let mut clean_reasoning = Map::new();
    clean_reasoning.insert("enabled".to_string(), Value::Bool(enabled));
    if let Some(effort) = effort {
        clean_reasoning.insert("effort".to_string(), Value::String(effort));
    }
    if let Some(budget_tokens) = budget_tokens {
        clean_reasoning.insert("budget_tokens".to_string(), json!(budget_tokens));
    }

    Ok(Some(json!({
        "reasoning": Value::Object(clean_reasoning)
    })))
}

fn is_known_reasoning_effort(effort: &str) -> bool {
    matches!(effort, "minimal" | "low" | "medium" | "high" | "xhigh")
}

fn application_environment_variable_payload(
    variables: &[domain::ApplicationEnvironmentVariable],
) -> Map<String, Value> {
    variables
        .iter()
        .map(|variable| (variable.name.clone(), variable.value.clone()))
        .collect()
}

fn native_status(status: domain::FlowRunStatus) -> NativeRunStatus {
    match status {
        domain::FlowRunStatus::Queued => NativeRunStatus::Queued,
        domain::FlowRunStatus::Running => NativeRunStatus::Running,
        domain::FlowRunStatus::WaitingCallback | domain::FlowRunStatus::WaitingHuman => {
            NativeRunStatus::Waiting
        }
        domain::FlowRunStatus::Paused => NativeRunStatus::Running,
        domain::FlowRunStatus::Succeeded => NativeRunStatus::Succeeded,
        domain::FlowRunStatus::Failed => NativeRunStatus::Failed,
        domain::FlowRunStatus::Cancelled => NativeRunStatus::Cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use time::OffsetDateTime;
    use uuid::Uuid;

    #[test]
    fn native_result_from_run_detail_aggregates_node_usage_when_flow_output_has_none() {
        let detail = domain::ApplicationRunDetail {
            flow_run: test_flow_run(json!({ "answer": "ok" })),
            node_runs: vec![
                test_node_run(
                    "node-llm",
                    json!({
                        "input_tokens": 47,
                        "output_tokens": 59,
                        "total_tokens": 106,
                        "input_cache_hit_tokens": 4,
                        "cache_write_tokens": 2
                    }),
                ),
                test_node_run(
                    "node-llm-1",
                    json!({
                        "input_tokens": 196,
                        "output_tokens": 1120,
                        "total_tokens": 1316,
                        "reasoning_tokens": 1063,
                        "cache_read_tokens": 8
                    }),
                ),
            ],
            checkpoints: Vec::new(),
            callback_tasks: Vec::new(),
            events: Vec::new(),
        };

        let run = native_result_from_run_detail(&detail, json!({}));
        let usage = run.usage.expect("node usage should be projected");

        assert_eq!(usage.prompt_tokens, Some(243));
        assert_eq!(usage.completion_tokens, Some(1179));
        assert_eq!(usage.total_tokens, Some(1422));
        assert_eq!(usage.reasoning_tokens, Some(1063));
        assert_eq!(usage.input_cache_hit_tokens, Some(4));
        assert_eq!(usage.cache_read_tokens, Some(8));
        assert_eq!(usage.cache_write_tokens, Some(2));
    }

    #[test]
    fn native_result_from_run_detail_prefers_flow_output_usage_selector() {
        let detail = domain::ApplicationRunDetail {
            flow_run: test_flow_run(json!({
                "answer": "ok",
                "usage": {
                    "prompt_tokens": 11,
                    "completion_tokens": 7,
                    "total_tokens": 18
                }
            })),
            node_runs: vec![test_node_run(
                "node-llm",
                json!({
                    "input_tokens": 100,
                    "output_tokens": 200,
                    "total_tokens": 300
                }),
            )],
            checkpoints: Vec::new(),
            callback_tasks: Vec::new(),
            events: Vec::new(),
        };

        let run = native_result_from_run_detail(&detail, json!({}));
        let usage = run.usage.expect("flow usage should be projected");

        assert_eq!(usage.prompt_tokens, Some(11));
        assert_eq!(usage.completion_tokens, Some(7));
        assert_eq!(usage.total_tokens, Some(18));
    }

    #[test]
    fn conversation_history_rehydration_filters_internal_claude_code_payloads() {
        let messages = vec![
            conversation_message(
                "user",
                "<system-reminder>internal skills</system-reminder>\n\nhi ?",
                1,
            ),
            conversation_message(
                "assistant",
                "<think>private reasoning</think>嗨，有什么需要我帮忙的？",
                2,
            ),
            conversation_message("user", "Describe image", 3),
            conversation_message(
                "assistant",
                "<think>need file</think><tool_call>read image</tool_call>Reading image",
                4,
            ),
            conversation_message("user", "Describe image", 5),
            conversation_message(
                "assistant",
                "<think>done</think>The diagram is an Agent scheduler.",
                6,
            ),
            conversation_message("user", "Find related code", 7),
            conversation_message(
                "assistant",
                "<think>search</think>Raw preface<tool_call>grep</tool_call>\n\n---\n\n下面是美化后内容\n\nFinal visible answer",
                8,
            ),
        ];

        let history = application_public_conversation_messages_to_native_history(messages);

        assert_eq!(
            history,
            vec![
                json!({"role": "user", "content": "hi ?"}),
                json!({"role": "assistant", "content": "嗨，有什么需要我帮忙的？"}),
                json!({"role": "user", "content": "Describe image"}),
                json!({"role": "assistant", "content": "The diagram is an Agent scheduler."}),
                json!({"role": "user", "content": "Find related code"}),
                json!({"role": "assistant", "content": "Final visible answer"}),
            ]
        );
    }

    fn test_flow_run(output_payload: Value) -> domain::FlowRunRecord {
        let created_at = OffsetDateTime::UNIX_EPOCH;
        domain::FlowRunRecord {
            id: Uuid::from_u128(0x11111111111111111111111111111111),
            application_id: Uuid::from_u128(0x22222222222222222222222222222222),
            flow_id: Uuid::from_u128(0x33333333333333333333333333333333),
            draft_id: Uuid::from_u128(0x44444444444444444444444444444444),
            compiled_plan_id: Some(Uuid::from_u128(0x55555555555555555555555555555555)),
            debug_session_id: String::new(),
            flow_schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "hash".to_string(),
            run_mode: domain::FlowRunMode::PublishedApiRun,
            target_node_id: None,
            title: "test".to_string(),
            status: domain::FlowRunStatus::Succeeded,
            input_payload: json!({}),
            output_payload,
            error_payload: None,
            created_by: Uuid::from_u128(0x66666666666666666666666666666666),
            authorized_account: None,
            api_key_id: Some(Uuid::from_u128(0x77777777777777777777777777777777)),
            publication_version_id: Some(Uuid::from_u128(0x88888888888888888888888888888888)),
            external_user: None,
            external_conversation_id: None,
            external_trace_id: None,
            compatibility_mode: Some("anthropic-messages-v1".to_string()),
            idempotency_key: None,
            started_at: created_at,
            finished_at: Some(created_at),
            created_at,
            updated_at: created_at,
        }
    }

    fn test_node_run(node_id: &str, usage: Value) -> domain::NodeRunRecord {
        let created_at = OffsetDateTime::UNIX_EPOCH;
        domain::NodeRunRecord {
            id: Uuid::now_v7(),
            flow_run_id: Uuid::from_u128(0x11111111111111111111111111111111),
            node_id: node_id.to_string(),
            node_type: "llm".to_string(),
            node_alias: node_id.to_string(),
            status: domain::NodeRunStatus::Succeeded,
            input_payload: json!({}),
            output_payload: json!({ "usage": usage.clone() }),
            error_payload: None,
            metrics_payload: json!({ "usage": usage }),
            debug_payload: json!({}),
            started_at: created_at,
            finished_at: Some(created_at),
        }
    }

    fn conversation_message(
        role: &str,
        content: &str,
        sequence: i64,
    ) -> ApplicationPublicConversationMessageRecord {
        ApplicationPublicConversationMessageRecord {
            role: role.to_string(),
            content: content.to_string(),
            sequence,
        }
    }
}
