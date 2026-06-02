use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use control_plane::{
    orchestration_runtime::{CompleteCallbackTaskCommand, OrchestrationRuntimeService},
    ports::{
        AppendRunEventInput, ClaimFlowRunResumeRequestInput, FinishFlowRunResumeRequestInput,
        OrchestrationRuntimeRepository, UpdateFlowRunInput,
    },
};
use serde_json::json;
use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::{
    sync::watch,
    time::{sleep, timeout, Duration as TokioDuration, Instant},
};
use tracing::{error, warn};
use uuid::Uuid;

use crate::{
    app_state::ApiState,
    provider_runtime::ApiProviderRuntime,
    runtime_activity::{scope_application_activity, ApplicationActivityKind},
};

const CLAIM_TTL_SECONDS: i64 = 60;
const IDLE_POLL_INTERVAL: TokioDuration = TokioDuration::from_millis(500);
const ERROR_POLL_INTERVAL: TokioDuration = TokioDuration::from_secs(2);
const REQUEST_TIMEOUT: TokioDuration = TokioDuration::from_secs(120);

pub fn spawn_native_resume_worker(state: Arc<ApiState>) {
    let worker_id = format!("api-server-native-resume-{}", Uuid::now_v7());
    let (shutdown_sender, mut shutdown_receiver) = watch::channel(false);
    state
        .native_resume_worker
        .mark_started(worker_id.clone(), shutdown_sender);
    tokio::spawn(async move {
        loop {
            let result = tokio::select! {
                changed = shutdown_receiver.changed() => {
                    if changed.is_err() || *shutdown_receiver.borrow() {
                        state.native_resume_worker.mark_stopped();
                        break;
                    }
                    continue;
                }
                result = process_next_native_resume_request(state.clone(), &worker_id) => result,
            };

            match result {
                Ok(true) => continue,
                Ok(false) => {
                    if sleep_or_shutdown(IDLE_POLL_INTERVAL, &mut shutdown_receiver).await {
                        state.native_resume_worker.mark_stopped();
                        break;
                    }
                }
                Err(error) => {
                    state.native_resume_worker.mark_error(error.to_string());
                    warn!(error = %error, "native resume worker poll failed");
                    if sleep_or_shutdown(ERROR_POLL_INTERVAL, &mut shutdown_receiver).await {
                        state.native_resume_worker.mark_stopped();
                        break;
                    }
                }
            }
        }
    });
}

async fn sleep_or_shutdown(
    duration: TokioDuration,
    shutdown_receiver: &mut watch::Receiver<bool>,
) -> bool {
    tokio::select! {
        _ = sleep(duration) => false,
        changed = shutdown_receiver.changed() => changed.is_err() || *shutdown_receiver.borrow(),
    }
}

