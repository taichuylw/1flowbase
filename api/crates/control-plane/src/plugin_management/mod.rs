mod artifact_instance;
mod catalog;
mod catalog_projection;
mod family;
mod filesystem;
mod install;
mod package_router;

use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use access_control::ensure_permission;
use anyhow::{Context, Result};
use plugin_framework::{
    compute_manifest_fingerprint, intake_package_bytes, parse_plugin_manifest,
    provider_package::ProviderPackage, PackageIntakePolicy, PackageIntakeResult, PluginManifestV1,
};
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    audit::audit_log,
    errors::ControlPlaneError,
    host_extension::{
        ensure_root_actor, ensure_uploaded_host_extensions_enabled, is_host_extension_installation,
        is_model_provider_installation, plugin_code_from_plugin_id,
    },
    i18n::{
        merge_i18n_catalog, plugin_namespace, trim_json_bundles, I18nCatalog, RequestedLocales,
    },
    plugin_lifecycle::derive_availability_status,
    ports::{
        AuthRepository, CreatePluginAssignmentInput, CreatePluginTaskInput,
        FrontendBlockCatalogRegistryInput, FrontendBlockCatalogRepository,
        JsDependencyRegistryInput, JsDependencyRepository, ModelProviderRepository,
        NodeContributionRegistryInput, NodeContributionRepository, OfficialPluginArtifact,
        OfficialPluginSourceEntry, OfficialPluginSourcePort, PluginRepository, ProviderRuntimePort,
        ReassignModelProviderInstancesInput, ReplaceInstallationFrontendBlocksInput,
        ReplaceInstallationJsDependenciesInput, ReplaceInstallationNodeContributionsInput,
        UpdatePluginDesiredStateInput, UpdatePluginRuntimeSnapshotInput,
        UpdatePluginTaskStatusInput, UpsertModelProviderCatalogCacheInput,
        UpsertPluginArtifactInstanceInput, UpsertPluginInstallationInput,
    },
    state_transition::ensure_plugin_task_transition,
};

pub use artifact_instance::*;
pub use catalog::*;
pub use catalog_projection::*;
pub use family::*;
pub use install::*;
pub use package_router::{route_plugin_package, RoutedPluginPackageKind};

pub struct PluginManagementService<R, H> {
    repository: R,
    runtime: H,
    official_source: Arc<dyn OfficialPluginSourcePort>,
    install_root: PathBuf,
    node_id: String,
    allow_uploaded_host_extensions: bool,
}

impl<R, H> PluginManagementService<R, H>
where
    R: AuthRepository
        + PluginRepository
        + ModelProviderRepository
        + NodeContributionRepository
        + JsDependencyRepository
        + FrontendBlockCatalogRepository,
    H: ProviderRuntimePort,
{
    pub fn new(
        repository: R,
        runtime: H,
        official_source: Arc<dyn OfficialPluginSourcePort>,
        install_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            repository,
            runtime,
            official_source,
            install_root: install_root.into(),
            node_id: String::new(),
            allow_uploaded_host_extensions: true,
        }
        .with_default_node_id()
    }

    pub fn with_allow_uploaded_host_extensions(mut self, allow: bool) -> Self {
        self.allow_uploaded_host_extensions = allow;
        self
    }

    fn with_default_node_id(mut self) -> Self {
        if self.node_id.is_empty() {
            self.node_id = format!("local:{}", self.install_root.display());
        }
        self
    }

    pub fn with_node_id(mut self, node_id: impl Into<String>) -> Self {
        let node_id = node_id.into();
        let node_id = node_id.trim();
        if !node_id.is_empty() {
            self.node_id = node_id.to_string();
        }
        self
    }
}
