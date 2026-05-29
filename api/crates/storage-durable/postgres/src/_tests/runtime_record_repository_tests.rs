use control_plane::ports::{
    AddModelFieldInput, CreateModelDefinitionInput, ModelDefinitionRepository,
};
use domain::{DataModelScopeKind, ModelFieldKind, DEFAULT_SCOPE_ID};
use runtime_core::runtime_engine::RuntimeModelError;
use runtime_core::runtime_record_repository::{
    RuntimeListQuery, RuntimeRecordRepository, RuntimeSortInput,
};
use serde_json::json;
use sqlx::PgPool;
use storage_postgres::{connect, run_migrations, PgControlPlaneStore};
use time::macros::datetime;
use uuid::Uuid;

fn base_database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:1flowbase@127.0.0.1:35432/1flowbase".into())
}

async fn isolated_database_url() -> String {
    let admin_pool = PgPool::connect(&base_database_url()).await.unwrap();
    let schema = format!("test_{}", Uuid::now_v7().to_string().replace('-', ""));
    sqlx::query(&format!("create schema if not exists {schema}"))
        .execute(&admin_pool)
        .await
        .unwrap();

    format!("{}?options=-csearch_path%3D{schema}", base_database_url())
}

async fn root_tenant_id(store: &PgControlPlaneStore) -> Uuid {
    sqlx::query_scalar("select id from tenants where code = 'root-tenant'")
        .fetch_one(store.pool())
        .await
        .unwrap()
}

async fn insert_user(store: &PgControlPlaneStore, user_id: Uuid, account: &str) {
    let unique_account = format!("{account}-{}", user_id.simple());
    sqlx::query(
        r#"
        insert into users (
            id, account, email, phone, password_hash, name, nickname, avatar_url, introduction,
            default_display_role, email_login_enabled, phone_login_enabled, status, session_version,
            created_by, updated_by
        )
        values (
            $1, $2, $3, null, 'hash', $4, $5, null, '', 'manager', true, false, 'active', 1, null, null
        )
        "#,
    )
    .bind(user_id)
    .bind(&unique_account)
    .bind(format!("{unique_account}@example.com"))
    .bind(&unique_account)
    .bind(&unique_account)
    .execute(store.pool())
    .await
    .unwrap();
}

async fn insert_workspace(store: &PgControlPlaneStore, workspace_id: Uuid) {
    let tenant_id = root_tenant_id(store).await;
    let workspace_name = format!("Core Workspace {}", workspace_id.simple());
    sqlx::query(
        "insert into workspaces (id, tenant_id, name, created_by, updated_by) values ($1, $2, $3, null, null)",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(&workspace_name)
    .execute(store.pool())
    .await
    .unwrap();
}

struct RuntimeReadModelSeed {
    workspace_id: Uuid,
    application_id: Uuid,
    flow_run_id: Uuid,
    node_run_id: Uuid,
}

async fn seed_runtime_read_model_rows(store: &PgControlPlaneStore) -> RuntimeReadModelSeed {
    let workspace_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let application_id = Uuid::now_v7();
    let flow_id = Uuid::now_v7();
    let draft_id = Uuid::now_v7();
    let compiled_plan_id = Uuid::now_v7();
    let flow_run_id = Uuid::now_v7();
    let node_run_id = Uuid::now_v7();
    let started_at = datetime!(2026-05-29 08:00:00 UTC);

    insert_workspace(store, workspace_id).await;
    insert_user(store, user_id, "runtime-read-model").await;

    sqlx::query(
        r#"
        insert into applications (
            id, workspace_id, application_type, name, description, created_by, updated_by
        ) values ($1, $2, 'agent_flow', 'Runtime Read Model App', '', $3, $3)
        "#,
    )
    .bind(application_id)
    .bind(workspace_id)
    .bind(user_id)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        "insert into flows (id, application_id, created_by, updated_by) values ($1, $2, $3, $3)",
    )
    .bind(flow_id)
    .bind(application_id)
    .bind(user_id)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into flow_drafts (id, flow_id, schema_version, document, updated_by)
        values ($1, $2, '1flowbase.flow/v2', '{}', $3)
        "#,
    )
    .bind(draft_id)
    .bind(flow_id)
    .bind(user_id)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into flow_compiled_plans (
            id, flow_id, flow_draft_id, schema_version, document_hash,
            document_updated_at, plan, created_by
        ) values ($1, $2, $3, '1flowbase.flow/v2', 'hash', $4, '{}', $5)
        "#,
    )
    .bind(compiled_plan_id)
    .bind(flow_id)
    .bind(draft_id)
    .bind(started_at)
    .bind(user_id)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into flow_runs (
            id, application_id, flow_id, flow_draft_id, compiled_plan_id,
            debug_session_id, flow_schema_version, document_hash, run_mode,
            title, status, input_payload, output_payload, created_by,
            started_at, finished_at, created_at, updated_at
        ) values (
            $1, $2, $3, $4, $5, 'runtime-read-model', '1flowbase.flow/v2',
            'hash', 'debug_flow_run', 'Alpha refund run', 'succeeded',
            '{"query":"refund"}', '{"answer":"done"}', $6, $7, $8, $7, $8
        )
        "#,
    )
    .bind(flow_run_id)
    .bind(application_id)
    .bind(flow_id)
    .bind(draft_id)
    .bind(compiled_plan_id)
    .bind(user_id)
    .bind(started_at)
    .bind(started_at + time::Duration::seconds(2))
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into application_run_log_summaries (
            flow_run_id, scope_id, application_id, run_mode, status, title,
            input_payload, unique_node_count, tool_callback_count,
            started_at, finished_at, created_at, updated_at
        ) values (
            $1, $2, $3, 'debug_flow_run', 'succeeded', 'Alpha refund run',
            '{}', 1, 1, $4, $5, $4, $5
        )
        "#,
    )
    .bind(flow_run_id)
    .bind(workspace_id)
    .bind(application_id)
    .bind(started_at)
    .bind(started_at + time::Duration::seconds(2))
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into node_runs (
            id, scope_id, flow_run_id, node_id, node_type, node_alias, status,
            input_payload, output_payload, metrics_payload, debug_payload, started_at, created_at
        ) values (
            $1, $2, $3, 'node-llm', 'llm', 'LLM', 'succeeded',
            '{"large":"input"}', '{"large":"output"}', '{"tokens":12}',
            '{"large":"debug"}', $4, $4
        )
        "#,
    )
    .bind(node_run_id)
    .bind(workspace_id)
    .bind(flow_run_id)
    .bind(started_at)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into flow_run_events (
            id, scope_id, flow_run_id, node_run_id, sequence, event_type, payload, created_at
        ) values (
            $1, $2, $3, $4, 1, 'node_run_completed', '{"large":"event"}', $5
        )
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(workspace_id)
    .bind(flow_run_id)
    .bind(node_run_id)
    .bind(started_at)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into flow_run_checkpoints (
            id, scope_id, flow_run_id, node_run_id, status, reason,
            locator_payload, variable_snapshot, external_ref_payload, created_at
        ) values (
            $1, $2, $3, $4, 'waiting_human', 'review',
            '{"large":"locator"}', '{"large":"variables"}', '{"large":"external"}', $5
        )
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(workspace_id)
    .bind(flow_run_id)
    .bind(node_run_id)
    .bind(started_at)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into flow_run_callback_tasks (
            id, scope_id, flow_run_id, node_run_id, callback_kind, status,
            request_payload, external_ref_payload, created_at
        ) values (
            $1, $2, $3, $4, 'tool', 'pending',
            '{"large":"request"}', '{"large":"external"}', $5
        )
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(workspace_id)
    .bind(flow_run_id)
    .bind(node_run_id)
    .bind(started_at)
    .execute(store.pool())
    .await
    .unwrap();

    RuntimeReadModelSeed {
        workspace_id,
        application_id,
        flow_run_id,
        node_run_id,
    }
}

