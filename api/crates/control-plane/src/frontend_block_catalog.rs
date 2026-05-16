use access_control::ensure_permission;
use anyhow::Result;
use uuid::Uuid;

use crate::{
    errors::ControlPlaneError,
    ports::{AuthRepository, FrontendBlockCatalogRepository},
};

pub struct ListFrontendBlockCatalogQuery {
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct FrontendBlockCatalogView {
    pub entries: Vec<domain::FrontendBlockCatalogEntry>,
}

pub struct FrontendBlockCatalogService<R> {
    repository: R,
}

impl<R> FrontendBlockCatalogService<R>
where
    R: AuthRepository + FrontendBlockCatalogRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn list_frontend_blocks(
        &self,
        query: ListFrontendBlockCatalogQuery,
    ) -> Result<FrontendBlockCatalogView> {
        let actor = self
            .repository
            .load_actor_context_for_user(query.actor_user_id)
            .await?;
        ensure_permission(&actor, "plugin_config.view.all")
            .map_err(ControlPlaneError::PermissionDenied)?;

        Ok(FrontendBlockCatalogView {
            entries: self
                .repository
                .list_workspace_frontend_blocks(actor.current_workspace_id)
                .await?,
        })
    }
}