pub async fn process_next_native_resume_request(
    state: Arc<ApiState>,
    worker_id: &str,
) -> Result<bool> {
    state.native_resume_worker.mark_poll();
    let claim_result =
        <storage_durable::MainDurableStore as OrchestrationRuntimeRepository>::claim_next_flow_run_resume_request(
            &state.store,
            &ClaimFlowRunResumeRequestInput {
                worker_id: worker_id.to_string(),
                claim_expires_at: OffsetDateTime::now_utc()
                    + TimeDuration::seconds(CLAIM_TTL_SECONDS),
            },
        )
        .await;
    let claim = match claim_result {
        Ok(claim) => claim,
        Err(error) => {
            state.native_resume_worker.mark_error(error.to_string());
            return Err(error);
        }
    };

    let Some(claim) = claim else {
        state.native_resume_worker.mark_idle();
        return Ok(false);
    };

    state
        .native_resume_worker
        .mark_processing(claim.request.id, claim.request.flow_run_id);
    let _ =
        <storage_durable::MainDurableStore as OrchestrationRuntimeRepository>::append_run_event(
            &state.store,
            &AppendRunEventInput {
                flow_run_id: claim.request.flow_run_id,
                node_run_id: None,
                event_type: "public_run_resume_claimed".into(),
                payload: json!({
                    "resume_request_id": claim.request.id,
                    "callback_task_id": claim.request.callback_task_id,
                    "worker_id": worker_id,
                }),
            },
        )
        .await;
    let processing_started = Instant::now();
    let _execution_activity = state.runtime_activity.start(
        claim.application_id,
        ApplicationActivityKind::ApplicationExecution,
    );
    let runtime_service = OrchestrationRuntimeService::new(
        state.store.clone(),
        ApiProviderRuntime::new_with_activity(
            state.provider_runtime.clone(),
            state.runtime_activity.clone(),
        ),
        state.runtime_engine.clone(),
        state.provider_secret_master_key.clone(),
    )
    .with_runtime_event_stream(state.runtime_event_stream.clone());

    let result = match timeout(
        REQUEST_TIMEOUT,
        scope_application_activity(
            claim.application_id,
            runtime_service.complete_callback_task(CompleteCallbackTaskCommand {
                actor_user_id: claim.actor_user_id,
                application_id: claim.application_id,
                callback_task_id: claim.request.callback_task_id,
                response_payload: claim.request.response_payload.clone(),
            }),
        ),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(anyhow!("native resume request timed out")),
    };

    match result {
        Ok(_) => {
            let finished = <storage_durable::MainDurableStore as OrchestrationRuntimeRepository>::finish_flow_run_resume_request(
                &state.store,
                &FinishFlowRunResumeRequestInput {
                    request_id: claim.request.id,
                    status: domain::FlowRunResumeRequestStatus::Succeeded,
                    error_payload: None,
                    completed_at: OffsetDateTime::now_utc(),
                },
            )
            .await?;
            if finished.status == domain::FlowRunResumeRequestStatus::Cancelled {
                state.native_resume_worker.mark_idle();
                return Ok(true);
            }
            let _ =
                <storage_durable::MainDurableStore as OrchestrationRuntimeRepository>::append_run_event(
                    &state.store,
                    &AppendRunEventInput {
                        flow_run_id: claim.request.flow_run_id,
                        node_run_id: None,
                        event_type: "public_run_resume_succeeded".into(),
                        payload: json!({
                            "resume_request_id": claim.request.id,
                            "callback_task_id": claim.request.callback_task_id,
                        }),
                    },
                )
                .await;
            state
                .native_resume_worker
                .mark_succeeded(processing_started.elapsed().as_millis() as u64);
        }
        Err(error) => {
            let error_payload = json!({ "message": error.to_string() });
            let _ =
                <storage_durable::MainDurableStore as OrchestrationRuntimeRepository>::update_flow_run_if_status(
                    &state.store,
                    &UpdateFlowRunInput {
                        flow_run_id: claim.request.flow_run_id,
                        status: domain::FlowRunStatus::Failed,
                        output_payload: json!({}),
                        error_payload: Some(error_payload.clone()),
                        finished_at: Some(OffsetDateTime::now_utc()),
                    },
                    domain::FlowRunStatus::WaitingCallback,
                )
                .await;
            let finished = <storage_durable::MainDurableStore as OrchestrationRuntimeRepository>::finish_flow_run_resume_request(
                &state.store,
                &FinishFlowRunResumeRequestInput {
                    request_id: claim.request.id,
                    status: domain::FlowRunResumeRequestStatus::Failed,
                    error_payload: Some(error_payload.clone()),
                    completed_at: OffsetDateTime::now_utc(),
                },
            )
            .await?;
            if finished.status == domain::FlowRunResumeRequestStatus::Cancelled {
                state.native_resume_worker.mark_idle();
                return Ok(true);
            }
            let _ =
                <storage_durable::MainDurableStore as OrchestrationRuntimeRepository>::append_run_event(
                    &state.store,
                    &AppendRunEventInput {
                        flow_run_id: claim.request.flow_run_id,
                        node_run_id: None,
                        event_type: "public_run_resume_failed".into(),
                        payload: json!({
                            "resume_request_id": claim.request.id,
                            "callback_task_id": claim.request.callback_task_id,
                            "error": error_payload,
                        }),
                    },
                )
                .await;
            state.native_resume_worker.mark_failed(
                error.to_string(),
                processing_started.elapsed().as_millis() as u64,
            );
            error!(
                flow_run_id = %claim.request.flow_run_id,
                callback_task_id = %claim.request.callback_task_id,
                "native resume request failed during continuation"
            );
        }
    }

    Ok(true)
}

#[derive(Default)]
pub struct NativeResumeWorkerRuntime {
    inner: Mutex<NativeResumeWorkerRuntimeState>,
}

#[derive(Default)]
struct NativeResumeWorkerRuntimeState {
    worker_id: Option<String>,
    status: String,
    started_at: Option<OffsetDateTime>,
    last_heartbeat_at: Option<OffsetDateTime>,
    last_poll_at: Option<OffsetDateTime>,
    last_claimed_at: Option<OffsetDateTime>,
    last_success_at: Option<OffsetDateTime>,
    last_error_at: Option<OffsetDateTime>,
    last_error: Option<String>,
    current_request_id: Option<Uuid>,
    current_flow_run_id: Option<Uuid>,
    processed_count: u64,
    succeeded_count: u64,
    failed_count: u64,
    last_duration_ms: Option<u64>,
    shutdown_sender: Option<watch::Sender<bool>>,
}

