use control_plane::mcp_management::{
    CreateMcpInstanceCommand, CreateMcpToolBindingCommand, CreateMcpToolCommand,
    McpManagementService, RefreshMcpToolDescriptionCommand, UpsertMcpGroupCommand,
};
use sqlx::PgPool;
use storage_postgres::{connect, run_migrations, PgControlPlaneStore};
use uuid::Uuid;

fn base_database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:1flowbase@127.0.0.1:35432/1flowbase".into())
}

async fn isolated_database_url() -> String {
    let admin_pool = PgPool::connect(&base_database_url()).await.unwrap();
    let schema = format!("test_{}", Uuid::now_v7().simple());
    sqlx::query(&format!("create schema if not exists {schema}"))
        .execute(&admin_pool)
        .await
        .unwrap();

    format!("{}?options=-csearch_path%3D{schema}", base_database_url())
}

async fn seed_store() -> (
    PgControlPlaneStore,
    domain::WorkspaceRecord,
    domain::UserRecord,
) {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);

    let tenant = store.upsert_root_tenant().await.unwrap();
    let workspace = store
        .upsert_workspace(tenant.id, "MCP Management")
        .await
        .unwrap();
    store
        .upsert_permission_catalog(&access_control::permission_catalog())
        .await
        .unwrap();
    store.upsert_builtin_roles(workspace.id).await.unwrap();
    store
        .upsert_authenticator(&domain::AuthenticatorRecord {
            name: "password-local".into(),
            auth_type: "password-local".into(),
            title: "Password".into(),
            enabled: true,
            is_builtin: true,
            options: serde_json::json!({}),
        })
        .await
        .unwrap();
    let actor = store
        .upsert_root_user(
            workspace.id,
            "root",
            "root@example.com",
            "$argon2id$v=19$m=19456,t=2,p=1$test$test",
            "Root",
            "Root",
        )
        .await
        .unwrap();

    (store, workspace, actor)
}

#[tokio::test]
async fn mcp_management_seeds_default_instance_without_overwriting_user_changes() {
    let (store, workspace, actor) = seed_store().await;
    let service = McpManagementService::new(store);

    let first = service
        .ensure_default_workspace_catalog(actor.id)
        .await
        .unwrap();
    assert_eq!(
        first.default_instance.unwrap().instance_id,
        "default_system"
    );

    let renamed = service
        .update_instance(CreateMcpInstanceCommand {
            actor_user_id: actor.id,
            instance_id: "default_system".into(),
            name: "Renamed Default".into(),
            description_short: Some("user edited".into()),
            status: domain::McpInstanceStatus::Enabled,
            default_entry_path: "/".into(),
            is_default: true,
        })
        .await
        .unwrap();
    assert_eq!(renamed.name, "Renamed Default");

    let second = service
        .ensure_default_workspace_catalog(actor.id)
        .await
        .unwrap();
    assert_eq!(second.default_instance.unwrap().name, "Renamed Default");
    assert_eq!(second.instances.len(), 1);
    assert_eq!(second.meta_tool_config.workspace_id, workspace.id);
}

