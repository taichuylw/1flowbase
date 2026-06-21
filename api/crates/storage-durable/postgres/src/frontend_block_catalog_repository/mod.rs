use anyhow::Result;
use async_trait::async_trait;
use control_plane::ports::{
    FrontendBlockCatalogRepository, ReplaceInstallationFrontendBlocksInput,
};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    mappers::frontend_block_catalog_mapper::{
        PgFrontendBlockCatalogMapper, StoredFrontendBlockCatalogRow,
    },
    repositories::PgControlPlaneStore,
};

fn map_catalog_row(row: sqlx::postgres::PgRow) -> Result<domain::FrontendBlockCatalogEntry> {
    PgFrontendBlockCatalogMapper::to_catalog_entry(StoredFrontendBlockCatalogRow {
        installation_id: row.get("installation_id"),
        provider_code: row.get("provider_code"),
        plugin_id: row.get("plugin_id"),
        plugin_version: row.get("plugin_version"),
        contribution_code: row.get("contribution_code"),
        title: row.get("title"),
        runtime: row.get("runtime"),
        entry: row.get("entry"),
        context_contract: row.get("context_contract"),
        permission_network: row.get("permission_network"),
        permission_storage: row.get("permission_storage"),
        permission_secrets: row.get("permission_secrets"),
        ui_capabilities: row.get("ui_capabilities"),
    })
}

#[async_trait]
impl FrontendBlockCatalogRepository for PgControlPlaneStore {
    async fn replace_installation_frontend_blocks(
        &self,
        input: &ReplaceInstallationFrontendBlocksInput,
    ) -> Result<()> {
        let mut tx = self.pool().begin().await?;
        sqlx::query(
            r#"
            delete from frontend_block_catalog
            where installation_id = $1
            "#,
        )
        .bind(input.installation_id)
        .execute(&mut *tx)
        .await?;

        for entry in &input.entries {
            let context_contract = serde_json::json!({
                "primitives": entry.context_contract.primitives,
                "input_schema": entry.context_contract.input_schema,
            });
            sqlx::query(
                r#"
                insert into frontend_block_catalog (
                    id,
                    scope_id,
                    installation_id,
                    provider_code,
                    plugin_id,
                    plugin_version,
                    contribution_code,
                    title,
                    runtime,
                    entry,
                    context_contract,
                    permission_network,
                    permission_storage,
                    permission_secrets,
                    ui_capabilities
                ) values (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
                )
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(domain::SYSTEM_SCOPE_ID)
            .bind(input.installation_id)
            .bind(&input.provider_code)
            .bind(&input.plugin_id)
            .bind(&input.plugin_version)
            .bind(&entry.contribution_code)
            .bind(&entry.title)
            .bind(&entry.runtime)
            .bind(&entry.entry)
            .bind(context_contract)
            .bind(&entry.permissions.network)
            .bind(&entry.permissions.storage)
            .bind(&entry.permissions.secrets)
            .bind(serde_json::to_value(&entry.ui_capabilities)?)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn list_workspace_frontend_blocks(
        &self,
        workspace_id: Uuid,
    ) -> Result<Vec<domain::FrontendBlockCatalogEntry>> {
        let rows = sqlx::query(
            r#"
            select
                reg.installation_id,
                reg.provider_code,
                reg.plugin_id,
                reg.plugin_version,
                reg.contribution_code,
                reg.title,
                reg.runtime,
                reg.entry,
                reg.context_contract,
                reg.permission_network,
                reg.permission_storage,
                reg.permission_secrets,
                reg.ui_capabilities
            from frontend_block_catalog reg
            inner join plugin_assignments pa
                on pa.workspace_id = $1
               and pa.installation_id = reg.installation_id
            order by reg.title asc, reg.contribution_code asc
            "#,
        )
        .bind(workspace_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(map_catalog_row).collect()
    }
}
