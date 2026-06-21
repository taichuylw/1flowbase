use control_plane::ports::{
    ApiKeyRepository, CreateApiKeyInput, RoleRepository, UpsertApiKeyDataModelPermissionInput,
};
use domain::{ApiKeyKind, DataModelScopeKind};
use sqlx::PgPool;
use storage_postgres::{connect, run_migrations, PgControlPlaneStore};
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

#[tokio::test]
async fn role_queries_respect_requested_workspace_instead_of_first_workspace() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let tenant_id: Uuid = sqlx::query_scalar("select id from tenants where code = 'root-tenant'")
        .fetch_one(store.pool())
        .await
        .unwrap();
    let first_workspace_id = Uuid::now_v7();
    let second_workspace_id = Uuid::now_v7();
    let first_workspace_name = format!("Workspace A {}", first_workspace_id.simple());
    let second_workspace_name = format!("Workspace B {}", second_workspace_id.simple());

    sqlx::query(
        r#"
        insert into workspaces (id, tenant_id, name, created_by, updated_by)
        values ($1, $2, $3, null, null),
               ($4, $2, $5, null, null)
        "#,
    )
    .bind(first_workspace_id)
    .bind(tenant_id)
    .bind(&first_workspace_name)
    .bind(second_workspace_id)
    .bind(&second_workspace_name)
    .execute(store.pool())
    .await
    .unwrap();

    sqlx::query(
        r#"
        insert into roles (
            id, scope_kind, workspace_id, code, name, introduction, is_builtin, is_editable
        )
        values ($1, 'workspace', $2, 'reviewer', 'Reviewer', '', false, true)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(second_workspace_id)
    .execute(store.pool())
    .await
    .unwrap();

    let roles = RoleRepository::list_roles(&store, second_workspace_id)
        .await
        .unwrap();

    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].code, "reviewer");
}

#[tokio::test]
async fn api_key_data_model_permissions_reject_cross_scope_model_permission() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let tenant_id: Uuid = sqlx::query_scalar("select id from tenants where code = 'root-tenant'")
        .fetch_one(store.pool())
        .await
        .unwrap();
    let workspace_id = Uuid::now_v7();
    let other_workspace_id = Uuid::now_v7();
    let actor_user_id = Uuid::now_v7();

    sqlx::query(
        r#"
        insert into workspaces (id, tenant_id, name, created_by, updated_by)
        values ($1, $2, 'API Key Scope A', null, null),
               ($3, $2, 'API Key Scope B', null, null)
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(other_workspace_id)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into users (
            id, account, email, phone, password_hash, name, nickname, avatar_url, introduction,
            default_display_role, email_login_enabled, phone_login_enabled, status, session_version,
            created_by, updated_by
        ) values (
            $1, 'api-key-scope', 'api-key-scope@example.com', null, 'hash', 'API Key Scope',
            'API Key Scope', null, '', 'manager', true, false, 'active', 1, null, null
        )
        "#,
    )
    .bind(actor_user_id)
    .execute(store.pool())
    .await
    .unwrap();

    let api_key_id = Uuid::now_v7();
    ApiKeyRepository::create_api_key(
        &store,
        &CreateApiKeyInput {
            id: api_key_id,
            name: "Scoped key".into(),
            token_hash: format!("hash-{}", api_key_id.simple()),
            token_prefix: format!("dmk_{}", api_key_id.simple()),
            key_kind: ApiKeyKind::DataModelApiKey,
            application_id: None,
            creator_user_id: actor_user_id,
            tenant_id,
            scope_kind: DataModelScopeKind::Workspace,
            scope_id: workspace_id,
            enabled: true,
            expires_at: None,
        },
    )
    .await
    .unwrap();

    let cross_scope_model_id = Uuid::now_v7();
    let same_scope_model_id = Uuid::now_v7();
    sqlx::query(
        r#"
        insert into model_definitions (
            id, scope_kind, scope_id, code, title, physical_table_name, acl_namespace,
            audit_namespace, created_by, updated_by
        )
        values
            ($1, 'workspace', $2, 'cross_scope_orders', 'Cross Scope Orders',
             'cross_scope_orders', 'cross_scope_orders', 'cross_scope_orders', $5, $5),
            ($3, 'workspace', $4, 'same_scope_orders', 'Same Scope Orders',
             'same_scope_orders', 'same_scope_orders', 'same_scope_orders', $5, $5)
        "#,
    )
    .bind(cross_scope_model_id)
    .bind(other_workspace_id)
    .bind(same_scope_model_id)
    .bind(workspace_id)
    .bind(actor_user_id)
    .execute(store.pool())
    .await
    .unwrap();

    let error = ApiKeyRepository::replace_api_key_data_model_permissions(
        &store,
        api_key_id,
        &[UpsertApiKeyDataModelPermissionInput {
            api_key_id,
            data_model_id: cross_scope_model_id,
            allow_list: true,
            allow_get: true,
            allow_create: false,
            allow_update: false,
            allow_delete: false,
        }],
    )
    .await
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("api_key_data_model_permission owner scope mismatch"));

    let permission_count: i64 = sqlx::query_scalar(
        "select count(*) from api_key_data_model_permissions where api_key_id = $1",
    )
    .bind(api_key_id)
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(permission_count, 0);

    let permissions = ApiKeyRepository::replace_api_key_data_model_permissions(
        &store,
        api_key_id,
        &[UpsertApiKeyDataModelPermissionInput {
            api_key_id,
            data_model_id: same_scope_model_id,
            allow_list: true,
            allow_get: true,
            allow_create: false,
            allow_update: false,
            allow_delete: false,
        }],
    )
    .await
    .unwrap();

    assert_eq!(permissions.len(), 1);
    assert_eq!(permissions[0].data_model_id, same_scope_model_id);
}
