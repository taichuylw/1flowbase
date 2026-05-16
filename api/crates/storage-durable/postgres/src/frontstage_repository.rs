use anyhow::Result;
use async_trait::async_trait;
use control_plane::{
    errors::ControlPlaneError,
    ports::{
        AuthRepository, CreateFrontstagePageInput, FrontstagePageRepository,
        MoveFrontstagePageInput, SaveFrontstageBlockCodeInput, UpdateFrontstagePageTitleInput,
        WorkspaceRepository,
    },
};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::repositories::PgControlPlaneStore;

fn map_frontstage_page_row(row: &sqlx::postgres::PgRow) -> Result<domain::FrontstagePageRecord> {
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

fn map_frontstage_schema_row(
    row: &sqlx::postgres::PgRow,
) -> domain::frontstage::FrontstagePageSchemaRecord {
    domain::frontstage::FrontstagePageSchemaRecord {
        workspace_id: row.get("schema_workspace_id"),
        page_id: row.get("schema_page_id"),
        root_uid: row.get("root_uid"),
        schema_payload: row.get("schema_payload"),
        root_payload: row.get("root_payload"),
        created_at: row.get("schema_created_at"),
        updated_at: row.get("schema_updated_at"),
    }
}

fn map_frontstage_block_code_row(
    row: sqlx::postgres::PgRow,
) -> domain::frontstage::FrontstageBlockCodeRecord {
    domain::frontstage::FrontstageBlockCodeRecord {
        workspace_id: row.get("workspace_id"),
        page_id: row.get("page_id"),
        code_ref: row.get("code_ref"),
        code: row.get("code"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
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

        rows.into_iter()
            .map(|row| map_frontstage_page_row(&row))
            .collect()
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

        row.map(|row| map_frontstage_page_row(&row)).transpose()
    }

    async fn get_frontstage_page_detail(
        &self,
        workspace_id: Uuid,
        page_id: Uuid,
    ) -> Result<Option<domain::frontstage::FrontstagePageDetail>> {
        let row = sqlx::query(
            r#"
            select
                p.id,
                p.workspace_id,
                p.parent_id,
                p.kind,
                p.title,
                p.slug,
                p.schema_root_uid,
                p.rank,
                p.created_at,
                p.updated_at,
                s.workspace_id as schema_workspace_id,
                s.page_id as schema_page_id,
                s.root_uid,
                s.schema_payload,
                s.root_payload,
                s.created_at as schema_created_at,
                s.updated_at as schema_updated_at
            from frontstage_pages p
            join frontstage_page_schemas s
              on s.workspace_id = p.workspace_id
             and s.page_id = p.id
            where p.workspace_id = $1 and p.id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(page_id)
        .fetch_optional(self.pool())
        .await?;

        row.map(|row| {
            let page = map_frontstage_page_row(&row)?;
            Ok(domain::frontstage::FrontstagePageDetail {
                page,
                schema: map_frontstage_schema_row(&row),
            })
        })
        .transpose()
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
        let page = map_frontstage_page_row(&row)?;
        if let Some(root_uid) = &input.schema_root_uid {
            sqlx::query(
                r#"
                insert into frontstage_page_schemas (
                    page_id,
                    workspace_id,
                    root_uid,
                    schema_payload,
                    root_payload
                ) values ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(input.id)
            .bind(input.workspace_id)
            .bind(root_uid)
            .bind(json!({
                "version": 1,
                "root_uid": root_uid,
                "nodes": []
            }))
            .bind(json!({
                "uid": root_uid,
                "kind": "frontstage.page.root",
                "children": []
            }))
            .execute(&mut *tx)
            .await?;
        }
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
            .map(|row| map_frontstage_page_row(&row))
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
            .map(|row| map_frontstage_page_row(&row))
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

    async fn get_frontstage_block_code(
        &self,
        workspace_id: Uuid,
        page_id: Uuid,
        code_ref: &str,
    ) -> Result<Option<domain::frontstage::FrontstageBlockCodeRecord>> {
        let row = sqlx::query(
            r#"
            select workspace_id, page_id, code_ref, code, created_at, updated_at
            from frontstage_block_codes
            where workspace_id = $1 and page_id = $2 and code_ref = $3
            "#,
        )
        .bind(workspace_id)
        .bind(page_id)
        .bind(code_ref)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(map_frontstage_block_code_row))
    }

    async fn save_frontstage_block_code(
        &self,
        input: &SaveFrontstageBlockCodeInput,
    ) -> Result<domain::frontstage::FrontstageBlockCodeRecord> {
        let row = sqlx::query(
            r#"
            insert into frontstage_block_codes (
                id,
                workspace_id,
                page_id,
                code_ref,
                code
            ) values ($1, $2, $3, $4, $5)
            on conflict (workspace_id, page_id, code_ref)
            do update set
                code = excluded.code,
                updated_at = now()
            returning workspace_id, page_id, code_ref, code, created_at, updated_at
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(input.workspace_id)
        .bind(input.page_id)
        .bind(&input.code_ref)
        .bind(&input.code)
        .fetch_one(self.pool())
        .await?;

        Ok(map_frontstage_block_code_row(row))
    }

    async fn append_audit_log(&self, event: &domain::AuditLogRecord) -> Result<()> {
        AuthRepository::append_audit_log(self, event).await
    }
}
