use super::*;

#[tokio::test]
async fn update_scope_grant_records_audit_event() {
    let repository = InMemoryModelDefinitionRepository::default();
    let service = ModelDefinitionService::new(repository.clone());
    let created = service
        .create_model(CreateModelDefinitionCommand {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::Workspace,
            data_source_instance_id: None,
            external_resource_key: None,
            external_table_id: None,
            code: "scope_grant_audit_orders".into(),
            title: "Scope Grant Audit Orders".into(),
            status: None,
        })
        .await
        .unwrap();
    let grant = service
        .create_scope_grant(
            control_plane::model_definition::CreateScopeDataModelGrantCommand {
                actor_user_id: Uuid::nil(),
                scope_kind: DataModelScopeKind::System,
                scope_id: SYSTEM_SCOPE_ID,
                data_model_id: created.id,
                enabled: true,
                permission_profile: "scope_all".into(),
                confirm_unsafe_external_source_system_all: false,
            },
        )
        .await
        .unwrap();

    service
        .update_scope_grant(UpdateScopeDataModelGrantCommand {
            actor_user_id: Uuid::nil(),
            data_model_id: created.id,
            grant_id: grant.id,
            enabled: Some(false),
            permission_profile: Some("owner".into()),
            confirm_unsafe_external_source_system_all: false,
        })
        .await
        .unwrap();

    assert!(repository
        .audit_events()
        .contains(&"state_model.scope_grant_updated".to_string()));
}

