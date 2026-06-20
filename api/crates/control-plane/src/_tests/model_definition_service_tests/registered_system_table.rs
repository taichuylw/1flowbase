use super::*;

fn registered_system_table_model(model_id: Uuid) -> ModelDefinitionRecord {
    ModelDefinitionRecord {
        protection: DataModelProtection {
            owner_kind: DataModelOwnerKind::Core,
            owner_id: None,
            is_protected: true,
        },
        fields: vec![ModelFieldRecord {
            id: Uuid::now_v7(),
            data_model_id: model_id,
            code: "status".into(),
            title: "Status".into(),
            physical_column_name: "status".into(),
            external_field_key: None,
            field_kind: ModelFieldKind::String,
            is_system: true,
            is_writable: false,
            is_required: true,
            is_unique: false,
            default_value: None,
            display_interface: None,
            display_options: json!({}),
            relation_target_model_id: None,
            relation_options: json!({}),
            sort_order: 0,
            availability_status: domain::MetadataAvailabilityStatus::Available,
        }],
        ..system_model(model_id)
    }
}

#[tokio::test]
async fn registered_system_table_rejects_physical_field_add_update_and_delete() {
    let actor_user_id = Uuid::now_v7();
    let actor_workspace_id = Uuid::now_v7();
    let model_id = Uuid::now_v7();
    let model = registered_system_table_model(model_id);
    let field_id = model.fields[0].id;
    let repository = ScopedModelDefinitionRepository::new(scoped_manager_in_workspace(
        actor_user_id,
        actor_workspace_id,
    ))
    .with_model(model);
    let service = ModelDefinitionService::new(repository.clone());

    let add_error = service
        .add_field(AddModelFieldCommand {
            actor_user_id,
            model_id,
            code: "new_physical_column".into(),
            title: "New physical column".into(),
            external_field_key: None,
            field_kind: ModelFieldKind::String,
            is_required: false,
            is_unique: false,
            default_value: None,
            display_interface: None,
            display_options: json!({}),
            relation_target_model_id: None,
            relation_options: json!({}),
        })
        .await
        .unwrap_err();
    assert!(add_error
        .to_string()
        .contains("registered_system_table_physical_fields_readonly"));

    let update_error = service
        .update_field(UpdateModelFieldCommand {
            actor_user_id,
            model_id,
            field_id,
            title: "Status".into(),
            is_required: false,
            is_unique: false,
            default_value: None,
            display_interface: None,
            display_options: json!({}),
            relation_options: json!({}),
        })
        .await
        .unwrap_err();
    assert!(update_error
        .to_string()
        .contains("registered_system_table_physical_fields_readonly"));

    let delete_error = service
        .delete_field(DeleteModelFieldCommand {
            actor_user_id,
            model_id,
            field_id,
            confirmed: true,
        })
        .await
        .unwrap_err();
    assert!(delete_error
        .to_string()
        .contains("registered_system_table_physical_fields_readonly"));

    let stored = repository
        .models
        .lock()
        .expect("model lock poisoned")
        .get(&model_id)
        .cloned()
        .unwrap();
    assert_eq!(stored.fields.len(), 1);
    assert_eq!(stored.fields[0].is_required, true);
}

#[tokio::test]
async fn registered_system_table_allows_non_physical_field_metadata_update() {
    let actor_user_id = Uuid::now_v7();
    let actor_workspace_id = Uuid::now_v7();
    let model_id = Uuid::now_v7();
    let model = registered_system_table_model(model_id);
    let field_id = model.fields[0].id;
    let repository = ScopedModelDefinitionRepository::new(scoped_manager_in_workspace(
        actor_user_id,
        actor_workspace_id,
    ))
    .with_model(model);
    let service = ModelDefinitionService::new(repository.clone());

    let updated = service
        .update_field(UpdateModelFieldCommand {
            actor_user_id,
            model_id,
            field_id,
            title: "Status display".into(),
            is_required: true,
            is_unique: false,
            default_value: None,
            display_interface: Some("badge".into()),
            display_options: json!({ "tone": "neutral" }),
            relation_options: json!({}),
        })
        .await
        .unwrap();

    assert_eq!(updated.title, "Status display");
    assert_eq!(updated.display_interface.as_deref(), Some("badge"));
    assert_eq!(updated.is_required, true);
}
