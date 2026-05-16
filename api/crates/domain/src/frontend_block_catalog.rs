use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrontendBlockPermissions {
    pub network: String,
    pub storage: String,
    pub secrets: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrontendBlockContextContract {
    pub primitives: Vec<String>,
    pub input_schema: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrontendBlockCatalogEntry {
    pub installation_id: Uuid,
    pub provider_code: String,
    pub plugin_id: String,
    pub plugin_version: String,
    pub contribution_code: String,
    pub title: String,
    pub runtime: String,
    pub entry: String,
    pub context_contract: FrontendBlockContextContract,
    pub permissions: FrontendBlockPermissions,
    pub ui_capabilities: Vec<String>,
}