#[tokio::test]
async fn mcp_management_refreshes_des_id_and_exports_configuration_only() {
    let (store, _workspace, actor) = seed_store().await;
    let service = McpManagementService::new(store);
    service
        .ensure_default_workspace_catalog(actor.id)
        .await
        .unwrap();

    let instance = service
        .create_instance(CreateMcpInstanceCommand {
            actor_user_id: actor.id,
            instance_id: "ops".into(),
            name: "Operations".into(),
            description_short: Some("Operations tools".into()),
            status: domain::McpInstanceStatus::Enabled,
            default_entry_path: "/".into(),
            is_default: false,
        })
        .await
        .unwrap();

    let tool = service
        .create_tool(CreateMcpToolCommand {
            actor_user_id: actor.id,
            tool_id: None,
            suggested_group_path: Some("/ops".into()),
            name: "Restart Worker".into(),
            short_description: "Restart a worker".into(),
            usage_description: Some("Use only after checking status.".into()),
            full_description: "Restarts a selected worker through the backend interface.".into(),
            interface_id: "settings.system_runtime.get_profile".into(),
            parameter_schema: serde_json::json!({"type":"object"}),
            result_schema: serde_json::json!({"type":"object"}),
            input_mapping: serde_json::json!({}),
            output_mapping: serde_json::json!({}),
            permission_code: Some("system_runtime.view.all".into()),
            risk_level: domain::McpRiskLevel::High,
            audit_policy: serde_json::json!({"enabled": true}),
            des_id_required: true,
            status: domain::McpToolStatus::Enabled,
        })
        .await
        .unwrap();

    assert_eq!(tool.des_id.len(), 8);
    assert!(
        service
            .description_check(actor.id, &tool.tool_id, Some(&tool.des_id))
            .await
            .unwrap()
            .accepted
    );

    let refreshed = service
        .refresh_tool_description(RefreshMcpToolDescriptionCommand {
            actor_user_id: actor.id,
            tool_id: tool.tool_id.clone(),
        })
        .await
        .unwrap();
    assert_ne!(refreshed.des_id, tool.des_id);
    assert!(
        !service
            .description_check(actor.id, &tool.tool_id, Some(&tool.des_id))
            .await
            .unwrap()
            .accepted
    );

    service
        .upsert_group(UpsertMcpGroupCommand {
            actor_user_id: actor.id,
            instance_id: instance.instance_id.clone(),
            path: "/ops".into(),
            display_name: "Operations".into(),
            description_short: Some("Operational tools".into()),
            enabled: true,
            sort_order: 10,
        })
        .await
        .unwrap();

    service
        .create_tool_binding(CreateMcpToolBindingCommand {
            actor_user_id: actor.id,
            instance_id: instance.instance_id,
            group_path: "/ops".into(),
            tool_id: tool.tool_id.clone(),
            display_alias: Some("Restart worker".into()),
            visible: true,
            sort_order: 10,
        })
        .await
        .unwrap();

    let export = service.export_workspace_catalog(actor.id).await.unwrap();
    assert_eq!(export.tools.len(), 1);
    assert_eq!(export.instances.len(), 2);
    assert_eq!(export.bindings.len(), 1);
    assert_eq!(export.groups.len(), 1);

    service.delete_tool(actor.id, &tool.tool_id).await.unwrap();
    let missing = service
        .description_check(actor.id, &tool.tool_id, Some(&refreshed.des_id))
        .await;
    assert!(missing.is_err());
}

