use std::{cmp::Ordering, collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use control_plane::workspace::WorkspaceService;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    app_state::ApiState, error_response::ApiError, middleware::require_session::require_session,
    response::ApiSuccess,
};

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FrontstagePageTreeNodeKind {
    Group,
    Page,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FrontstagePageTreeNodeResponse {
    pub id: String,
    pub title: Option<String>,
    pub kind: FrontstagePageTreeNodeKind,
    #[serde(default)]
    pub children: Vec<FrontstagePageTreeNodeResponse>,
}

#[derive(Debug)]
struct FrontstagePageRecord {
    id: Uuid,
    title: Option<String>,
    kind: FrontstagePageTreeNodeKind,
    parent_id: Option<Uuid>,
    rank: Option<String>,
}

#[derive(Debug)]
struct FrontstagePageTreeNode {
    id: Uuid,
    node: FrontstagePageTreeNodeResponse,
}

pub fn router() -> Router<Arc<ApiState>> {
    Router::new().route("/frontstage/:workspace_id/pages", get(list_frontstage_pages))
}

#[utoipa::path(
    get,
    path = "/api/console/frontstage/{workspace_id}/pages",
    params(
        ("workspace_id" = String, Path, description = "Workspace id"),
    ),
    responses(
        (status = 200, body = [FrontstagePageTreeNodeResponse]),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody)
    )
)]
pub async fn list_frontstage_pages(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(workspace_id): Path<String>,
) -> Result<Json<ApiSuccess<Vec<FrontstagePageTreeNodeResponse>>>, ApiError> {
    let context = require_session(&state, &headers).await?;

    let workspace_id = parse_uuid(&workspace_id, "workspace_id")?;
    WorkspaceService::new(state.store.clone())
        .get_accessible_workspace(context.user.id, workspace_id)
        .await?;

    let rows = sqlx::query(
        "
        select id, title, kind, parent_id, rank
        from frontstage_pages
        where workspace_id = $1
        order by workspace_id, parent_id nulls first, rank nulls last
        ",
    )
    .bind(workspace_id)
    .fetch_all(state.store.pool())
    .await?;

    let records = rows
        .into_iter()
        .map(|row| {
            let raw_kind = row.get::<String, _>("kind");
            Ok(FrontstagePageRecord {
                id: row.get("id"),
                title: row.get("title"),
                kind: parse_frontstage_page_kind(&raw_kind)?,
                parent_id: row.get("parent_id"),
                rank: row.get("rank"),
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    Ok(Json(ApiSuccess::new(build_frontstage_page_tree(records))))
}

fn parse_uuid(raw: &str, field: &'static str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| control_plane::errors::ControlPlaneError::InvalidInput(field).into())
}

fn parse_frontstage_page_kind(
    raw_kind: &str,
) -> Result<FrontstagePageTreeNodeKind, ApiError> {
    match raw_kind {
        "group" => Ok(FrontstagePageTreeNodeKind::Group),
        "page" => Ok(FrontstagePageTreeNodeKind::Page),
        _ => Err(control_plane::errors::ControlPlaneError::InvalidInput("kind").into()),
    }
}

fn build_frontstage_page_tree(
    mut records: Vec<FrontstagePageRecord>,
) -> Vec<FrontstagePageTreeNodeResponse> {
    records.sort_by(|left, right| {
        let parent_cmp = left.parent_id.cmp(&right.parent_id);
        if parent_cmp != Ordering::Equal {
            return parent_cmp;
        }

        match (&left.rank, &right.rank) {
            (Some(left_rank), Some(right_rank)) => left_rank.cmp(right_rank),
            (None, None) => left.id.cmp(&right.id),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
        }
    });

    let mut nodes_by_parent: HashMap<Option<Uuid>, Vec<FrontstagePageTreeNode>> = HashMap::new();
    for record in records {
        nodes_by_parent
            .entry(record.parent_id)
            .or_default()
            .push(FrontstagePageTreeNode {
                id: record.id,
                node: FrontstagePageTreeNodeResponse {
                    id: record.id.to_string(),
                    title: record.title,
                    kind: record.kind,
                    children: vec![],
                },
            });
    }

    let mut roots = nodes_by_parent.remove(&None).unwrap_or_default();
    for root in &mut roots {
        root.node.children = nodes_by_parent
            .remove(&Some(root.id))
            .unwrap_or_default()
            .into_iter()
            .map(|child| child.node)
            .collect();
    }

    roots.into_iter().map(|node| node.node).collect()
}