#[tokio::test]
async fn non_root_scope_grant_create_rejects_system_and_other_workspace_scope() {
    let actor_user_id = Uuid::now_v7();
    let actor_workspace_id = Uuid::now_v7();
    let other_workspace_id = Uuid::now_v7();
    let model_id = Uuid::now_v7();
    let repository = ScopedModelDefinitionRepository::new(scoped_manager_in_workspace(
        actor_user_id,
        actor_workspace_id,
    ))
    .with_model(system_model(model_id));
    let service = ModelDefinitionService::new(repository);

    let system_error = service
        .create_scope_grant(
            control_plane::model_definition::CreateScopeDataModelGrantCommand {
                actor_user_id,
                scope_kind: DataModelScopeKind::System,
                scope_id: SYSTEM_SCOPE_ID,
                data_model_id: model_id,
                enabled: true,
                permission_profile: "scope_all".into(),
                confirm_unsafe_external_source_system_all: false,
            },
        )
        .await
        .unwrap_err();
    assert!(system_error.to_string().contains("permission_denied"));

    let other_workspace_error = service
        .create_scope_grant(
            control_plane::model_definition::CreateScopeDataModelGrantCommand {
                actor_user_id,
                scope_kind: DataModelScopeKind::Workspace,
                scope_id: other_workspace_id,
                data_model_id: model_id,
                enabled: true,
                permission_profile: "scope_all".into(),
                confirm_unsafe_external_source_system_all: false,
            },
        )
        .await
        .unwrap_err();
    assert!(other_workspace_error
        .to_string()
        .contains("permission_denied"));

    let current_workspace_grant = service
        .create_scope_grant(
            control_plane::model_definition::CreateScopeDataModelGrantCommand {
                actor_user_id,
                scope_kind: DataModelScopeKind::Workspace,
                scope_id: actor_workspace_id,
                data_model_id: model_id,
                enabled: true,
                permission_profile: "scope_all".into(),
                confirm_unsafe_external_source_system_all: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(current_workspace_grant.scope_id, actor_workspace_id);
}

#[tokio::test]
async fn non_root_scope_grant_update_delete_authorizes_existing_grant_scope() {
    let actor_user_id = Uuid::now_v7();
    let actor_workspace_id = Uuid::now_v7();
    let other_workspace_id = Uuid::now_v7();
    let model_id = Uuid::now_v7();
    let system_grant_id = Uuid::now_v7();
    let other_workspace_grant_id = Uuid::now_v7();
    let current_workspace_grant_id = Uuid::now_v7();
    let repository = ScopedModelDefinitionRepository::new(scoped_manager_in_workspace(
        actor_user_id,
        actor_workspace_id,
    ))
    .with_model(system_model(model_id))
    .with_grant(scope_grant(
        system_grant_id,
        model_id,
        DataModelScopeKind::System,
        SYSTEM_SCOPE_ID,
    ))
    .with_grant(scope_grant(
        other_workspace_grant_id,
        model_id,
        DataModelScopeKind::Workspace,
        other_workspace_id,
    ))
    .with_grant(scope_grant(
        current_workspace_grant_id,
        model_id,
        DataModelScopeKind::Workspace,
        actor_workspace_id,
    ));
    let service = ModelDefinitionService::new(repository);

    for grant_id in [system_grant_id, other_workspace_grant_id] {
        let update_error = service
            .update_scope_grant(UpdateScopeDataModelGrantCommand {
                actor_user_id,
                data_model_id: model_id,
                grant_id,
                enabled: Some(false),
                permission_profile: None,
                confirm_unsafe_external_source_system_all: false,
            })
            .await
            .unwrap_err();
        assert!(update_error.to_string().contains("permission_denied"));

        let delete_error = service
            .delete_scope_grant(DeleteScopeDataModelGrantCommand {
                actor_user_id,
                data_model_id: model_id,
                grant_id,
            })
            .await
            .unwrap_err();
        assert!(delete_error.to_string().contains("permission_denied"));
    }

    let updated = service
        .update_scope_grant(UpdateScopeDataModelGrantCommand {
            actor_user_id,
            data_model_id: model_id,
            grant_id: current_workspace_grant_id,
            enabled: Some(false),
            permission_profile: Some("owner".into()),
            confirm_unsafe_external_source_system_all: false,
        })
        .await
        .unwrap();
    assert_eq!(updated.scope_id, actor_workspace_id);
    assert!(!updated.enabled);

    let deleted = service
        .delete_scope_grant(DeleteScopeDataModelGrantCommand {
            actor_user_id,
            data_model_id: model_id,
            grant_id: current_workspace_grant_id,
        })
        .await
        .unwrap();
    assert_eq!(deleted.scope_id, actor_workspace_id);
}

#[tokio::test]
async fn root_scope_grant_lifecycle_can_manage_any_scope() {
    let actor_user_id = Uuid::now_v7();
    let actor_workspace_id = Uuid::now_v7();
    let other_workspace_id = Uuid::now_v7();
    let model_id = Uuid::now_v7();
    let repository =
        ScopedModelDefinitionRepository::new(actor_in_workspace(actor_user_id, actor_workspace_id))
            .with_model(system_model(model_id));
    let service = ModelDefinitionService::new(repository);

    let system_grant = service
        .create_scope_grant(
            control_plane::model_definition::CreateScopeDataModelGrantCommand {
                actor_user_id,
                scope_kind: DataModelScopeKind::System,
                scope_id: SYSTEM_SCOPE_ID,
                data_model_id: model_id,
                enabled: true,
                permission_profile: "scope_all".into(),
                confirm_unsafe_external_source_system_all: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(system_grant.scope_kind, DataModelScopeKind::System);

    let other_workspace_grant = service
        .create_scope_grant(
            control_plane::model_definition::CreateScopeDataModelGrantCommand {
                actor_user_id,
                scope_kind: DataModelScopeKind::Workspace,
                scope_id: other_workspace_id,
                data_model_id: model_id,
                enabled: true,
                permission_profile: "scope_all".into(),
                confirm_unsafe_external_source_system_all: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(other_workspace_grant.scope_id, other_workspace_id);

    let updated = service
        .update_scope_grant(UpdateScopeDataModelGrantCommand {
            actor_user_id,
            data_model_id: model_id,
            grant_id: system_grant.id,
            enabled: Some(false),
            permission_profile: Some("owner".into()),
            confirm_unsafe_external_source_system_all: false,
        })
        .await
        .unwrap();
    assert_eq!(updated.scope_kind, DataModelScopeKind::System);
    assert!(!updated.enabled);

    let deleted = service
        .delete_scope_grant(DeleteScopeDataModelGrantCommand {
            actor_user_id,
            data_model_id: model_id,
            grant_id: other_workspace_grant.id,
        })
        .await
        .unwrap();
    assert_eq!(deleted.scope_id, other_workspace_id);
}

#[tokio::test]
async fn unsafe_external_system_all_scope_grant_requires_explicit_confirmation() {
    let actor_user_id = Uuid::now_v7();
    let actor_workspace_id = Uuid::now_v7();
    let model_id = Uuid::now_v7();
    let repository =
        ScopedModelDefinitionRepository::new(actor_in_workspace(actor_user_id, actor_workspace_id))
            .with_model(unsafe_external_system_model(model_id));
    let service = ModelDefinitionService::new(repository);

    let error = service
        .create_scope_grant(CreateScopeDataModelGrantCommand {
            actor_user_id,
            scope_kind: DataModelScopeKind::System,
            scope_id: SYSTEM_SCOPE_ID,
            data_model_id: model_id,
            enabled: true,
            permission_profile: "system_all".into(),
            confirm_unsafe_external_source_system_all: false,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("confirmation"));

    let grant = service
        .create_scope_grant(CreateScopeDataModelGrantCommand {
            actor_user_id,
            scope_kind: DataModelScopeKind::System,
            scope_id: SYSTEM_SCOPE_ID,
            data_model_id: model_id,
            enabled: true,
            permission_profile: "system_all".into(),
            confirm_unsafe_external_source_system_all: true,
        })
        .await
        .unwrap();
    assert_eq!(
        grant.permission_profile,
        domain::ScopeDataModelPermissionProfile::SystemAll
    );
}

#[tokio::test]
async fn unsafe_external_workspace_scope_system_all_grant_requires_explicit_confirmation() {
    let actor_user_id = Uuid::now_v7();
    let actor_workspace_id = Uuid::now_v7();
    let model_id = Uuid::now_v7();
    let repository =
        ScopedModelDefinitionRepository::new(actor_in_workspace(actor_user_id, actor_workspace_id))
            .with_model(unsafe_external_system_model(model_id));
    let service = ModelDefinitionService::new(repository);

    let error = service
        .create_scope_grant(CreateScopeDataModelGrantCommand {
            actor_user_id,
            scope_kind: DataModelScopeKind::Workspace,
            scope_id: actor_workspace_id,
            data_model_id: model_id,
            enabled: true,
            permission_profile: "system_all".into(),
            confirm_unsafe_external_source_system_all: false,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("confirmation"));

    let grant = service
        .create_scope_grant(CreateScopeDataModelGrantCommand {
            actor_user_id,
            scope_kind: DataModelScopeKind::Workspace,
            scope_id: actor_workspace_id,
            data_model_id: model_id,
            enabled: true,
            permission_profile: "system_all".into(),
            confirm_unsafe_external_source_system_all: true,
        })
        .await
        .unwrap();
    assert_eq!(grant.scope_kind, DataModelScopeKind::Workspace);
    assert_eq!(grant.scope_id, actor_workspace_id);
    assert_eq!(
        grant.permission_profile,
        domain::ScopeDataModelPermissionProfile::SystemAll
    );
}

#[tokio::test]
async fn unsafe_external_system_all_scope_grant_update_requires_explicit_confirmation() {
    let actor_user_id = Uuid::now_v7();
    let actor_workspace_id = Uuid::now_v7();
    let model_id = Uuid::now_v7();
    let grant_id = Uuid::now_v7();
    let repository =
        ScopedModelDefinitionRepository::new(actor_in_workspace(actor_user_id, actor_workspace_id))
            .with_model(unsafe_external_system_model(model_id))
            .with_grant(scope_grant(
                grant_id,
                model_id,
                DataModelScopeKind::System,
                SYSTEM_SCOPE_ID,
            ));
    let service = ModelDefinitionService::new(repository);

    let error = service
        .update_scope_grant(UpdateScopeDataModelGrantCommand {
            actor_user_id,
            data_model_id: model_id,
            grant_id,
            enabled: Some(true),
            permission_profile: Some("system_all".into()),
            confirm_unsafe_external_source_system_all: false,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("confirmation"));

    let grant = service
        .update_scope_grant(UpdateScopeDataModelGrantCommand {
            actor_user_id,
            data_model_id: model_id,
            grant_id,
            enabled: Some(true),
            permission_profile: Some("system_all".into()),
            confirm_unsafe_external_source_system_all: true,
        })
        .await
        .unwrap();
    assert_eq!(
        grant.permission_profile,
        domain::ScopeDataModelPermissionProfile::SystemAll
    );
}

#[tokio::test]
async fn unsafe_external_workspace_scope_system_all_grant_update_requires_explicit_confirmation() {
    let actor_user_id = Uuid::now_v7();
    let actor_workspace_id = Uuid::now_v7();
    let model_id = Uuid::now_v7();
    let grant_id = Uuid::now_v7();
    let repository =
        ScopedModelDefinitionRepository::new(actor_in_workspace(actor_user_id, actor_workspace_id))
            .with_model(unsafe_external_system_model(model_id))
            .with_grant(scope_grant(
                grant_id,
                model_id,
                DataModelScopeKind::Workspace,
                actor_workspace_id,
            ));
    let service = ModelDefinitionService::new(repository);

    let error = service
        .update_scope_grant(UpdateScopeDataModelGrantCommand {
            actor_user_id,
            data_model_id: model_id,
            grant_id,
            enabled: Some(true),
            permission_profile: Some("system_all".into()),
            confirm_unsafe_external_source_system_all: false,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("confirmation"));

    let grant = service
        .update_scope_grant(UpdateScopeDataModelGrantCommand {
            actor_user_id,
            data_model_id: model_id,
            grant_id,
            enabled: Some(true),
            permission_profile: Some("system_all".into()),
            confirm_unsafe_external_source_system_all: true,
        })
        .await
        .unwrap();
    assert_eq!(grant.scope_kind, DataModelScopeKind::Workspace);
    assert_eq!(grant.scope_id, actor_workspace_id);
    assert_eq!(
        grant.permission_profile,
        domain::ScopeDataModelPermissionProfile::SystemAll
    );
}

#[tokio::test]
async fn safe_external_system_all_scope_grant_does_not_require_risk_confirmation() {
    let actor_user_id = Uuid::now_v7();
    let actor_workspace_id = Uuid::now_v7();
    let model_id = Uuid::now_v7();
    let grant_id = Uuid::now_v7();
    let repository =
        ScopedModelDefinitionRepository::new(actor_in_workspace(actor_user_id, actor_workspace_id))
            .with_model(safe_external_system_model(model_id))
            .with_grant(scope_grant(
                grant_id,
                model_id,
                DataModelScopeKind::Workspace,
                actor_workspace_id,
            ));
    let service = ModelDefinitionService::new(repository);

    let created = service
        .create_scope_grant(CreateScopeDataModelGrantCommand {
            actor_user_id,
            scope_kind: DataModelScopeKind::Workspace,
            scope_id: actor_workspace_id,
            data_model_id: model_id,
            enabled: true,
            permission_profile: "system_all".into(),
            confirm_unsafe_external_source_system_all: false,
        })
        .await
        .unwrap();
    assert_eq!(
        created.permission_profile,
        domain::ScopeDataModelPermissionProfile::SystemAll
    );

    let updated = service
        .update_scope_grant(UpdateScopeDataModelGrantCommand {
            actor_user_id,
            data_model_id: model_id,
            grant_id,
            enabled: Some(true),
            permission_profile: Some("system_all".into()),
            confirm_unsafe_external_source_system_all: false,
        })
        .await
        .unwrap();
    assert_eq!(
        updated.permission_profile,
        domain::ScopeDataModelPermissionProfile::SystemAll
    );
}

#[tokio::test]
async fn main_source_system_all_scope_grant_does_not_require_external_risk_confirmation() {
    let actor_user_id = Uuid::now_v7();
    let actor_workspace_id = Uuid::now_v7();
    let model_id = Uuid::now_v7();
    let repository =
        ScopedModelDefinitionRepository::new(actor_in_workspace(actor_user_id, actor_workspace_id))
            .with_model(system_model(model_id));
    let service = ModelDefinitionService::new(repository);

    let grant = service
        .create_scope_grant(CreateScopeDataModelGrantCommand {
            actor_user_id,
            scope_kind: DataModelScopeKind::System,
            scope_id: SYSTEM_SCOPE_ID,
            data_model_id: model_id,
            enabled: true,
            permission_profile: "system_all".into(),
            confirm_unsafe_external_source_system_all: false,
        })
        .await
        .unwrap();

    assert_eq!(
        grant.permission_profile,
        domain::ScopeDataModelPermissionProfile::SystemAll
    );
}

#[tokio::test]
async fn delete_scope_grant_records_audit_event() {
    let repository = InMemoryModelDefinitionRepository::default();
    let service = ModelDefinitionService::new(repository.clone());
    let created = service
        .create_model(CreateModelDefinitionCommand {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::Workspace,
            data_source_instance_id: None,
            external_resource_key: None,
            external_table_id: None,
            code: "delete_scope_grant_audit_orders".into(),
            title: "Delete Scope Grant Audit Orders".into(),
            status: None,
        })
        .await
        .unwrap();
    let grant = service
        .create_scope_grant(
            control_plane::model_definition::CreateScopeDataModelGrantCommand {
                actor_user_id: Uuid::nil(),
                scope_kind: DataModelScopeKind::System,
                scope_id: SYSTEM_SCOPE_ID,
                data_model_id: created.id,
                enabled: true,
                permission_profile: "scope_all".into(),
                confirm_unsafe_external_source_system_all: false,
            },
        )
        .await
        .unwrap();

    service
        .delete_scope_grant(DeleteScopeDataModelGrantCommand {
            actor_user_id: Uuid::nil(),
            data_model_id: created.id,
            grant_id: grant.id,
        })
        .await
        .unwrap();

    assert!(repository
        .audit_events()
        .contains(&"state_model.scope_grant_deleted".to_string()));
}

#[tokio::test]
async fn delete_scope_grant_rejects_invisible_model_and_wrong_model_grant_pair() {
    let actor_user_id = Uuid::now_v7();
    let actor_workspace_id = Uuid::now_v7();
    let foreign_workspace_id = Uuid::now_v7();
    let foreign_model_id = Uuid::now_v7();
    let grant_model_id = Uuid::now_v7();
    let wrong_model_id = Uuid::now_v7();
    let repository =
        ScopedModelDefinitionRepository::new(actor_in_workspace(actor_user_id, actor_workspace_id))
            .with_model(model_in_workspace(foreign_model_id, foreign_workspace_id))
            .with_model(model_in_workspace(grant_model_id, actor_workspace_id))
            .with_model(model_in_workspace(wrong_model_id, actor_workspace_id));
    let service = ModelDefinitionService::new(repository.clone());
    let grant = service
        .create_scope_grant(
            control_plane::model_definition::CreateScopeDataModelGrantCommand {
                actor_user_id,
                scope_kind: DataModelScopeKind::Workspace,
                scope_id: actor_workspace_id,
                data_model_id: grant_model_id,
                enabled: true,
                permission_profile: "scope_all".into(),
                confirm_unsafe_external_source_system_all: false,
            },
        )
        .await
        .unwrap();

    let invisible_error = service
        .delete_scope_grant(DeleteScopeDataModelGrantCommand {
            actor_user_id,
            data_model_id: foreign_model_id,
            grant_id: grant.id,
        })
        .await
        .unwrap_err();
    assert!(invisible_error.to_string().contains("model_definition"));

    let wrong_pair_error = service
        .delete_scope_grant(DeleteScopeDataModelGrantCommand {
            actor_user_id,
            data_model_id: wrong_model_id,
            grant_id: grant.id,
        })
        .await
        .unwrap_err();
    assert!(wrong_pair_error
        .to_string()
        .contains("scope_data_model_grant"));

    assert!(!repository
        .audit_events()
        .contains(&"state_model.scope_grant_deleted".to_string()));
}
