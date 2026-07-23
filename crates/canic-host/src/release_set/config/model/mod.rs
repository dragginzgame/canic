use std::path::PathBuf;

pub(super) const DEFAULT_INITIAL_CYCLES: u128 = 5_000_000_000_000;
pub const LOCAL_ROOT_MIN_READY_CYCLES: u128 = 100_000_000_000_000;

///
/// ConfiguredPoolExpectation
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfiguredPoolExpectation {
    pub pool: String,
    pub canister_role: String,
}

///
/// ConfiguredRoleLifecycle
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfiguredRoleLifecycle {
    pub app: String,
    pub role: String,
    pub display: String,
    pub declaration_kind: String,
    pub package: String,
    pub attached: bool,
    pub state: String,
    pub topology: Option<String>,
}

///
/// DeclaredAppRole
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredAppRole {
    pub app: String,
    pub role: String,
    pub display: String,
    pub package: String,
}

///
/// AttachedAppRole
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttachedAppRole {
    pub app: String,
    pub role: String,
    pub display: String,
    pub subnet: String,
    pub kind: String,
    pub topology: String,
}

///
/// RenamedAppRole
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenamedAppRole {
    pub app: String,
    pub old_role: String,
    pub new_role: String,
    pub old_display: String,
    pub new_display: String,
    pub package_manifest: Option<PathBuf>,
    pub package_manifest_note: Option<String>,
}