#[derive(Debug, Clone)]
pub struct NativeResumeWorkerRuntimeSnapshot {
    pub worker_id: Option<String>,
    pub status: String,
    pub started_at: Option<OffsetDateTime>,
    pub last_heartbeat_at: Option<OffsetDateTime>,
    pub last_poll_at: Option<OffsetDateTime>,
    pub last_claimed_at: Option<OffsetDateTime>,
    pub last_success_at: Option<OffsetDateTime>,
    pub last_error_at: Option<OffsetDateTime>,
    pub last_error: Option<String>,
    pub current_request_id: Option<Uuid>,
    pub current_flow_run_id: Option<Uuid>,
    pub processed_count: u64,
    pub succeeded_count: u64,
    pub failed_count: u64,
    pub last_duration_ms: Option<u64>,
}

impl NativeResumeWorkerRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_started(&self, worker_id: String, shutdown_sender: watch::Sender<bool>) {
        let now = OffsetDateTime::now_utc();
        let mut inner = self.inner.lock().expect("native resume worker mutex");
        inner.worker_id = Some(worker_id);
        inner.status = "idle".into();
        inner.started_at = Some(now);
        inner.last_heartbeat_at = Some(now);
        inner.shutdown_sender = Some(shutdown_sender);
    }

    pub fn mark_poll(&self) {
        let now = OffsetDateTime::now_utc();
        let mut inner = self.inner.lock().expect("native resume worker mutex");
        inner.status = "polling".into();
        inner.last_heartbeat_at = Some(now);
        inner.last_poll_at = Some(now);
    }

    pub fn mark_idle(&self) {
        let mut inner = self.inner.lock().expect("native resume worker mutex");
        inner.status = "idle".into();
        inner.current_request_id = None;
        inner.current_flow_run_id = None;
        inner.last_heartbeat_at = Some(OffsetDateTime::now_utc());
    }

    pub fn mark_processing(&self, request_id: Uuid, flow_run_id: Uuid) {
        let now = OffsetDateTime::now_utc();
        let mut inner = self.inner.lock().expect("native resume worker mutex");
        inner.status = "processing".into();
        inner.last_heartbeat_at = Some(now);
        inner.last_claimed_at = Some(now);
        inner.current_request_id = Some(request_id);
        inner.current_flow_run_id = Some(flow_run_id);
    }

    pub fn mark_succeeded(&self, duration_ms: u64) {
        let now = OffsetDateTime::now_utc();
        let mut inner = self.inner.lock().expect("native resume worker mutex");
        inner.status = "idle".into();
        inner.last_heartbeat_at = Some(now);
        inner.last_success_at = Some(now);
        inner.current_request_id = None;
        inner.current_flow_run_id = None;
        inner.processed_count += 1;
        inner.succeeded_count += 1;
        inner.last_duration_ms = Some(duration_ms);
        inner.last_error = None;
    }

    pub fn mark_failed(&self, error: String, duration_ms: u64) {
        let now = OffsetDateTime::now_utc();
        let mut inner = self.inner.lock().expect("native resume worker mutex");
        inner.status = "error".into();
        inner.last_heartbeat_at = Some(now);
        inner.last_error_at = Some(now);
        inner.last_error = Some(error);
        inner.current_request_id = None;
        inner.current_flow_run_id = None;
        inner.processed_count += 1;
        inner.failed_count += 1;
        inner.last_duration_ms = Some(duration_ms);
    }

    pub fn mark_error(&self, error: String) {
        let now = OffsetDateTime::now_utc();
        let mut inner = self.inner.lock().expect("native resume worker mutex");
        inner.status = "error".into();
        inner.last_heartbeat_at = Some(now);
        inner.last_error_at = Some(now);
        inner.last_error = Some(error);
    }

    pub fn mark_stopped(&self) {
        let mut inner = self.inner.lock().expect("native resume worker mutex");
        inner.status = "stopped".into();
        inner.last_heartbeat_at = Some(OffsetDateTime::now_utc());
        inner.shutdown_sender = None;
    }

    pub fn shutdown(&self) -> bool {
        let inner = self.inner.lock().expect("native resume worker mutex");
        inner
            .shutdown_sender
            .as_ref()
            .is_some_and(|sender| sender.send(true).is_ok())
    }

    pub fn snapshot(&self) -> NativeResumeWorkerRuntimeSnapshot {
        let inner = self.inner.lock().expect("native resume worker mutex");
        NativeResumeWorkerRuntimeSnapshot {
            worker_id: inner.worker_id.clone(),
            status: if inner.status.is_empty() {
                "not_started".into()
            } else {
                inner.status.clone()
            },
            started_at: inner.started_at,
            last_heartbeat_at: inner.last_heartbeat_at,
            last_poll_at: inner.last_poll_at,
            last_claimed_at: inner.last_claimed_at,
            last_success_at: inner.last_success_at,
            last_error_at: inner.last_error_at,
            last_error: inner.last_error.clone(),
            current_request_id: inner.current_request_id,
            current_flow_run_id: inner.current_flow_run_id,
            processed_count: inner.processed_count,
            succeeded_count: inner.succeeded_count,
            failed_count: inner.failed_count,
            last_duration_ms: inner.last_duration_ms,
        }
    }
}
