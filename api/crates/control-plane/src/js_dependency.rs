use access_control::ensure_permission;
use anyhow::Result;
use uuid::Uuid;

use crate::{
    errors::ControlPlaneError,
    ports::{AuthRepository, JsDependencyRepository},
};

pub struct ListWorkspaceJsDependenciesQuery {
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct JsDependencyCatalogView {
    pub entries: Vec<domain::JsDependencyRegistryEntry>,
}

pub struct JsDependencyService<R> {
    repository: R,
}

impl<R> JsDependencyService<R>
where
    R: AuthRepository + JsDependencyRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn list_workspace_js_dependencies(
        &self,
        query: ListWorkspaceJsDependenciesQuery,
    ) -> Result<JsDependencyCatalogView> {
        let actor = self
            .repository
            .load_actor_context_for_user(query.actor_user_id)
            .await?;
        ensure_permission(&actor, "plugin_config.view.all")
            .map_err(ControlPlaneError::PermissionDenied)?;

        Ok(JsDependencyCatalogView {
            entries: self
                .repository
                .list_workspace_js_dependencies(actor.current_workspace_id)
                .await?,
        })
    }
}
