use crate::DurableBackendKind;

pub type MainDurableStore = storage_postgres::PgControlPlaneStore;

#[derive(Clone)]
pub struct MainDurableRuntime {
    pub kind: DurableBackendKind,
    pub store: MainDurableStore,
}

pub async fn build_main_durable_postgres(database_url: &str) -> anyhow::Result<MainDurableRuntime> {
    build_main_durable_postgres_with_max_connections(database_url, 5).await
}

pub async fn build_main_durable_postgres_with_max_connections(
    database_url: &str,
    max_connections: u32,
) -> anyhow::Result<MainDurableRuntime> {
    build_main_durable_postgres_with_pool_settings(
        database_url,
        storage_postgres::PgPoolSettings::with_max_connections(max_connections),
    )
    .await
}

pub async fn build_main_durable_postgres_with_pool_settings(
    database_url: &str,
    settings: storage_postgres::PgPoolSettings,
) -> anyhow::Result<MainDurableRuntime> {
    let pool = storage_postgres::connect_with_pool_settings(database_url, settings).await?;
    storage_postgres::run_migrations(&pool).await?;

    Ok(MainDurableRuntime {
        kind: DurableBackendKind::Postgres,
        store: storage_postgres::PgControlPlaneStore::new(pool),
    })
}
