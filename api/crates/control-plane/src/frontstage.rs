use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use access_control::ensure_permission;
use anyhow::Result;
use uuid::Uuid;

use crate::{
    audit::audit_log,
    errors::ControlPlaneError,
    ports::{
        CreateFrontstagePageInput, FrontstagePageRepository, MoveFrontstagePageInput,
        UpdateFrontstagePageTitleInput,
    },
};

pub struct CreateFrontstageGroupCommand {
    pub actor_user_id: Uuid,
    pub workspace_id: Uuid,
    pub title: Option<String>,
    pub parent_id: Option<Uuid>,
    pub rank: Option<String>,
}

pub struct CreateFrontstagePageCommand {
    pub actor_user_id: Uuid,
    pub workspace_id: Uuid,
    pub title: Option<String>,
    pub parent_id: Option<Uuid>,
    pub rank: Option<String>,
}

pub struct UpdateFrontstagePageTitleCommand {
    pub actor_user_id: Uuid,
    pub workspace_id: Uuid,
    pub page_id: Uuid,
    pub title: Option<String>,
}

pub struct MoveFrontstagePageCommand {
    pub actor_user_id: Uuid,
    pub workspace_id: Uuid,
    pub page_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub rank: Option<String>,
}

pub struct DeleteFrontstagePageCommand {
    pub actor_user_id: Uuid,
    pub workspace_id: Uuid,
    pub page_id: Uuid,
}

pub struct FrontstagePageService<R> {
    repository: R,
}

impl<R> FrontstagePageService<R>
where
    R: FrontstagePageRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn list_page_tree(
        &self,
        actor_user_id: Uuid,
        workspace_id: Uuid,
    ) -> Result<Vec<domain::FrontstagePageTreeNode>> {
        self.repository
            .load_actor_context_for_workspace(actor_user_id, workspace_id)
            .await?;
        let pages = self.repository.list_frontstage_pages(workspace_id).await?;

        Ok(build_frontstage_page_tree(pages))
    }

    pub async fn create_group(
        &self,
        command: CreateFrontstageGroupCommand,
    ) -> Result<domain::FrontstagePageRecord> {
        let actor = self
            .repository
            .load_actor_context_for_workspace(command.actor_user_id, command.workspace_id)
            .await?;
        ensure_design_permission(&actor)?;

        if command.parent_id.is_some() {
            return Err(ControlPlaneError::InvalidInput("parent_id").into());
        }

        let created = self
            .repository
            .create_frontstage_page(&CreateFrontstagePageInput {
                id: Uuid::now_v7(),
                workspace_id: command.workspace_id,
                actor_user_id: command.actor_user_id,
                parent_id: None,
                kind: domain::FrontstagePageKind::Group,
                title: command.title,
                rank: normalize_rank(command.rank),
                schema_root_uid: None,
            })
            .await?;
        self.audit(&actor, &created, "frontstage.page_group_created")
            .await?;

        Ok(created)
    }

    pub async fn create_page(
        &self,
        command: CreateFrontstagePageCommand,
    ) -> Result<domain::FrontstagePageRecord> {
        let actor = self
            .repository
            .load_actor_context_for_workspace(command.actor_user_id, command.workspace_id)
            .await?;
        ensure_design_permission(&actor)?;
        self.ensure_page_parent(command.workspace_id, command.parent_id)
            .await?;

        let page_id = Uuid::now_v7();
        let created = self
            .repository
            .create_frontstage_page(&CreateFrontstagePageInput {
                id: page_id,
                workspace_id: command.workspace_id,
                actor_user_id: command.actor_user_id,
                parent_id: command.parent_id,
                kind: domain::FrontstagePageKind::Page,
                title: command.title,
                rank: normalize_rank(command.rank),
                schema_root_uid: Some(reserved_schema_root_uid(page_id)),
            })
            .await?;
        self.audit(&actor, &created, "frontstage.page_created")
            .await?;

        Ok(created)
    }

    pub async fn update_title(
        &self,
        command: UpdateFrontstagePageTitleCommand,
    ) -> Result<domain::FrontstagePageRecord> {
        let actor = self
            .repository
            .load_actor_context_for_workspace(command.actor_user_id, command.workspace_id)
            .await?;
        ensure_design_permission(&actor)?;

        let updated = self
            .repository
            .update_frontstage_page_title(&UpdateFrontstagePageTitleInput {
                workspace_id: command.workspace_id,
                actor_user_id: command.actor_user_id,
                page_id: command.page_id,
                title: command.title,
            })
            .await?;
        self.audit(&actor, &updated, "frontstage.page_title_updated")
            .await?;

        Ok(updated)
    }

    pub async fn move_page(
        &self,
        command: MoveFrontstagePageCommand,
    ) -> Result<domain::FrontstagePageRecord> {
        let actor = self
            .repository
            .load_actor_context_for_workspace(command.actor_user_id, command.workspace_id)
            .await?;
        ensure_design_permission(&actor)?;

        let existing = self
            .repository
            .get_frontstage_page(command.workspace_id, command.page_id)
            .await?
            .ok_or(ControlPlaneError::NotFound("frontstage_page"))?;
        match existing.kind {
            domain::FrontstagePageKind::Group if command.parent_id.is_some() => {
                return Err(ControlPlaneError::InvalidInput("parent_id").into());
            }
            domain::FrontstagePageKind::Page => {
                self.ensure_page_parent(command.workspace_id, command.parent_id)
                    .await?;
            }
            domain::FrontstagePageKind::Group => {}
        }

        let moved = self
            .repository
            .move_frontstage_page(&MoveFrontstagePageInput {
                workspace_id: command.workspace_id,
                actor_user_id: command.actor_user_id,
                page_id: command.page_id,
                parent_id: command.parent_id,
                rank: normalize_rank(command.rank),
            })
            .await?;
        self.audit(&actor, &moved, "frontstage.page_moved").await?;

        Ok(moved)
    }

    pub async fn delete_page(&self, command: DeleteFrontstagePageCommand) -> Result<()> {
        let actor = self
            .repository
            .load_actor_context_for_workspace(command.actor_user_id, command.workspace_id)
            .await?;
        ensure_design_permission(&actor)?;

        let existing = self
            .repository
            .get_frontstage_page(command.workspace_id, command.page_id)
            .await?
            .ok_or(ControlPlaneError::NotFound("frontstage_page"))?;
        self.repository
            .delete_frontstage_page(command.workspace_id, command.page_id)
            .await?;
        self.audit(&actor, &existing, "frontstage.page_deleted")
            .await?;

        Ok(())
    }

    async fn ensure_page_parent(&self, workspace_id: Uuid, parent_id: Option<Uuid>) -> Result<()> {
        let Some(parent_id) = parent_id else {
            return Ok(());
        };

        let parent = self
            .repository
            .get_frontstage_page(workspace_id, parent_id)
            .await?
            .ok_or(ControlPlaneError::InvalidInput("parent_id"))?;

        if parent.kind != domain::FrontstagePageKind::Group {
            return Err(ControlPlaneError::InvalidInput("parent_id").into());
        }

        Ok(())
    }

    async fn audit(
        &self,
        actor: &domain::ActorContext,
        page: &domain::FrontstagePageRecord,
        event_code: &'static str,
    ) -> Result<()> {
        self.repository
            .append_audit_log(&audit_log(
                Some(actor.current_workspace_id),
                Some(actor.user_id),
                "frontstage_page",
                Some(page.id),
                event_code,
                serde_json::json!({
                    "kind": page.kind.as_str(),
                    "title": page.title,
                    "parent_id": page.parent_id,
                }),
            ))
            .await
    }
}