#[tokio::test]
async fn mcp_instance_directory_rules_cover_visibility_and_directory_export() {
    let (store, _workspace, actor) = seed_store().await;
    let service = McpManagementService::new(store);
    service
        .ensure_default_workspace_catalog(actor.id)
        .await
        .unwrap();

    let tool = service
        .create_tool(CreateMcpToolCommand {
            actor_user_id: actor.id,
            tool_id: Some("runtime_profile".into()),
            suggested_group_path: Some("/ops".into()),
            name: "Runtime Profile".into(),
            short_description: "Read runtime profile".into(),
            usage_description: None,
            full_description: "Read the current runtime profile.".into(),
            interface_id: "settings.system_runtime.get_profile".into(),
            parameter_schema: serde_json::json!({}),
            result_schema: serde_json::json!({}),
            input_mapping: serde_json::json!({}),
            output_mapping: serde_json::json!({}),
            permission_code: Some("system_runtime.view.all".into()),
            risk_level: domain::McpRiskLevel::High,
            audit_policy: serde_json::json!({}),
            des_id_required: true,
            status: domain::McpToolStatus::Enabled,
        })
        .await
        .unwrap();

    service
        .upsert_group(UpsertMcpGroupCommand {
            actor_user_id: actor.id,
            instance_id: "default_system".into(),
            path: "/ops".into(),
            display_name: "Operations".into(),
            description_short: None,
            enabled: true,
            sort_order: 1,
        })
        .await
        .unwrap();
    service
        .create_tool_binding(CreateMcpToolBindingCommand {
            actor_user_id: actor.id,
            instance_id: "default_system".into(),
            group_path: "/ops".into(),
            tool_id: tool.tool_id.clone(),
            display_alias: None,
            visible: true,
            sort_order: 1,
        })
        .await
        .unwrap();
    service
        .create_tool_binding(CreateMcpToolBindingCommand {
            actor_user_id: actor.id,
            instance_id: "default_system".into(),
            group_path: "/admin".into(),
            tool_id: tool.tool_id.clone(),
            display_alias: Some("Admin Runtime".into()),
            visible: true,
            sort_order: 2,
        })
        .await
        .unwrap();

    let disabled_instance = service
        .create_instance(CreateMcpInstanceCommand {
            actor_user_id: actor.id,
            instance_id: "disabled_ops".into(),
            name: "Disabled Ops".into(),
            description_short: None,
            status: domain::McpInstanceStatus::Disabled,
            default_entry_path: "/".into(),
            is_default: false,
        })
        .await
        .unwrap();
    service
        .create_tool_binding(CreateMcpToolBindingCommand {
            actor_user_id: actor.id,
            instance_id: disabled_instance.instance_id.clone(),
            group_path: "/ops".into(),
            tool_id: tool.tool_id.clone(),
            display_alias: None,
            visible: true,
            sort_order: 1,
        })
        .await
        .unwrap();

    let root_items = service
        .list_items(actor.id, None, Some("/"), None)
        .await
        .unwrap();
    assert!(root_items
        .iter()
        .any(|item| item.item_kind == domain::McpListItemKind::Group && item.path == "/ops"));
    assert_eq!(
        root_items
            .iter()
            .filter(|item| item.item_kind == domain::McpListItemKind::Tool)
            .count(),
        2
    );

    let ops_items = service
        .list_items(actor.id, None, Some("/ops"), None)
        .await
        .unwrap();
    assert!(ops_items
        .iter()
        .all(|item| item.path == "/ops" || item.path.starts_with("/ops/")));
    assert!(service
        .list_items(actor.id, Some(&disabled_instance.instance_id), None, None)
        .await
        .is_err());

    let directory_export = service.export_instance_directory(actor.id).await.unwrap();
    assert_eq!(directory_export.instances.len(), 2);
    assert_eq!(directory_export.bindings.len(), 3);
    assert_eq!(directory_export.groups.len(), 1);

    let full_export = service.export_workspace_catalog(actor.id).await.unwrap();
    assert_eq!(full_export.tools.len(), 1);

    service
        .delete_group(actor.id, "default_system", "/ops")
        .await
        .unwrap();
    let after_group_delete = service.read_workspace_catalog(actor.id).await.unwrap();
    assert!(!after_group_delete
        .groups
        .iter()
        .any(|group| group.path == "/ops"));

    service
        .delete_instance(actor.id, &disabled_instance.instance_id)
        .await
        .unwrap();
    let after_instance_delete = service.read_workspace_catalog(actor.id).await.unwrap();
    assert!(!after_instance_delete
        .instances
        .iter()
        .any(|instance| instance.instance_id == disabled_instance.instance_id));
    assert!(!after_instance_delete
        .bindings
        .iter()
        .any(|binding| binding.instance_record_id == disabled_instance.id));

    service
        .update_instance(CreateMcpInstanceCommand {
            actor_user_id: actor.id,
            instance_id: "default_system".into(),
            name: "Default System".into(),
            description_short: None,
            status: domain::McpInstanceStatus::Enabled,
            default_entry_path: "/".into(),
            is_default: false,
        })
        .await
        .unwrap();
    assert!(service
        .list_items(actor.id, None, None, None)
        .await
        .is_err());
}
