use anyhow::{anyhow, Result};
use async_trait::async_trait;
use control_plane::{
    errors::ControlPlaneError,
    ports::{
        AppendBillingSessionInput, AppendCapabilityInvocationInput, AppendContextProjectionInput,
        AppendCostLedgerInput, AppendCreditLedgerInput, AppendModelFailoverAttemptLedgerInput,
        AppendRunEventInput, AppendRuntimeEventInput, AppendRuntimeItemInput,
        AppendRuntimeSpanInput, AppendUsageLedgerInput, AttachCompiledPlanToFlowRunInput,
        CompleteCallbackTaskInput, CompleteFlowRunInput, CompleteNodeRunInput,
        CreateCallbackTaskInput, CreateCheckpointInput, CreateFlowRunInput,
        CreateFlowRunShellInput, CreateNodeRunInput, CreateRuntimeDebugArtifactInput,
        DataModelSideEffectReceiptClaim, FailQueuedFlowRunShellInput, GetRuntimeDebugArtifactInput,
        LinkUsageLedgerToModelFailoverAttemptInput, OrchestrationRuntimeRepository,
        UpdateFlowRunInput, UpdateFlowRunPayloadsInput, UpdateNodeRunInput,
        UpdateNodeRunPayloadsInput, UpdateRunEventPayloadInput, UpsertCompiledPlanInput,
        UpsertDataModelSideEffectReceiptInput,
    },
};
use sqlx::{Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::repositories::PgControlPlaneStore;

mod detail_queries;
mod record_mappers;
mod sequencing;

use detail_queries::*;
use record_mappers::*;
use sequencing::*;

include!("event_methods.rs");
include!("artifact_methods.rs");
include!("flow_run_methods.rs");
include!("ledger_methods.rs");
include!("read_methods.rs");
include!("side_effect_receipt_methods.rs");

#[async_trait]
impl OrchestrationRuntimeRepository for PgControlPlaneStore {
    async fn upsert_compiled_plan(
        &self,
        input: &UpsertCompiledPlanInput,
    ) -> Result<domain::CompiledPlanRecord> {
        PgControlPlaneStore::upsert_compiled_plan(self, input).await
    }

    async fn get_compiled_plan(
        &self,
        compiled_plan_id: Uuid,
    ) -> Result<Option<domain::CompiledPlanRecord>> {
        PgControlPlaneStore::get_compiled_plan(self, compiled_plan_id).await
    }

    async fn create_flow_run(&self, input: &CreateFlowRunInput) -> Result<domain::FlowRunRecord> {
        PgControlPlaneStore::create_flow_run(self, input).await
    }

    async fn create_flow_run_shell(
        &self,
        input: &CreateFlowRunShellInput,
    ) -> Result<domain::FlowRunRecord> {
        PgControlPlaneStore::create_flow_run_shell(self, input).await
    }

    async fn attach_compiled_plan_to_flow_run(
        &self,
        input: &AttachCompiledPlanToFlowRunInput,
    ) -> Result<domain::FlowRunRecord> {
        PgControlPlaneStore::attach_compiled_plan_to_flow_run(self, input).await
    }

    async fn fail_queued_flow_run_shell(
        &self,
        input: &FailQueuedFlowRunShellInput,
    ) -> Result<Option<domain::FlowRunRecord>> {
        PgControlPlaneStore::fail_queued_flow_run_shell(self, input).await
    }

    async fn get_flow_run(
        &self,
        application_id: Uuid,
        flow_run_id: Uuid,
    ) -> Result<Option<domain::FlowRunRecord>> {
        PgControlPlaneStore::get_flow_run(self, application_id, flow_run_id).await
    }

    async fn create_node_run(&self, input: &CreateNodeRunInput) -> Result<domain::NodeRunRecord> {
        PgControlPlaneStore::create_node_run(self, input).await
    }

    async fn update_node_run(&self, input: &UpdateNodeRunInput) -> Result<domain::NodeRunRecord> {
        PgControlPlaneStore::update_node_run(self, input).await
    }

    async fn complete_node_run(
        &self,
        input: &CompleteNodeRunInput,
    ) -> Result<domain::NodeRunRecord> {
        PgControlPlaneStore::complete_node_run(self, input).await
    }

    async fn update_flow_run(&self, input: &UpdateFlowRunInput) -> Result<domain::FlowRunRecord> {
        PgControlPlaneStore::update_flow_run(self, input).await
    }

    async fn update_flow_run_if_status(
        &self,
        input: &UpdateFlowRunInput,
        expected_status: domain::FlowRunStatus,
    ) -> Result<Option<domain::FlowRunRecord>> {
        PgControlPlaneStore::update_flow_run_if_status(self, input, expected_status).await
    }

    async fn complete_flow_run(
        &self,
        input: &CompleteFlowRunInput,
    ) -> Result<domain::FlowRunRecord> {
        PgControlPlaneStore::complete_flow_run(self, input).await
    }

    async fn get_checkpoint(
        &self,
        flow_run_id: Uuid,
        checkpoint_id: Uuid,
    ) -> Result<Option<domain::CheckpointRecord>> {
        PgControlPlaneStore::get_checkpoint(self, flow_run_id, checkpoint_id).await
    }

    async fn create_checkpoint(
        &self,
        input: &CreateCheckpointInput,
    ) -> Result<domain::CheckpointRecord> {
        PgControlPlaneStore::create_checkpoint(self, input).await
    }

    async fn create_callback_task(
        &self,
        input: &CreateCallbackTaskInput,
    ) -> Result<domain::CallbackTaskRecord> {
        PgControlPlaneStore::create_callback_task(self, input).await
    }

    async fn get_callback_task(
        &self,
        callback_task_id: Uuid,
    ) -> Result<Option<domain::CallbackTaskRecord>> {
        PgControlPlaneStore::get_callback_task(self, callback_task_id).await
    }

    async fn complete_callback_task(
        &self,
        input: &CompleteCallbackTaskInput,
    ) -> Result<domain::CallbackTaskRecord> {
        PgControlPlaneStore::complete_callback_task(self, input).await
    }

    async fn append_run_event(
        &self,
        input: &AppendRunEventInput,
    ) -> Result<domain::RunEventRecord> {
        PgControlPlaneStore::append_run_event(self, input).await
    }

    async fn append_run_events(
        &self,
        inputs: &[AppendRunEventInput],
    ) -> Result<Vec<domain::RunEventRecord>> {
        PgControlPlaneStore::append_run_events(self, inputs).await
    }

    async fn update_flow_run_payloads(
        &self,
        input: &UpdateFlowRunPayloadsInput,
    ) -> Result<domain::FlowRunRecord> {
        PgControlPlaneStore::update_flow_run_payloads(self, input).await
    }

    async fn update_node_run_payloads(
        &self,
        input: &UpdateNodeRunPayloadsInput,
    ) -> Result<domain::NodeRunRecord> {
        PgControlPlaneStore::update_node_run_payloads(self, input).await
    }

    async fn update_run_event_payload(
        &self,
        input: &UpdateRunEventPayloadInput,
    ) -> Result<domain::RunEventRecord> {
        PgControlPlaneStore::update_run_event_payload(self, input).await
    }

    async fn create_runtime_debug_artifact(
        &self,
        input: &CreateRuntimeDebugArtifactInput,
    ) -> Result<domain::RuntimeDebugArtifactRecord> {
        PgControlPlaneStore::create_runtime_debug_artifact(self, input).await
    }

    async fn get_runtime_debug_artifact(
        &self,
        input: &GetRuntimeDebugArtifactInput,
    ) -> Result<Option<domain::RuntimeDebugArtifactRecord>> {
        PgControlPlaneStore::get_runtime_debug_artifact(self, input).await
    }

    async fn get_data_model_side_effect_receipt(
        &self,
        workspace_id: Uuid,
        idempotency_key: &str,
    ) -> Result<Option<domain::DataModelSideEffectReceiptRecord>> {
        PgControlPlaneStore::get_data_model_side_effect_receipt(self, workspace_id, idempotency_key)
            .await
    }

    async fn claim_data_model_side_effect_receipt(
        &self,
        input: &UpsertDataModelSideEffectReceiptInput,
    ) -> Result<DataModelSideEffectReceiptClaim> {
        PgControlPlaneStore::claim_data_model_side_effect_receipt(self, input).await
    }

    async fn upsert_data_model_side_effect_receipt(
        &self,
        input: &UpsertDataModelSideEffectReceiptInput,
    ) -> Result<domain::DataModelSideEffectReceiptRecord> {
        PgControlPlaneStore::upsert_data_model_side_effect_receipt(self, input).await
    }

    async fn append_runtime_span(
        &self,
        input: &AppendRuntimeSpanInput,
    ) -> Result<domain::RuntimeSpanRecord> {
        PgControlPlaneStore::append_runtime_span(self, input).await
    }

    async fn append_runtime_event(
        &self,
        input: &AppendRuntimeEventInput,
    ) -> Result<domain::RuntimeEventRecord> {
        PgControlPlaneStore::append_runtime_event(self, input).await
    }

    async fn append_runtime_events(
        &self,
        inputs: &[AppendRuntimeEventInput],
    ) -> Result<Vec<domain::RuntimeEventRecord>> {
        PgControlPlaneStore::append_runtime_events(self, inputs).await
    }

    async fn append_runtime_item(
        &self,
        input: &AppendRuntimeItemInput,
    ) -> Result<domain::RuntimeItemRecord> {
        PgControlPlaneStore::append_runtime_item(self, input).await
    }

    async fn append_context_projection(
        &self,
        input: &AppendContextProjectionInput,
    ) -> Result<domain::ContextProjectionRecord> {
        PgControlPlaneStore::append_context_projection(self, input).await
    }

    async fn append_usage_ledger(
        &self,
        input: &AppendUsageLedgerInput,
    ) -> Result<domain::UsageLedgerRecord> {
        PgControlPlaneStore::append_usage_ledger(self, input).await
    }

    async fn append_cost_ledger(
        &self,
        input: &AppendCostLedgerInput,
    ) -> Result<domain::CostLedgerRecord> {
        PgControlPlaneStore::append_cost_ledger(self, input).await
    }

    async fn append_credit_ledger(
        &self,
        input: &AppendCreditLedgerInput,
    ) -> Result<domain::CreditLedgerRecord> {
        PgControlPlaneStore::append_credit_ledger(self, input).await
    }

    async fn append_billing_session(
        &self,
        input: &AppendBillingSessionInput,
    ) -> Result<domain::BillingSessionRecord> {
        PgControlPlaneStore::append_billing_session(self, input).await
    }

    async fn append_audit_hash(
        &self,
        flow_run_id: Uuid,
        fact_table: &str,
        fact_id: Uuid,
        payload: serde_json::Value,
    ) -> Result<domain::AuditHashRecord> {
        PgControlPlaneStore::append_audit_hash(self, flow_run_id, fact_table, fact_id, payload)
            .await
    }

    async fn append_model_failover_attempt_ledger(
        &self,
        input: &AppendModelFailoverAttemptLedgerInput,
    ) -> Result<domain::ModelFailoverAttemptLedgerRecord> {
        PgControlPlaneStore::append_model_failover_attempt_ledger(self, input).await
    }

    async fn link_usage_ledger_to_model_failover_attempt(
        &self,
        input: &LinkUsageLedgerToModelFailoverAttemptInput,
    ) -> Result<domain::ModelFailoverAttemptLedgerRecord> {
        PgControlPlaneStore::link_usage_ledger_to_model_failover_attempt(self, input).await
    }

    async fn append_capability_invocation(
        &self,
        input: &AppendCapabilityInvocationInput,
    ) -> Result<domain::CapabilityInvocationRecord> {
        PgControlPlaneStore::append_capability_invocation(self, input).await
    }

    async fn list_runtime_spans(
        &self,
        flow_run_id: Uuid,
    ) -> Result<Vec<domain::RuntimeSpanRecord>> {
        PgControlPlaneStore::list_runtime_spans(self, flow_run_id).await
    }

    async fn list_runtime_events(
        &self,
        flow_run_id: Uuid,
        after_sequence: i64,
    ) -> Result<Vec<domain::RuntimeEventRecord>> {
        PgControlPlaneStore::list_runtime_events(self, flow_run_id, after_sequence).await
    }

    async fn list_runtime_items(
        &self,
        flow_run_id: Uuid,
    ) -> Result<Vec<domain::RuntimeItemRecord>> {
        PgControlPlaneStore::list_runtime_items(self, flow_run_id).await
    }

    async fn list_context_projections(
        &self,
        flow_run_id: Uuid,
    ) -> Result<Vec<domain::ContextProjectionRecord>> {
        PgControlPlaneStore::list_context_projections(self, flow_run_id).await
    }

    async fn list_usage_ledger(&self, flow_run_id: Uuid) -> Result<Vec<domain::UsageLedgerRecord>> {
        PgControlPlaneStore::list_usage_ledger(self, flow_run_id).await
    }

    async fn list_model_failover_attempt_ledger(
        &self,
        flow_run_id: Uuid,
    ) -> Result<Vec<domain::ModelFailoverAttemptLedgerRecord>> {
        PgControlPlaneStore::list_model_failover_attempt_ledger(self, flow_run_id).await
    }

    async fn list_capability_invocations(
        &self,
        flow_run_id: Uuid,
    ) -> Result<Vec<domain::CapabilityInvocationRecord>> {
        PgControlPlaneStore::list_capability_invocations(self, flow_run_id).await
    }

    async fn list_application_runs(
        &self,
        application_id: Uuid,
    ) -> Result<Vec<domain::ApplicationRunSummary>> {
        PgControlPlaneStore::list_application_runs(self, application_id).await
    }

    async fn get_application_run_detail(
        &self,
        application_id: Uuid,
        flow_run_id: Uuid,
    ) -> Result<Option<domain::ApplicationRunDetail>> {
        PgControlPlaneStore::get_application_run_detail(self, application_id, flow_run_id).await
    }

    async fn get_latest_node_run(
        &self,
        application_id: Uuid,
        node_id: &str,
    ) -> Result<Option<domain::NodeLastRun>> {
        PgControlPlaneStore::get_latest_node_run(self, application_id, node_id).await
    }
}
