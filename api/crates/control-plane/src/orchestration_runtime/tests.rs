use super::*;
use crate::{errors::ControlPlaneError, ports::ModelProviderRepository};

#[tokio::test]
async fn orchestration_runtime_resolve_llm_instance_does_not_fallback_when_selected_instance_is_missing(
) {
    let repository = test_support::InMemoryOrchestrationRuntimeRepository::with_permissions(vec![]);
    let (alpha_instance_id, _) = repository.seed_included_provider_instances();
    let invoker = RuntimeProviderInvoker {
        repository,
        runtime: test_support::InMemoryProviderRuntime::default(),
        workspace_id: Uuid::nil(),
        provider_secret_master_key: "test-master-key".to_string(),
        live_provider_events: None,
        persist_events: None,
        runtime_event_stream: None,
        flow_run_id: None,
        active_node_id: None,
        active_node_run_id: None,
        answer_presentation: None,
    };

    let error = invoker
        .resolve_llm_instance(&orchestration_runtime::compiled_plan::CompiledLlmRuntime {
            provider_instance_id: Uuid::now_v7().to_string(),
            provider_code: "fixture_provider".to_string(),
            protocol: "openai_compatible".to_string(),
            model: "gpt-5.4-mini".to_string(),
            routing: None,
        })
        .await
        .expect_err("missing selected instance should fail");

    assert!(matches!(
        error.downcast_ref::<ControlPlaneError>(),
        Some(ControlPlaneError::InvalidInput("source_instance_id"))
    ));
    assert_ne!(alpha_instance_id, Uuid::nil());
}

#[tokio::test]
async fn orchestration_runtime_resolve_llm_instance_does_not_fallback_when_selected_instance_is_not_ready(
) {
    let repository = test_support::InMemoryOrchestrationRuntimeRepository::with_permissions(vec![]);
    let (_, backup_instance_id) = repository.seed_included_provider_instances();
    repository.set_instance_status(
        backup_instance_id,
        domain::ModelProviderInstanceStatus::Disabled,
    );
    let invoker = RuntimeProviderInvoker {
        repository,
        runtime: test_support::InMemoryProviderRuntime::default(),
        workspace_id: Uuid::nil(),
        provider_secret_master_key: "test-master-key".to_string(),
        live_provider_events: None,
        persist_events: None,
        runtime_event_stream: None,
        flow_run_id: None,
        active_node_id: None,
        active_node_run_id: None,
        answer_presentation: None,
    };

    let error = invoker
        .resolve_llm_instance(&orchestration_runtime::compiled_plan::CompiledLlmRuntime {
            provider_instance_id: backup_instance_id.to_string(),
            provider_code: "fixture_provider".to_string(),
            protocol: "openai_compatible".to_string(),
            model: "gpt-5.4-mini".to_string(),
            routing: None,
        })
        .await
        .expect_err("non-ready selected instance should fail");

    assert!(matches!(
        error.downcast_ref::<ControlPlaneError>(),
        Some(ControlPlaneError::InvalidInput("source_instance_id"))
    ));
}

#[tokio::test]
async fn orchestration_runtime_resolve_llm_instance_uses_selected_child_instance_without_provider_fallback(
) {
    let repository = test_support::InMemoryOrchestrationRuntimeRepository::with_permissions(vec![]);
    let (_, backup_instance_id) = repository.seed_included_provider_instances();
    repository.set_instance_enabled_models(backup_instance_id, vec!["gpt-5.4-mini"]);
    let invoker = RuntimeProviderInvoker {
        repository: repository.clone(),
        runtime: test_support::InMemoryProviderRuntime::default(),
        workspace_id: Uuid::nil(),
        provider_secret_master_key: "test-master-key".to_string(),
        live_provider_events: None,
        persist_events: None,
        runtime_event_stream: None,
        flow_run_id: None,
        active_node_id: None,
        active_node_run_id: None,
        answer_presentation: None,
    };

    let resolved = invoker
        .resolve_llm_instance(&orchestration_runtime::compiled_plan::CompiledLlmRuntime {
            provider_instance_id: backup_instance_id.to_string(),
            provider_code: "fixture_provider".to_string(),
            protocol: "openai_compatible".to_string(),
            model: "gpt-5.4-mini".to_string(),
            routing: None,
        })
        .await
        .expect("selected child instance should resolve");

    let repository_instance =
        ModelProviderRepository::get_instance(&repository, Uuid::nil(), backup_instance_id)
            .await
            .expect("instance lookup should succeed")
            .expect("instance should exist");
    assert_eq!(resolved.id, repository_instance.id);
    assert_eq!(resolved.display_name, repository_instance.display_name);
}

#[tokio::test]
async fn orchestration_runtime_resolve_llm_instance_rejects_model_only_present_in_catalog_cache() {
    let repository = test_support::InMemoryOrchestrationRuntimeRepository::with_permissions(vec![]);
    let selected_instance_id = repository.seed_provider_instance(
        "fixture_provider",
        "Cache Wider Than Enabled",
        true,
        domain::ModelProviderInstanceStatus::Ready,
        vec!["other-model"],
    );
    repository
        .set_instance_catalog_models(selected_instance_id, vec!["other-model", "gpt-5.4-mini"]);
    let invoker = RuntimeProviderInvoker {
        repository,
        runtime: test_support::InMemoryProviderRuntime::default(),
        workspace_id: Uuid::nil(),
        provider_secret_master_key: "test-master-key".to_string(),
        live_provider_events: None,
        persist_events: None,
        runtime_event_stream: None,
        flow_run_id: None,
        active_node_id: None,
        active_node_run_id: None,
        answer_presentation: None,
    };

    let error = invoker
        .resolve_llm_instance(&orchestration_runtime::compiled_plan::CompiledLlmRuntime {
            provider_instance_id: selected_instance_id.to_string(),
            provider_code: "fixture_provider".to_string(),
            protocol: "openai_compatible".to_string(),
            model: "gpt-5.4-mini".to_string(),
            routing: None,
        })
        .await
        .expect_err("model outside enabled_model_ids should fail");

    assert!(matches!(
        error.downcast_ref::<ControlPlaneError>(),
        Some(ControlPlaneError::InvalidInput("model"))
    ));
}
