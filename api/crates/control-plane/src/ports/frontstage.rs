use super::*;

#[derive(Debug, Clone)]
pub struct CreateFrontstagePageInput {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub actor_user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub kind: domain::FrontstagePageKind,
    pub title: Option<String>,
    pub rank: String,
    pub schema_root_uid: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateFrontstagePageTitleInput {
    pub workspace_id: Uuid,
    pub actor_user_id: Uuid,
    pub page_id: Uuid,
    pub title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MoveFrontstagePageInput {
    pub workspace_id: Uuid,
    pub actor_user_id: Uuid,
    pub page_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub rank: String,
}

#[async_trait]
pub trait FrontstagePageRepository: Send + Sync {
    async fn load_actor_context_for_workspace(
        &self,
        actor_user_id: Uuid,
        workspace_id: Uuid,
    ) -> anyhow::Result<domain::ActorContext>;

    async fn list_frontstage_pages(
        &self,
        workspace_id: Uuid,
    ) -> anyhow::Result<Vec<domain::FrontstagePageRecord>>;

    async fn get_frontstage_page(
        &self,
        workspace_id: Uuid,
        page_id: Uuid,
    ) -> anyhow::Result<Option<domain::FrontstagePageRecord>>;

    async fn create_frontstage_page(
        &self,
        input: &CreateFrontstagePageInput,
    ) -> anyhow::Result<domain::FrontstagePageRecord>;

    async fn update_frontstage_page_title(
        &self,
        input: &UpdateFrontstagePageTitleInput,
    ) -> anyhow::Result<domain::FrontstagePageRecord>;

    async fn move_frontstage_page(
        &self,
        input: &MoveFrontstagePageInput,
    ) -> anyhow::Result<domain::FrontstagePageRecord>;

    async fn delete_frontstage_page(&self, workspace_id: Uuid, page_id: Uuid)
        -> anyhow::Result<()>;

    async fn append_audit_log(&self, event: &domain::AuditLogRecord) -> anyhow::Result<()>;
}
