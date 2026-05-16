use anyhow::Result;
use async_trait::async_trait;
use control_plane::{
    errors::ControlPlaneError,
    ports::{
        AuthRepository, CreateFrontstagePageInput, FrontstagePageRepository,
        MoveFrontstagePageInput, UpdateFrontstagePageTitleInput, WorkspaceRepository,
    },
};
use sqlx::Row;
use uuid::Uuid;

use crate::repositories::PgControlPlaneStore;

fn map_frontstage_page_row(row: sqlx::postgres::PgRow) -> Result<domain::FrontstagePageRecord> {
    let raw_kind: String = row.get("kind");
    let kind = domain::FrontstagePageKind::from_db(&raw_kind)
        .ok_or(ControlPlaneError::InvalidInput("frontstage_page_kind"))?;

    Ok(domain::FrontstagePageRecord {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        parent_id: row.get("parent_id"),
        kind,
        title: row.get("title"),
        slug: row.get("slug"),
        schema_root_uid: row.get("schema_root_uid"),
        rank: row.get("rank"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

#[async_trait]
impl FrontstagePageRepository for PgControlPlaneStore {
    async fn load_actor_context_for_workspace(
        &self,
        actor_user_id: Uuid,
        workspace_id: Uuid,
    ) -> Result<domain::ActorContext> {
        let workspace =
            WorkspaceRepository::get_accessible_workspace(self, actor_user_id, workspace_id)
                .await?
                .ok_or(ControlPlaneError::PermissionDenied(
                    "workspace_access_denied",
                ))?;

        AuthRepository::load_actor_context(
            self,
            actor_user_id,
            workspace.tenant_id,
            workspace.id,
            None,
        )
        .await
    }

    async fn list_frontstage_pages(
        &self,
        workspace_id: Uuid,
    ) -> Result<Vec<domain::FrontstagePageRecord>> {
        let rows = sqlx::query(
            r#"
            select
                id,
                workspace_id,
                parent_id,
                kind,
                title,
                slug,
                schema_root_uid,
                rank,
                created_at,
                updated_at
            from frontstage_pages
            where workspace_id = $1
            order by parent_id nulls first, rank asc, id asc
            "#,
        )
        .bind(workspace_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(map_frontstage_page_row).collect()
    }

    async fn get_frontstage_page(
        &self,
        workspace_id: Uuid,
        page_id: Uuid,
    ) -> Result<Option<domain::FrontstagePageRecord>> {
        let row = sqlx::query(
            r#"
            select
                id,
                workspace_id,
                parent_id,
                kind,
                title,
                slug,
                schema_root_uid,
                rank,
                created_at,
                updated_at
            from frontstage_pages
            where workspace_id = $1 and id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(page_id)
        .fetch_optional(self.pool())
        .await?;

        row.map(map_frontstage_page_row).transpose()
    }

    async fn create_frontstage_page(
        &self,
        input: &CreateFrontstagePageInput,
    ) -> Result<domain::FrontstagePageRecord> {
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"
            insert into frontstage_pages (
                id,
                workspace_id,
                parent_id,
                kind,
                title,
                schema_root_uid,
                rank
            ) values ($1, $2, $3, $4, $5, $6, $7)
            returning
                id,
                workspace_id,
                parent_id,
                kind,
                title,
                slug,
                schema_root_uid,
                rank,
                created_at,
                updated_at
            "#,
        )
        .bind(input.id)
        .bind(input.workspace_id)
        .bind(input.parent_id)
        .bind(input.kind.as_str())
        .bind(&input.title)
        .bind(&input.schema_root_uid)
        .bind(&input.rank)
        .fetch_one(&mut *tx)
        .await?;
        let page = map_frontstage_page_row(row)?;
        tx.commit().await?;

        Ok(page)
    }

    async fn update_frontstage_page_title(
        &self,
        input: &UpdateFrontstagePageTitleInput,
    ) -> Result<domain::FrontstagePageRecord> {
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"
            update frontstage_pages
            set title = $3,
                updated_at = now()
            where workspace_id = $1 and id = $2
            returning
                id,
                workspace_id,
                parent_id,
                kind,
                title,
                slug,
                schema_root_uid,
                rank,
                created_at,
                updated_at
            "#,
        )
        .bind(input.workspace_id)
        .bind(input.page_id)
        .bind(&input.title)
        .fetch_optional(&mut *tx)
        .await?;
        let page = row
            .map(map_frontstage_page_row)
            .transpose()?
            .ok_or(ControlPlaneError::NotFound("frontstage_page"))?;
        tx.commit().await?;

        Ok(page)
    }

    async fn move_frontstage_page(
        &self,
        input: &MoveFrontstagePageInput,
    ) -> Result<domain::FrontstagePageRecord> {
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"
            update frontstage_pages
            set parent_id = $3,
                rank = $4,
                updated_at = now()
            where workspace_id = $1 and id = $2
            returning
                id,
                workspace_id,
                parent_id,
                kind,
                title,
                slug,
                schema_root_uid,
                rank,
                created_at,
                updated_at
            "#,
        )
        .bind(input.workspace_id)
        .bind(input.page_id)
        .bind(input.parent_id)
        .bind(&input.rank)
        .fetch_optional(&mut *tx)
        .await?;
        let page = row
            .map(map_frontstage_page_row)
            .transpose()?
            .ok_or(ControlPlaneError::NotFound("frontstage_page"))?;
        tx.commit().await?;

        Ok(page)
    }

    async fn delete_frontstage_page(&self, workspace_id: Uuid, page_id: Uuid) -> Result<()> {
        let mut tx = self.pool().begin().await?;
        let deleted = sqlx::query(
            r#"
            delete from frontstage_pages
            where workspace_id = $1 and id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(page_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if deleted == 0 {
            return Err(ControlPlaneError::NotFound("frontstage_page").into());
        }

        tx.commit().await?;
        Ok(())
    }

    async fn append_audit_log(&self, event: &domain::AuditLogRecord) -> Result<()> {
        AuthRepository::append_audit_log(self, event).await
    }
}