#[tokio::test]
async fn runtime_record_repository_scopes_dynamic_rows_without_workspace_row() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let default_scope_id = domain::DEFAULT_SCOPE_ID;
    let alternate_scope_id = Uuid::now_v7();
    assert_ne!(default_scope_id, domain::SYSTEM_SCOPE_ID);

    let workspace_count: i64 = sqlx::query_scalar("select count(*) from workspaces")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(workspace_count, 0);

    let model = ModelDefinitionRepository::create_model_definition(
        &store,
        &CreateModelDefinitionInput {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::System,
            scope_id: domain::SYSTEM_SCOPE_ID,
            data_source_instance_id: None,
            source_kind: domain::DataModelSourceKind::MainSource,
            external_resource_key: None,
            external_table_id: None,
            external_capability_snapshot: None,
            status: domain::DataModelStatus::Published,
            api_exposure_status: domain::ApiExposureStatus::PublishedNotExposed,
            protection: domain::DataModelProtection::default(),
            code: "default_scope_orders".into(),
            title: "Default Scope Orders".into(),
        },
    )
    .await
    .unwrap();
    ModelDefinitionRepository::add_model_field(
        &store,
        &AddModelFieldInput {
            actor_user_id: Uuid::nil(),
            model_id: model.id,
            physical_column_name: None,
            external_field_key: None,
            code: "title".into(),
            title: "Title".into(),
            field_kind: ModelFieldKind::String,
            is_system: false,
            is_writable: true,
            apply_physical_schema: true,
            is_required: true,
            is_unique: false,
            default_value: None,
            display_interface: Some("input".into()),
            display_options: json!({}),
            relation_target_model_id: None,
            relation_options: json!({}),
        },
    )
    .await
    .unwrap();

    let metadata = store
        .list_runtime_model_metadata()
        .await
        .unwrap()
        .into_iter()
        .find(|model| model.model_code == "default_scope_orders")
        .unwrap();
    assert_eq!(metadata.scope_kind, DataModelScopeKind::System);
    assert_eq!(metadata.scope_id, domain::SYSTEM_SCOPE_ID);
    assert_eq!(metadata.scope_column_name, "scope_id");

    let default_record = RuntimeRecordRepository::create_record(
        &store,
        &metadata,
        Uuid::nil(),
        default_scope_id,
        json!({ "title": "default-scope" }),
    )
    .await
    .unwrap();
    let alternate_record = RuntimeRecordRepository::create_record(
        &store,
        &metadata,
        Uuid::nil(),
        alternate_scope_id,
        json!({ "title": "alternate-scope" }),
    )
    .await
    .unwrap();
    let default_record_id = default_record["id"].as_str().unwrap().to_string();
    let alternate_record_id = alternate_record["id"].as_str().unwrap().to_string();

    let default_list = RuntimeRecordRepository::list_records(
        &store,
        &metadata,
        RuntimeListQuery {
            scope_id: Some(default_scope_id),
            owner_user_id: None,
            filter: domain::ResourceFilterExpr::All(vec![]),
            sorts: vec![],
            expand_relations: vec![],
            page: 1,
            page_size: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(default_list.total, 1);
    assert_eq!(default_list.items[0]["title"], json!("default-scope"));

    let alternate_list = RuntimeRecordRepository::list_records(
        &store,
        &metadata,
        RuntimeListQuery {
            scope_id: Some(alternate_scope_id),
            owner_user_id: None,
            filter: domain::ResourceFilterExpr::All(vec![]),
            sorts: vec![],
            expand_relations: vec![],
            page: 1,
            page_size: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(alternate_list.total, 1);
    assert_eq!(alternate_list.items[0]["title"], json!("alternate-scope"));

    let cross_scope_get = RuntimeRecordRepository::get_record(
        &store,
        &metadata,
        Some(alternate_scope_id),
        None,
        &default_record_id,
    )
    .await
    .unwrap();
    assert!(cross_scope_get.is_none());

    let cross_scope_update = RuntimeRecordRepository::update_record(
        &store,
        &metadata,
        Uuid::nil(),
        Some(alternate_scope_id),
        None,
        &default_record_id,
        json!({ "title": "blocked" }),
    )
    .await;
    assert!(cross_scope_update.is_err());

    let cross_scope_delete = RuntimeRecordRepository::delete_record(
        &store,
        &metadata,
        Some(alternate_scope_id),
        None,
        &default_record_id,
    )
    .await
    .unwrap();
    assert!(!cross_scope_delete);

    let updated = RuntimeRecordRepository::update_record(
        &store,
        &metadata,
        Uuid::nil(),
        Some(default_scope_id),
        None,
        &default_record_id,
        json!({ "title": "default-scope-updated" }),
    )
    .await
    .unwrap();
    assert_eq!(updated["title"], json!("default-scope-updated"));

    let alternate_deleted = RuntimeRecordRepository::delete_record(
        &store,
        &metadata,
        Some(alternate_scope_id),
        None,
        &alternate_record_id,
    )
    .await
    .unwrap();
    assert!(alternate_deleted);
}

#[tokio::test]
async fn runtime_record_repository_uses_default_scope_id_without_workspace_row() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let future_scope_id = Uuid::now_v7();

    let missing_workspace_count: i64 =
        sqlx::query_scalar("select count(*)::bigint from workspaces where id = any($1)")
            .bind(vec![DEFAULT_SCOPE_ID, future_scope_id])
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(missing_workspace_count, 0);

    let model = ModelDefinitionRepository::create_model_definition(
        &store,
        &CreateModelDefinitionInput {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::Workspace,
            scope_id: DEFAULT_SCOPE_ID,
            data_source_instance_id: None,
            source_kind: domain::DataModelSourceKind::MainSource,
            external_resource_key: None,
            external_table_id: None,
            external_capability_snapshot: None,
            status: domain::DataModelStatus::Published,
            api_exposure_status: domain::ApiExposureStatus::PublishedNotExposed,
            protection: domain::DataModelProtection::default(),
            code: "default_scope_orders".into(),
            title: "Default Scope Orders".into(),
        },
    )
    .await
    .unwrap();
    ModelDefinitionRepository::add_model_field(
        &store,
        &AddModelFieldInput {
            actor_user_id: Uuid::nil(),
            model_id: model.id,
            physical_column_name: None,
            external_field_key: None,
            code: "title".into(),
            title: "Title".into(),
            field_kind: ModelFieldKind::String,
            is_system: false,
            is_writable: true,
            apply_physical_schema: true,
            is_required: true,
            is_unique: false,
            default_value: None,
            display_interface: Some("input".into()),
            display_options: json!({}),
            relation_target_model_id: None,
            relation_options: json!({}),
        },
    )
    .await
    .unwrap();

    let metadata = store
        .list_runtime_model_metadata()
        .await
        .unwrap()
        .into_iter()
        .find(|model| {
            model.model_code == "default_scope_orders" && model.scope_id == DEFAULT_SCOPE_ID
        })
        .unwrap();
    assert_eq!(metadata.scope_column_name, "scope_id");

    let columns: Vec<String> = sqlx::query_scalar(
        r#"
        select column_name
        from information_schema.columns
        where table_schema = current_schema()
          and table_name = $1
        order by ordinal_position
        "#,
    )
    .bind(&metadata.physical_table_name)
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert!(columns.contains(&"scope_id".to_string()));
    assert!(!columns.contains(&"workspace_id".to_string()));

    let default_record = RuntimeRecordRepository::create_record(
        &store,
        &metadata,
        Uuid::nil(),
        DEFAULT_SCOPE_ID,
        json!({ "title": "default-scope" }),
    )
    .await
    .unwrap();
    let default_record_id = default_record["id"].as_str().unwrap().to_string();
    RuntimeRecordRepository::create_record(
        &store,
        &metadata,
        Uuid::nil(),
        future_scope_id,
        json!({ "title": "future-provider-scope" }),
    )
    .await
    .unwrap();

    let default_list = RuntimeRecordRepository::list_records(
        &store,
        &metadata,
        RuntimeListQuery {
            scope_id: Some(DEFAULT_SCOPE_ID),
            owner_user_id: None,
            filter: domain::ResourceFilterExpr::All(vec![]),
            sorts: vec![],
            expand_relations: vec![],
            page: 1,
            page_size: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(default_list.total, 1);
    assert_eq!(default_list.items[0]["title"], json!("default-scope"));

    let future_scope_list = RuntimeRecordRepository::list_records(
        &store,
        &metadata,
        RuntimeListQuery {
            scope_id: Some(future_scope_id),
            owner_user_id: None,
            filter: domain::ResourceFilterExpr::All(vec![]),
            sorts: vec![],
            expand_relations: vec![],
            page: 1,
            page_size: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(future_scope_list.total, 1);
    assert_eq!(
        future_scope_list.items[0]["title"],
        json!("future-provider-scope")
    );

    let blocked_cross_scope_get = RuntimeRecordRepository::get_record(
        &store,
        &metadata,
        Some(future_scope_id),
        None,
        &default_record_id,
    )
    .await
    .unwrap();
    assert!(blocked_cross_scope_get.is_none());

    let updated = RuntimeRecordRepository::update_record(
        &store,
        &metadata,
        Uuid::nil(),
        Some(DEFAULT_SCOPE_ID),
        None,
        &default_record_id,
        json!({ "title": "default-scope-updated" }),
    )
    .await
    .unwrap();
    assert_eq!(updated["title"], json!("default-scope-updated"));

    let deleted = RuntimeRecordRepository::delete_record(
        &store,
        &metadata,
        Some(DEFAULT_SCOPE_ID),
        None,
        &default_record_id,
    )
    .await
    .unwrap();
    assert!(deleted);
}

#[tokio::test]
async fn runtime_record_repository_supports_crud_filter_sort_and_relation_expansion() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let workspace_id = Uuid::now_v7();
    let tenant_id = root_tenant_id(&store).await;
    let workspace_name = format!("Core Workspace {}", workspace_id.simple());
    sqlx::query(
        "insert into workspaces (id, tenant_id, name, created_by, updated_by) values ($1, $2, $3, null, null)",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(&workspace_name)
    .execute(store.pool())
    .await
    .unwrap();

    let customer_model = ModelDefinitionRepository::create_model_definition(
        &store,
        &CreateModelDefinitionInput {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::Workspace,
            scope_id: workspace_id,
            data_source_instance_id: None,
            source_kind: domain::DataModelSourceKind::MainSource,
            external_resource_key: None,
            external_table_id: None,
            external_capability_snapshot: None,
            status: domain::DataModelStatus::Published,
            api_exposure_status: domain::ApiExposureStatus::PublishedNotExposed,
            protection: domain::DataModelProtection::default(),
            code: "customers".into(),
            title: "Customers".into(),
        },
    )
    .await
    .unwrap();
    let order_model = ModelDefinitionRepository::create_model_definition(
        &store,
        &CreateModelDefinitionInput {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::Workspace,
            scope_id: workspace_id,
            data_source_instance_id: None,
            source_kind: domain::DataModelSourceKind::MainSource,
            external_resource_key: None,
            external_table_id: None,
            external_capability_snapshot: None,
            status: domain::DataModelStatus::Published,
            api_exposure_status: domain::ApiExposureStatus::PublishedNotExposed,
            protection: domain::DataModelProtection::default(),
            code: "orders".into(),
            title: "Orders".into(),
        },
    )
    .await
    .unwrap();

    ModelDefinitionRepository::add_model_field(
        &store,
        &AddModelFieldInput {
            actor_user_id: Uuid::nil(),
            model_id: customer_model.id,
            physical_column_name: None,
            external_field_key: None,
            code: "name".into(),
            title: "Name".into(),
            field_kind: ModelFieldKind::String,
            is_system: false,
            is_writable: true,
            apply_physical_schema: true,
            is_required: true,
            is_unique: false,
            default_value: None,
            display_interface: Some("input".into()),
            display_options: json!({}),
            relation_target_model_id: None,
            relation_options: json!({}),
        },
    )
    .await
    .unwrap();
    ModelDefinitionRepository::add_model_field(
        &store,
        &AddModelFieldInput {
            actor_user_id: Uuid::nil(),
            model_id: order_model.id,
            physical_column_name: None,
            external_field_key: None,
            code: "title".into(),
            title: "Title".into(),
            field_kind: ModelFieldKind::String,
            is_system: false,
            is_writable: true,
            apply_physical_schema: true,
            is_required: true,
            is_unique: false,
            default_value: None,
            display_interface: Some("input".into()),
            display_options: json!({}),
            relation_target_model_id: None,
            relation_options: json!({}),
        },
    )
    .await
    .unwrap();
    ModelDefinitionRepository::add_model_field(
        &store,
        &AddModelFieldInput {
            actor_user_id: Uuid::nil(),
            model_id: order_model.id,
            physical_column_name: None,
            external_field_key: None,
            code: "status".into(),
            title: "Status".into(),
            field_kind: ModelFieldKind::Enum,
            is_system: false,
            is_writable: true,
            apply_physical_schema: true,
            is_required: true,
            is_unique: false,
            default_value: None,
            display_interface: Some("select".into()),
            display_options: json!({ "options": ["draft", "paid"] }),
            relation_target_model_id: None,
            relation_options: json!({}),
        },
    )
    .await
    .unwrap();
    ModelDefinitionRepository::add_model_field(
        &store,
        &AddModelFieldInput {
            actor_user_id: Uuid::nil(),
            model_id: order_model.id,
            physical_column_name: None,
            external_field_key: None,
            code: "customer".into(),
            title: "Customer".into(),
            field_kind: ModelFieldKind::ManyToOne,
            is_system: false,
            is_writable: true,
            apply_physical_schema: true,
            is_required: true,
            is_unique: false,
            default_value: None,
            display_interface: Some("select".into()),
            display_options: json!({}),
            relation_target_model_id: Some(customer_model.id),
            relation_options: json!({}),
        },
    )
    .await
    .unwrap();
    ModelDefinitionRepository::add_model_field(
        &store,
        &AddModelFieldInput {
            actor_user_id: Uuid::nil(),
            model_id: customer_model.id,
            physical_column_name: None,
            external_field_key: None,
            code: "orders".into(),
            title: "Orders".into(),
            field_kind: ModelFieldKind::OneToMany,
            is_system: false,
            is_writable: true,
            apply_physical_schema: true,
            is_required: false,
            is_unique: false,
            default_value: None,
            display_interface: None,
            display_options: json!({}),
            relation_target_model_id: Some(order_model.id),
            relation_options: json!({ "mapped_by": "customer" }),
        },
    )
    .await
    .unwrap();

    let mut metadata = store.list_runtime_model_metadata().await.unwrap();
    metadata.sort_by(|left, right| left.model_code.cmp(&right.model_code));
    let customer_metadata = metadata
        .iter()
        .find(|model| model.model_code == "customers" && model.scope_id == workspace_id)
        .unwrap()
        .clone();
    let order_metadata = metadata
        .iter()
        .find(|model| model.model_code == "orders" && model.scope_id == workspace_id)
        .unwrap()
        .clone();
    assert_eq!(customer_metadata.scope_column_name, "scope_id");
    assert_eq!(order_metadata.scope_column_name, "scope_id");

    let alice = RuntimeRecordRepository::create_record(
        &store,
        &customer_metadata,
        Uuid::nil(),
        workspace_id,
        json!({ "name": "Alice" }),
    )
    .await
    .unwrap();
    let bob = RuntimeRecordRepository::create_record(
        &store,
        &customer_metadata,
        Uuid::nil(),
        workspace_id,
        json!({ "name": "Bob" }),
    )
    .await
    .unwrap();

    let alice_id = alice["id"].as_str().unwrap().to_string();
    let bob_id = bob["id"].as_str().unwrap().to_string();

    let first = RuntimeRecordRepository::create_record(
        &store,
        &order_metadata,
        Uuid::nil(),
        workspace_id,
        json!({ "title": "A-001", "status": "draft", "customer": alice_id }),
    )
    .await
    .unwrap();
    let created = RuntimeRecordRepository::create_record(
        &store,
        &order_metadata,
        Uuid::nil(),
        workspace_id,
        json!({ "title": "A-002", "status": "paid", "customer": bob_id.clone() }),
    )
    .await
    .unwrap();

    let first_id = first["id"].as_str().unwrap().to_string();
    let order_id = created["id"].as_str().unwrap().to_string();

    let listed = RuntimeRecordRepository::list_records(
        &store,
        &order_metadata,
        RuntimeListQuery {
            scope_id: Some(workspace_id),
            owner_user_id: None,
            filter: domain::ResourceFilterExpr::Field {
                field: "status".into(),
                operator: domain::ResourceFilterOperator::Eq,
                value: json!("paid"),
            },
            sorts: vec![RuntimeSortInput {
                field_code: "title".into(),
                direction: "desc".into(),
            }],
            expand_relations: vec!["customer".into()],
            page: 1,
            page_size: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(listed.total, 1);
    assert_eq!(listed.items[0]["title"], json!("A-002"));
    assert_eq!(listed.items[0]["customer"]["name"], json!("Bob"));

    let filtered_by_ast = RuntimeRecordRepository::list_records(
        &store,
        &order_metadata,
        RuntimeListQuery {
            scope_id: Some(workspace_id),
            owner_user_id: None,
            filter: domain::ResourceFilterExpr::All(vec![
                domain::ResourceFilterExpr::Field {
                    field: "title".into(),
                    operator: domain::ResourceFilterOperator::Includes,
                    value: json!("A-00"),
                },
                domain::ResourceFilterExpr::Field {
                    field: "status".into(),
                    operator: domain::ResourceFilterOperator::In,
                    value: json!(["paid"]),
                },
            ]),
            sorts: vec![],
            expand_relations: vec![],
            page: 1,
            page_size: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(filtered_by_ast.total, 1);
    assert_eq!(filtered_by_ast.items[0]["title"], json!("A-002"));

    let fetched = RuntimeRecordRepository::get_record(
        &store,
        &order_metadata,
        Some(workspace_id),
        None,
        &first_id,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(fetched["title"], json!("A-001"));

    let updated = RuntimeRecordRepository::update_record(
        &store,
        &order_metadata,
        Uuid::nil(),
        Some(workspace_id),
        None,
        &order_id,
        json!({ "title": "A-002X", "status": "paid", "customer": bob_id }),
    )
    .await
    .unwrap();
    assert_eq!(updated["title"], json!("A-002X"));

    let customers = RuntimeRecordRepository::list_records(
        &store,
        &customer_metadata,
        RuntimeListQuery {
            scope_id: Some(workspace_id),
            owner_user_id: None,
            filter: domain::ResourceFilterExpr::All(vec![]),
            sorts: vec![],
            expand_relations: vec!["orders".into()],
            page: 1,
            page_size: 20,
        },
    )
    .await
    .unwrap();
    let alice_row = customers
        .items
        .iter()
        .find(|item| item["name"] == json!("Alice"))
        .unwrap();
    assert_eq!(alice_row["orders"].as_array().unwrap().len(), 1);

    let deleted = RuntimeRecordRepository::delete_record(
        &store,
        &order_metadata,
        Some(workspace_id),
        None,
        &order_id,
    )
    .await
    .unwrap();
    assert!(deleted);
}

#[tokio::test]
async fn runtime_record_repository_blocks_expanding_draft_relation_targets() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let workspace_id = Uuid::now_v7();
    insert_workspace(&store, workspace_id).await;

    let customer_model = ModelDefinitionRepository::create_model_definition(
        &store,
        &CreateModelDefinitionInput {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::Workspace,
            scope_id: workspace_id,
            data_source_instance_id: None,
            source_kind: domain::DataModelSourceKind::MainSource,
            external_resource_key: None,
            external_table_id: None,
            external_capability_snapshot: None,
            status: domain::DataModelStatus::Draft,
            api_exposure_status: domain::ApiExposureStatus::Draft,
            protection: domain::DataModelProtection::default(),
            code: "draft_relation_customers".into(),
            title: "Draft Relation Customers".into(),
        },
    )
    .await
    .unwrap();
    let order_model = ModelDefinitionRepository::create_model_definition(
        &store,
        &CreateModelDefinitionInput {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::Workspace,
            scope_id: workspace_id,
            data_source_instance_id: None,
            source_kind: domain::DataModelSourceKind::MainSource,
            external_resource_key: None,
            external_table_id: None,
            external_capability_snapshot: None,
            status: domain::DataModelStatus::Published,
            api_exposure_status: domain::ApiExposureStatus::PublishedNotExposed,
            protection: domain::DataModelProtection::default(),
            code: "draft_relation_orders".into(),
            title: "Draft Relation Orders".into(),
        },
    )
    .await
    .unwrap();

    ModelDefinitionRepository::add_model_field(
        &store,
        &AddModelFieldInput {
            actor_user_id: Uuid::nil(),
            model_id: customer_model.id,
            physical_column_name: None,
            external_field_key: None,
            code: "name".into(),
            title: "Name".into(),
            field_kind: ModelFieldKind::String,
            is_system: false,
            is_writable: true,
            apply_physical_schema: true,
            is_required: true,
            is_unique: false,
            default_value: None,
            display_interface: Some("input".into()),
            display_options: json!({}),
            relation_target_model_id: None,
            relation_options: json!({}),
        },
    )
    .await
    .unwrap();
    ModelDefinitionRepository::add_model_field(
        &store,
        &AddModelFieldInput {
            actor_user_id: Uuid::nil(),
            model_id: order_model.id,
            physical_column_name: None,
            external_field_key: None,
            code: "title".into(),
            title: "Title".into(),
            field_kind: ModelFieldKind::String,
            is_system: false,
            is_writable: true,
            apply_physical_schema: true,
            is_required: true,
            is_unique: false,
            default_value: None,
            display_interface: Some("input".into()),
            display_options: json!({}),
            relation_target_model_id: None,
            relation_options: json!({}),
        },
    )
    .await
    .unwrap();
    ModelDefinitionRepository::add_model_field(
        &store,
        &AddModelFieldInput {
            actor_user_id: Uuid::nil(),
            model_id: order_model.id,
            physical_column_name: None,
            external_field_key: None,
            code: "customer".into(),
            title: "Customer".into(),
            field_kind: ModelFieldKind::ManyToOne,
            is_system: false,
            is_writable: true,
            apply_physical_schema: true,
            is_required: true,
            is_unique: false,
            default_value: None,
            display_interface: Some("select".into()),
            display_options: json!({}),
            relation_target_model_id: Some(customer_model.id),
            relation_options: json!({}),
        },
    )
    .await
    .unwrap();

    let metadata = store.list_runtime_model_metadata().await.unwrap();
    let customer_metadata = metadata
        .iter()
        .find(|model| model.model_code == "draft_relation_customers")
        .unwrap()
        .clone();
    let order_metadata = metadata
        .iter()
        .find(|model| model.model_code == "draft_relation_orders")
        .unwrap()
        .clone();

    let customer = RuntimeRecordRepository::create_record(
        &store,
        &customer_metadata,
        Uuid::nil(),
        workspace_id,
        json!({ "name": "Draft Customer" }),
    )
    .await
    .unwrap();
    let customer_id = customer["id"].as_str().unwrap().to_string();
    RuntimeRecordRepository::create_record(
        &store,
        &order_metadata,
        Uuid::nil(),
        workspace_id,
        json!({ "title": "Published Order", "customer": customer_id }),
    )
    .await
    .unwrap();

    let error = RuntimeRecordRepository::list_records(
        &store,
        &order_metadata,
        RuntimeListQuery {
            scope_id: Some(workspace_id),
            owner_user_id: None,
            filter: domain::ResourceFilterExpr::All(vec![]),
            sorts: vec![],
            expand_relations: vec!["customer".into()],
            page: 1,
            page_size: 20,
        },
    )
    .await
    .unwrap_err();

    let model_error = error.downcast_ref::<RuntimeModelError>().unwrap();
    assert_eq!(
        model_error,
        &RuntimeModelError::not_published("draft_relation_customers")
    );
}

#[tokio::test]
async fn runtime_record_repository_enforces_owner_scope() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let workspace_id = Uuid::now_v7();
    let owner_user_id = Uuid::now_v7();
    let other_user_id = Uuid::now_v7();
    let tenant_id = root_tenant_id(&store).await;
    let workspace_name = format!("Core Workspace {}", workspace_id.simple());

    sqlx::query(
        "insert into workspaces (id, tenant_id, name, created_by, updated_by) values ($1, $2, $3, null, null)",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(&workspace_name)
    .execute(store.pool())
    .await
    .unwrap();
    insert_user(&store, owner_user_id, "owner-user").await;
    insert_user(&store, other_user_id, "other-user").await;

    let order_model = ModelDefinitionRepository::create_model_definition(
        &store,
        &CreateModelDefinitionInput {
            actor_user_id: Uuid::nil(),
            scope_kind: DataModelScopeKind::Workspace,
            scope_id: workspace_id,
            data_source_instance_id: None,
            source_kind: domain::DataModelSourceKind::MainSource,
            external_resource_key: None,
            external_table_id: None,
            external_capability_snapshot: None,
            status: domain::DataModelStatus::Published,
            api_exposure_status: domain::ApiExposureStatus::PublishedNotExposed,
            protection: domain::DataModelProtection::default(),
            code: "orders_acl".into(),
            title: "Orders ACL".into(),
        },
    )
    .await
    .unwrap();
    ModelDefinitionRepository::add_model_field(
        &store,
        &AddModelFieldInput {
            actor_user_id: Uuid::nil(),
            model_id: order_model.id,
            physical_column_name: None,
            external_field_key: None,
            code: "title".into(),
            title: "Title".into(),
            field_kind: ModelFieldKind::String,
            is_system: false,
            is_writable: true,
            apply_physical_schema: true,
            is_required: true,
            is_unique: false,
            default_value: None,
            display_interface: Some("input".into()),
            display_options: json!({}),
            relation_target_model_id: None,
            relation_options: json!({}),
        },
    )
    .await
    .unwrap();

    let metadata = store
        .list_runtime_model_metadata()
        .await
        .unwrap()
        .into_iter()
        .find(|model| model.model_code == "orders_acl" && model.scope_id == workspace_id)
        .unwrap();
    assert_eq!(metadata.scope_column_name, "scope_id");

    let owner_record = RuntimeRecordRepository::create_record(
        &store,
        &metadata,
        owner_user_id,
        workspace_id,
        json!({ "title": "owner-record" }),
    )
    .await
    .unwrap();
    let other_record = RuntimeRecordRepository::create_record(
        &store,
        &metadata,
        other_user_id,
        workspace_id,
        json!({ "title": "other-record" }),
    )
    .await
    .unwrap();
    let owner_record_id = owner_record["id"].as_str().unwrap().to_string();
    let other_record_id = other_record["id"].as_str().unwrap().to_string();

    let own_list = RuntimeRecordRepository::list_records(
        &store,
        &metadata,
        RuntimeListQuery {
            scope_id: Some(workspace_id),
            owner_user_id: Some(owner_user_id),
            filter: domain::ResourceFilterExpr::All(vec![]),
            sorts: vec![],
            expand_relations: vec![],
            page: 1,
            page_size: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(own_list.total, 1);
    assert_eq!(own_list.items[0]["title"], json!("owner-record"));

    let own_get = RuntimeRecordRepository::get_record(
        &store,
        &metadata,
        Some(workspace_id),
        Some(owner_user_id),
        &owner_record_id,
    )
    .await
    .unwrap();
    assert!(own_get.is_some());
    let blocked_get = RuntimeRecordRepository::get_record(
        &store,
        &metadata,
        Some(workspace_id),
        Some(owner_user_id),
        &other_record_id,
    )
    .await
    .unwrap();
    assert!(blocked_get.is_none());

    let updated = RuntimeRecordRepository::update_record(
        &store,
        &metadata,
        owner_user_id,
        Some(workspace_id),
        Some(owner_user_id),
        &owner_record_id,
        json!({ "title": "owner-record-updated" }),
    )
    .await
    .unwrap();
    assert_eq!(updated["title"], json!("owner-record-updated"));

    let blocked_update = RuntimeRecordRepository::update_record(
        &store,
        &metadata,
        owner_user_id,
        Some(workspace_id),
        Some(owner_user_id),
        &other_record_id,
        json!({ "title": "blocked-update" }),
    )
    .await;
    assert!(blocked_update.is_err());

    let blocked_delete = RuntimeRecordRepository::delete_record(
        &store,
        &metadata,
        Some(workspace_id),
        Some(owner_user_id),
        &other_record_id,
    )
    .await
    .unwrap();
    assert!(!blocked_delete);

    let all_list = RuntimeRecordRepository::list_records(
        &store,
        &metadata,
        RuntimeListQuery {
            scope_id: Some(workspace_id),
            owner_user_id: None,
            filter: domain::ResourceFilterExpr::All(vec![]),
            sorts: vec![],
            expand_relations: vec![],
            page: 1,
            page_size: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(all_list.total, 2);

    let all_get = RuntimeRecordRepository::get_record(
        &store,
        &metadata,
        Some(workspace_id),
        None,
        &other_record_id,
    )
    .await
    .unwrap();
    assert!(all_get.is_some());

    let all_updated = RuntimeRecordRepository::update_record(
        &store,
        &metadata,
        owner_user_id,
        Some(workspace_id),
        None,
        &other_record_id,
        json!({ "title": "other-record-updated" }),
    )
    .await
    .unwrap();
    assert_eq!(all_updated["title"], json!("other-record-updated"));

    let all_deleted = RuntimeRecordRepository::delete_record(
        &store,
        &metadata,
        Some(workspace_id),
        None,
        &other_record_id,
    )
    .await
    .unwrap();
    assert!(all_deleted);
}

#[tokio::test]
async fn runtime_record_repository_registers_builtin_runtime_read_models() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);

    let metadata = store.list_runtime_model_metadata().await.unwrap();
    let model_codes = metadata
        .iter()
        .map(|model| model.model_code.as_str())
        .collect::<Vec<_>>();
    for expected in [
        "application_run_log_summaries",
        "application_conversations",
        "application_conversation_messages",
        "node_runs",
        "flow_run_events",
        "flow_run_checkpoints",
        "flow_run_callback_tasks",
    ] {
        assert!(
            model_codes.contains(&expected),
            "missing builtin runtime read model {expected}"
        );
    }

    let run_logs = metadata
        .iter()
        .find(|model| model.model_code == "application_run_log_summaries")
        .unwrap();
    assert_eq!(
        run_logs.physical_table_name,
        "application_run_log_summaries"
    );
    assert_eq!(run_logs.scope_column_name, "scope_id");
    assert!(run_logs.fields.iter().any(|field| {
        field.code == "flow_run_id"
            && field.physical_column_name == "flow_run_id"
            && !field.is_writable
    }));
    assert!(run_logs.fields.iter().any(|field| {
        field.code == "scope_id" && field.physical_column_name == "scope_id" && !field.is_writable
    }));
    assert!(run_logs.fields.iter().all(|field| !field.is_writable));

    let node_runs = metadata
        .iter()
        .find(|model| model.model_code == "node_runs")
        .unwrap();
    let node_field_codes = node_runs
        .fields
        .iter()
        .map(|field| field.code.as_str())
        .collect::<Vec<_>>();
    assert!(node_field_codes.contains(&"flow_run_id"));
    assert!(node_field_codes.contains(&"node_run_id"));
    assert!(!node_field_codes.contains(&"input_payload"));
    assert!(!node_field_codes.contains(&"output_payload"));
    assert!(!node_field_codes.contains(&"debug_payload"));
}

#[tokio::test]
async fn runtime_record_repository_lists_application_run_logs_as_scoped_read_model() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let seed = seed_runtime_read_model_rows(&store).await;
    let metadata = store
        .list_runtime_model_metadata()
        .await
        .unwrap()
        .into_iter()
        .find(|model| model.model_code == "application_run_log_summaries")
        .unwrap();

    let page = RuntimeRecordRepository::list_records(
        &store,
        &metadata,
        RuntimeListQuery {
            scope_id: Some(seed.workspace_id),
            owner_user_id: None,
            filter: domain::ResourceFilterExpr::All(vec![
                domain::ResourceFilterExpr::Field {
                    field: "title".into(),
                    operator: domain::ResourceFilterOperator::Includes,
                    value: json!("refund"),
                },
                domain::ResourceFilterExpr::Field {
                    field: "application_id".into(),
                    operator: domain::ResourceFilterOperator::Eq,
                    value: json!(seed.application_id.to_string()),
                },
                domain::ResourceFilterExpr::Field {
                    field: "scope_id".into(),
                    operator: domain::ResourceFilterOperator::Eq,
                    value: json!(seed.workspace_id.to_string()),
                },
                domain::ResourceFilterExpr::Field {
                    field: "started_at".into(),
                    operator: domain::ResourceFilterOperator::Gte,
                    value: json!("2026-05-29T07:59:00Z"),
                },
            ]),
            sorts: vec![RuntimeSortInput {
                field_code: "created_at".into(),
                direction: "desc".into(),
            }],
            expand_relations: vec![],
            page: 1,
            page_size: 1,
        },
    )
    .await
    .unwrap();

    assert_eq!(page.total, 1);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0]["id"], json!(seed.flow_run_id.to_string()));
    assert_eq!(
        page.items[0]["flow_run_id"],
        json!(seed.flow_run_id.to_string())
    );
    assert_eq!(
        page.items[0]["scope_id"],
        json!(seed.workspace_id.to_string())
    );
    assert_eq!(
        page.items[0]["application_id"],
        json!(seed.application_id.to_string())
    );
}

#[tokio::test]
async fn runtime_record_repository_lists_application_conversation_messages_by_declared_filters() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let seed = seed_runtime_read_model_rows(&store).await;
    let conversation_id = Uuid::now_v7();
    let message_id = Uuid::now_v7();
    let created_at = datetime!(2026-05-29 08:01:00 UTC);

    sqlx::query(
        r#"
        insert into application_conversations (
            id, scope_id, application_id, external_conversation_id, external_user,
            created_at, updated_at
        ) values ($1, $2, $3, 'conversation-1', 'customer-1', $4, $4)
        "#,
    )
    .bind(conversation_id)
    .bind(seed.workspace_id)
    .bind(seed.application_id)
    .bind(created_at)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into application_conversation_messages (
            id, scope_id, conversation_id, application_id, flow_run_id, node_run_id,
            role, content, sequence, created_at, updated_at
        ) values (
            $1, $2, $3, $4, $5, $6, 'assistant', 'Refund policy answer', 1, $7, $7
        )
        "#,
    )
    .bind(message_id)
    .bind(seed.workspace_id)
    .bind(conversation_id)
    .bind(seed.application_id)
    .bind(seed.flow_run_id)
    .bind(seed.node_run_id)
    .bind(created_at)
    .execute(store.pool())
    .await
    .unwrap();

    let metadata = store
        .list_runtime_model_metadata()
        .await
        .unwrap()
        .into_iter()
        .find(|model| model.model_code == "application_conversation_messages")
        .unwrap();
    let page = RuntimeRecordRepository::list_records(
        &store,
        &metadata,
        RuntimeListQuery {
            scope_id: Some(seed.workspace_id),
            owner_user_id: None,
            filter: domain::ResourceFilterExpr::All(vec![
                domain::ResourceFilterExpr::Field {
                    field: "conversation_id".into(),
                    operator: domain::ResourceFilterOperator::Eq,
                    value: json!(conversation_id.to_string()),
                },
                domain::ResourceFilterExpr::Field {
                    field: "flow_run_id".into(),
                    operator: domain::ResourceFilterOperator::Eq,
                    value: json!(seed.flow_run_id.to_string()),
                },
                domain::ResourceFilterExpr::Field {
                    field: "role".into(),
                    operator: domain::ResourceFilterOperator::Eq,
                    value: json!("assistant"),
                },
                domain::ResourceFilterExpr::Field {
                    field: "content".into(),
                    operator: domain::ResourceFilterOperator::Includes,
                    value: json!("policy"),
                },
            ]),
            sorts: vec![RuntimeSortInput {
                field_code: "created_at".into(),
                direction: "asc".into(),
            }],
            expand_relations: vec![],
            page: 1,
            page_size: 20,
        },
    )
    .await
    .unwrap();

    assert_eq!(page.total, 1);
    assert_eq!(page.items[0]["id"], json!(message_id.to_string()));
    assert_eq!(page.items[0]["role"], json!("assistant"));
    assert_eq!(page.items[0]["content"], json!("Refund policy answer"));
}

#[tokio::test]
async fn runtime_record_repository_lists_run_detail_shards_without_large_payload_columns() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let seed = seed_runtime_read_model_rows(&store).await;

    let metadata = store
        .list_runtime_model_metadata()
        .await
        .unwrap()
        .into_iter()
        .find(|model| model.model_code == "node_runs")
        .unwrap();
    let page = RuntimeRecordRepository::list_records(
        &store,
        &metadata,
        RuntimeListQuery {
            scope_id: Some(seed.workspace_id),
            owner_user_id: None,
            filter: domain::ResourceFilterExpr::All(vec![
                domain::ResourceFilterExpr::Field {
                    field: "flow_run_id".into(),
                    operator: domain::ResourceFilterOperator::Eq,
                    value: json!(seed.flow_run_id.to_string()),
                },
                domain::ResourceFilterExpr::Field {
                    field: "node_run_id".into(),
                    operator: domain::ResourceFilterOperator::Eq,
                    value: json!(seed.node_run_id.to_string()),
                },
            ]),
            sorts: vec![],
            expand_relations: vec![],
            page: 1,
            page_size: 20,
        },
    )
    .await
    .unwrap();

    assert_eq!(page.total, 1);
    assert_eq!(page.items[0]["id"], json!(seed.node_run_id.to_string()));
    assert_eq!(
        page.items[0]["node_run_id"],
        json!(seed.node_run_id.to_string())
    );
    assert!(page.items[0].get("input_payload").is_none());
    assert!(page.items[0].get("output_payload").is_none());
    assert!(page.items[0].get("debug_payload").is_none());
}
