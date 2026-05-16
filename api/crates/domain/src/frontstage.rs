use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontstagePageKind {
    Group,
    Page,
}

impl FrontstagePageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Group => "group",
            Self::Page => "page",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "group" => Some(Self::Group),
            "page" => Some(Self::Page),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontstagePageRecord {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub kind: FrontstagePageKind,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub schema_root_uid: Option<String>,
    pub rank: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontstagePageTreeNode {
    pub page: FrontstagePageRecord,
    pub children: Vec<FrontstagePageTreeNode>,
}
