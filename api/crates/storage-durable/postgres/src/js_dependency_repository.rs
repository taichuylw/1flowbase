use anyhow::Result;
use async_trait::async_trait;
use control_plane::ports::{JsDependencyRepository, ReplaceInstallationJsDependenciesInput};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    mappers::js_dependency_mapper::{PgJsDependencyMapper, StoredJsDependencyRegistryRow},
    repositories::PgControlPlaneStore,
};

fn map_registry_row(row: sqlx::postgres::PgRow) -> Result<domain::JsDependencyRegistryEntry> {
    PgJsDependencyMapper::to_registry_entry(StoredJsDependencyRegistryRow {
        installation_id: row.get("installation_id"),
        provider_code: row.get("provider_code"),
        plugin_id: row.get("plugin_id"),
        plugin_version: row.get("plugin_version"),
        alias: row.get("alias"),
        package: row.get("package"),
        version: row.get("version"),
        target: row.get("target"),
        artifact_path: row.get("artifact_path"),
        integrity: row.get("integrity"),
        permission_network: row.get("permission_network"),
        permission_filesystem: row.get("permission_filesystem"),
        permission_env: row.get("permission_env"),
    })
}

#[async_trait]
impl JsDependencyRepository for PgControlPlaneStore {
    async fn replace_installation_js_dependencies(
        &self,
        input: &ReplaceInstallationJsDependenciesInput,
    ) -> Result<()> {
        let mut tx = self.pool().begin().await?;
        sqlx::query(
            r#"
            delete from js_dependency_registry
            where installation_id = $1
            "#,
        )
        .bind(input.installation_id)
        .execute(&mut *tx)
        .await?;

        for entry in &input.entries {
            sqlx::query(
                r#"
                insert into js_dependency_registry (
                    id,
                    installation_id,
                    provider_code,
                    plugin_id,
                    plugin_version,
                    alias,
                    package,
                    version,
                    target,
                    artifact_path,
                    integrity,
                    permission_network,
                    permission_filesystem,
                    permission_env
                ) values (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
                )
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(input.installation_id)
            .bind(&input.provider_code)
            .bind(&input.plugin_id)
            .bind(&input.plugin_version)
            .bind(&entry.alias)
            .bind(&entry.package)
            .bind(&entry.version)
            .bind(&entry.target)
            .bind(&entry.artifact_path)
            .bind(&entry.integrity)
            .bind(&entry.permissions.network)
            .bind(&entry.permissions.filesystem)
            .bind(&entry.permissions.env)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn list_workspace_js_dependencies(
        &self,
        workspace_id: Uuid,
    ) -> Result<Vec<domain::JsDependencyRegistryEntry>> {
        let rows = sqlx::query(
            r#"
            select
                reg.installation_id,
                reg.provider_code,
                reg.plugin_id,
                reg.plugin_version,
                reg.alias,
                reg.package,
                reg.version,
                reg.target,
                reg.artifact_path,
                reg.integrity,
                reg.permission_network,
                reg.permission_filesystem,
                reg.permission_env
            from js_dependency_registry reg
            inner join plugin_assignments pa
                on pa.workspace_id = $1
               and pa.installation_id = reg.installation_id
            order by reg.alias asc, reg.target asc
            "#,
        )
        .bind(workspace_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(map_registry_row).collect()
    }
}