fn ensure_design_permission(actor: &domain::ActorContext) -> Result<()> {
    ensure_permission(actor, "frontstage.page.design")
        .map_err(ControlPlaneError::PermissionDenied)?;
    Ok(())
}

fn normalize_rank(rank: Option<String>) -> String {
    rank.unwrap_or_default()
}

fn reserved_schema_root_uid(page_id: Uuid) -> String {
    format!("frontstage_page_schema_root:{page_id}")
}

fn build_frontstage_page_tree(
    mut records: Vec<domain::FrontstagePageRecord>,
) -> Vec<domain::FrontstagePageTreeNode> {
    let existing_ids = records
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();

    for record in &mut records {
        if !matches!(record.parent_id, Some(parent_id) if existing_ids.contains(&parent_id)) {
            record.parent_id = None;
        }
    }

    records.sort_by(compare_frontstage_pages);

    let mut nodes_by_parent: HashMap<Option<Uuid>, Vec<domain::FrontstagePageRecord>> =
        HashMap::new();
    for record in records {
        nodes_by_parent
            .entry(record.parent_id)
            .or_default()
            .push(record);
    }

    fn flatten_group_children(
        group_id: Uuid,
        nodes_by_parent: &HashMap<Option<Uuid>, Vec<domain::FrontstagePageRecord>>,
        visiting_groups: &mut HashSet<Uuid>,
    ) -> Vec<domain::FrontstagePageTreeNode> {
        if !visiting_groups.insert(group_id) {
            return vec![];
        }

        let mut output = vec![];
        if let Some(children) = nodes_by_parent.get(&Some(group_id)) {
            output.reserve(children.len());
            for child in children {
                if child.kind == domain::FrontstagePageKind::Page {
                    output.push(domain::FrontstagePageTreeNode {
                        page: child.clone(),
                        children: vec![],
                    });
                    continue;
                }

                output.extend(flatten_group_children(
                    child.id,
                    nodes_by_parent,
                    visiting_groups,
                ));
            }
        }

        visiting_groups.remove(&group_id);
        output
    }

    nodes_by_parent
        .remove(&None)
        .unwrap_or_default()
        .into_iter()
        .map(|record| {
            let children = if record.kind == domain::FrontstagePageKind::Group {
                flatten_group_children(record.id, &nodes_by_parent, &mut HashSet::new())
            } else {
                vec![]
            };

            domain::FrontstagePageTreeNode {
                page: record,
                children,
            }
        })
        .collect()
}

fn compare_frontstage_pages(
    left: &domain::FrontstagePageRecord,
    right: &domain::FrontstagePageRecord,
) -> Ordering {
    let parent_cmp = left.parent_id.cmp(&right.parent_id);
    if parent_cmp != Ordering::Equal {
        return parent_cmp;
    }

    let rank_cmp = left.rank.cmp(&right.rank);
    if rank_cmp != Ordering::Equal {
        return rank_cmp;
    }

    left.id.cmp(&right.id)
}
