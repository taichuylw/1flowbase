use super::*;

#[tokio::test]
async fn update_model_status_forces_draft_exposure_and_downgrades_direct_ready() {
    let service = ModelDefinitionService::for_tests();
    let created = service
        .create_model(CreateModelDefinitionCommand {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::Workspace,
            data_source_instance_id: None,
            external_resource_key: None,
            external_table_id: None,
            code: "status_orders".into(),
            title: "Status Orders".into(),
            status: None,
        })
        .await
        .unwrap();

    let draft = service
        .update_model_status(UpdateModelDefinitionStatusCommand {
            actor_user_id: Uuid::nil(),
            model_id: created.id,
            status: DataModelStatus::Draft,
            api_exposure_status: ApiExposureStatus::PublishedNotExposed,
        })
        .await
        .unwrap();
    assert_eq!(draft.status, DataModelStatus::Draft);
    assert_eq!(draft.api_exposure_status, ApiExposureStatus::Draft);

    let direct_ready = service
        .update_model_status(UpdateModelDefinitionStatusCommand {
            actor_user_id: Uuid::nil(),
            model_id: created.id,
            status: DataModelStatus::Published,
            api_exposure_status: ApiExposureStatus::ApiExposedReady,
        })
        .await
        .unwrap();

    assert_eq!(
        direct_ready.api_exposure_status,
        ApiExposureStatus::PublishedNotExposed
    );
}

#[tokio::test]
async fn update_model_status_downgrades_raw_ready_without_readiness_facts() {
    let service = ModelDefinitionService::for_tests();
    let created = service
        .create_model(CreateModelDefinitionCommand {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::Workspace,
            data_source_instance_id: None,
            external_resource_key: None,
            external_table_id: None,
            code: "raw_ready_orders".into(),
            title: "Raw Ready Orders".into(),
            status: None,
        })
        .await
        .unwrap();

    let updated = service
        .update_model_status(UpdateModelDefinitionStatusCommand {
            actor_user_id: Uuid::nil(),
            model_id: created.id,
            status: DataModelStatus::Published,
            api_exposure_status: ApiExposureStatus::ApiExposedReady,
        })
        .await
        .unwrap();

    assert_eq!(updated.status, DataModelStatus::Published);
    assert_eq!(
        updated.api_exposure_status,
        ApiExposureStatus::PublishedNotExposed
    );
}

#[tokio::test]
async fn get_model_maps_stored_ready_or_no_permission_without_api_key_to_not_exposed() {
    let actor_user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    for stored_status in [
        ApiExposureStatus::ApiExposedReady,
        ApiExposureStatus::ApiExposedNoPermission,
    ] {
        let model_id = Uuid::now_v7();
        let repository =
            ScopedModelDefinitionRepository::new(actor_in_workspace(actor_user_id, workspace_id))
                .with_model(ModelDefinitionRecord {
                    api_exposure_status: stored_status,
                    ..model_in_workspace(model_id, workspace_id)
                });
        let service = ModelDefinitionService::new(repository);

        let model = service.get_model(actor_user_id, model_id).await.unwrap();

        assert_eq!(
            model.api_exposure_status,
            ApiExposureStatus::PublishedNotExposed
        );
    }
}

#[tokio::test]
async fn get_model_computes_ready_from_api_key_scope_grant_and_audit_facts() {
    let repository = InMemoryModelDefinitionRepository::default();
    let service = ModelDefinitionService::new(repository.clone());
    let created = service
        .create_model(CreateModelDefinitionCommand {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::Workspace,
            data_source_instance_id: None,
            external_resource_key: None,
            external_table_id: None,
            code: "ready_orders".into(),
            title: "Ready Orders".into(),
            status: None,
        })
        .await
        .unwrap();
    repository.add_api_key_readiness(ApiKeyDataModelReadinessRecord {
        api_key_id: Uuid::now_v7(),
        data_model_id: created.id,
        scope_kind: DataModelScopeKind::Workspace,
        scope_id: Uuid::nil(),
        key_enabled: true,
        expires_at: None,
        allow_list: true,
        allow_get: false,
        allow_create: false,
        allow_update: false,
        allow_delete: false,
    });

    let ready = service.get_model(Uuid::nil(), created.id).await.unwrap();

    assert_eq!(
        ready.api_exposure_status,
        ApiExposureStatus::ApiExposedReady
    );
}

#[tokio::test]
async fn update_model_status_keeps_disabled_effective_exposure_not_ready() {
    let service = ModelDefinitionService::for_tests();
    let created = service
        .create_model(CreateModelDefinitionCommand {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::Workspace,
            data_source_instance_id: None,
            external_resource_key: None,
            external_table_id: None,
            code: "disabled_ready_orders".into(),
            title: "Disabled Ready Orders".into(),
            status: None,
        })
        .await
        .unwrap();

    let updated = service
        .update_model_status(UpdateModelDefinitionStatusCommand {
            actor_user_id: Uuid::nil(),
            model_id: created.id,
            status: DataModelStatus::Disabled,
            api_exposure_status: ApiExposureStatus::ApiExposedReady,
        })
        .await
        .unwrap();

    assert_eq!(updated.status, DataModelStatus::Disabled);
    assert_eq!(
        updated.api_exposure_status,
        ApiExposureStatus::ApiExposedNoPermission
    );
}

#[tokio::test]
async fn update_model_status_audits_effective_api_exposure_transition() {
    let repository = InMemoryModelDefinitionRepository::default();
    let service = ModelDefinitionService::new(repository.clone());
    let created = service
        .create_model(CreateModelDefinitionCommand {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::Workspace,
            data_source_instance_id: None,
            external_resource_key: None,
            external_table_id: None,
            code: "transition_audit_orders".into(),
            title: "Transition Audit Orders".into(),
            status: Some(DataModelStatus::Draft),
        })
        .await
        .unwrap();
    repository.add_api_key_readiness(ApiKeyDataModelReadinessRecord {
        api_key_id: Uuid::now_v7(),
        data_model_id: created.id,
        scope_kind: DataModelScopeKind::Workspace,
        scope_id: Uuid::nil(),
        key_enabled: true,
        expires_at: None,
        allow_list: true,
        allow_get: false,
        allow_create: false,
        allow_update: false,
        allow_delete: false,
    });

    service
        .update_model_status(UpdateModelDefinitionStatusCommand {
            actor_user_id: Uuid::nil(),
            model_id: created.id,
            status: DataModelStatus::Published,
            api_exposure_status: ApiExposureStatus::ApiExposedReady,
        })
        .await
        .unwrap();

    assert!(repository
        .audit_events()
        .contains(&"state_model.api_exposure_status_changed".to_string()));
}
