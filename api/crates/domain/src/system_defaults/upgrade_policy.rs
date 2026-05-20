#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultUpgradePolicy {
    CreateOnly,
    SystemMaintainedAutoUpdate,
    UserConfigurablePreserveHistory,
    ExplicitMigrationOnly,
}

impl DefaultUpgradePolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CreateOnly => "create_only",
            Self::SystemMaintainedAutoUpdate => "system_maintained_auto_update",
            Self::UserConfigurablePreserveHistory => "user_configurable_preserve_history",
            Self::ExplicitMigrationOnly => "explicit_migration_only",
        }
    }
}
