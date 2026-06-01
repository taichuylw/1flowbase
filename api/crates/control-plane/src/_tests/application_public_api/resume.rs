use control_plane::application_public_api::{
    api_keys::{ApplicationApiKeyService, CreateApplicationApiKeyCommand},
    mapping::{
        ApplicationApiMappingConfig, ApplicationApiMappingInput, ApplicationApiMappingOutput,
    },
    native::{
        ApplicationNativeRunService, CreateNativeRunCommand, NativeRunValidationError,
        ResumeNativeRunCommand,
    },
    publications::{ApplicationPublicationService, PublishApplicationCommand},
    ApplicationPublicApiTestHarness,
};
use serde_json::json;
use uuid::Uuid;

fn actor_user_id() -> Uuid {
    Uuid::from_u128(0x11111111111111111111111111111111)
}

fn other_user_id() -> Uuid {
    Uuid::from_u128(0x22222222222222222222222222222222)
}

fn published_mapping() -> ApplicationApiMappingConfig {
    ApplicationApiMappingConfig {
        input: ApplicationApiMappingInput {
            query_target: "node-start.query".into(),
            model_target: None,
            inputs_target: None,
            history_target: None,
            attachments_target: None,
        },
        output: ApplicationApiMappingOutput::default(),
    }
}

async fn issue_key(
    harness: &ApplicationPublicApiTestHarness,
    application_id: Uuid,
    owner_user_id: Uuid,
) -> String {
    ApplicationApiKeyService::new(harness.repository())
        .create_api_key(CreateApplicationApiKeyCommand {
            actor_user_id: owner_user_id,
            application_id,
            name: "Native runner".into(),
            expires_at: None,
        })
        .await
        .unwrap()
        .token
}

async fn publish_application(
    harness: &ApplicationPublicApiTestHarness,
    application_id: Uuid,
    owner_user_id: Uuid,
) {
    ApplicationPublicationService::new(harness.repository())
        .publish_active_version(PublishApplicationCommand {
            actor_user_id: owner_user_id,
            application_id,
            mapping: published_mapping(),
            api_enabled: true,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn native_resume_rejects_callback_task_from_another_run() {
    let harness = ApplicationPublicApiTestHarness::new();
    let application = harness.seed_application(actor_user_id(), "Resume App");
    let token = issue_key(&harness, application.id, actor_user_id()).await;
    publish_application(&harness, application.id, actor_user_id()).await;
    let repository = harness.repository();
    let service = ApplicationNativeRunService::new(repository.clone());
    let first = service
        .create_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: serde_json::from_value(json!({ "query": "First" })).unwrap(),
        })
        .await
        .unwrap();
    let second = service
        .create_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: serde_json::from_value(json!({ "query": "Second" })).unwrap(),
        })
        .await
        .unwrap();
    let callback_task = repository.seed_pending_callback_task(second.id);

    let error = service
        .resume_native_run(ResumeNativeRunCommand {
            bearer_token: token,
            run_id: first.id,
            callback_task_id: callback_task.id,
            response_payload: json!({ "answer": "approved" }),
            response_mode: Some("blocking".into()),
        })
        .await
        .unwrap_err();

    assert_eq!(error, NativeRunValidationError::Forbidden);
}

#[tokio::test]
async fn native_resume_validates_ownership_before_execution_continuation_boundary() {
    let harness = ApplicationPublicApiTestHarness::new();
    let first_application = harness.seed_application(actor_user_id(), "Owned Resume App");
    let second_application = harness.seed_application(other_user_id(), "Other Resume App");
    let first_token = issue_key(&harness, first_application.id, actor_user_id()).await;
    let second_token = issue_key(&harness, second_application.id, other_user_id()).await;
    publish_application(&harness, first_application.id, actor_user_id()).await;
    publish_application(&harness, second_application.id, other_user_id()).await;
    let repository = harness.repository();
    let service = ApplicationNativeRunService::new(repository.clone());
    let run = service
        .create_native_run(CreateNativeRunCommand {
            bearer_token: first_token,
            request: serde_json::from_value(json!({ "query": "First" })).unwrap(),
        })
        .await
        .unwrap();
    let callback_task = repository.seed_pending_callback_task(run.id);

    let error = service
        .resume_native_run(ResumeNativeRunCommand {
            bearer_token: second_token,
            run_id: run.id,
            callback_task_id: callback_task.id,
            response_payload: json!({ "answer": "approved" }),
            response_mode: Some("streaming".into()),
        })
        .await
        .unwrap_err();

    assert_eq!(error, NativeRunValidationError::Forbidden);
}

#[tokio::test]
async fn native_get_run_exposes_pending_callback_required_action() {
    let harness = ApplicationPublicApiTestHarness::new();
    let application = harness.seed_application(actor_user_id(), "Required Action App");
    let token = issue_key(&harness, application.id, actor_user_id()).await;
    publish_application(&harness, application.id, actor_user_id()).await;
    let repository = harness.repository();
    let service = ApplicationNativeRunService::new(repository.clone());
    let run = service
        .create_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: serde_json::from_value(json!({ "query": "First" })).unwrap(),
        })
        .await
        .unwrap();
    let callback_task = repository.seed_pending_callback_task(run.id);

    let result = service
        .get_native_run(
            control_plane::application_public_api::native::GetNativeRunCommand {
                bearer_token: token,
                run_id: run.id,
            },
        )
        .await
        .unwrap();

    assert_eq!(
        result.status,
        control_plane::application_public_api::native::NativeRunStatus::Waiting
    );
    let required_action = result
        .required_action
        .expect("pending callback should be exposed");
    assert_eq!(required_action.action_type, "callback");
    assert_eq!(
        required_action.payload["callback_task_id"],
        json!(callback_task.id)
    );
    assert_eq!(
        required_action.payload["request_payload"],
        callback_task.request_payload
    );
}

#[tokio::test]
async fn native_resume_owned_callback_records_stable_resume_request_event() {
    let harness = ApplicationPublicApiTestHarness::new();
    let application = harness.seed_application(actor_user_id(), "Resume App");
    let token = issue_key(&harness, application.id, actor_user_id()).await;
    publish_application(&harness, application.id, actor_user_id()).await;
    let repository = harness.repository();
    let service = ApplicationNativeRunService::new(repository.clone());
    let run = service
        .create_native_run(CreateNativeRunCommand {
            bearer_token: token.clone(),
            request: serde_json::from_value(json!({ "query": "First" })).unwrap(),
        })
        .await
        .unwrap();
    let callback_task = repository.seed_pending_callback_task(run.id);

    let error = service
        .resume_native_run(ResumeNativeRunCommand {
            bearer_token: token,
            run_id: run.id,
            callback_task_id: callback_task.id,
            response_payload: json!({ "answer": "approved" }),
            response_mode: Some("blocking".into()),
        })
        .await
        .unwrap_err();

    assert_eq!(
        error,
        NativeRunValidationError::ResumeContinuationNotImplemented
    );
    let resume_events: Vec<_> = repository
        .run_events(run.id)
        .into_iter()
        .filter(|event| event.event_type == "public_run_resume_requested")
        .collect();
    assert_eq!(resume_events.len(), 1);
    assert_eq!(
        resume_events[0].payload["callback_task_id"],
        json!(callback_task.id)
    );
    assert_eq!(
        resume_events[0].payload["response_payload"],
        json!({ "answer": "approved" })
    );
    assert!(!resume_events[0]
        .payload
        .as_object()
        .unwrap()
        .contains_key("todo"));
}
