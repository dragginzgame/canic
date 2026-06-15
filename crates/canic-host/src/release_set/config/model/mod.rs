use std::path::PathBuf;

pub(super) const DEFAULT_INITIAL_CYCLES: u128 = 5_000_000_000_000;
pub const LOCAL_ROOT_MIN_READY_CYCLES: u128 = 100_000_000_000_000;
pub(super) const DEFAULT_RANDOMNESS_RESEED_INTERVAL_SECS: u64 = 3600;

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
    pub fleet: String,
    pub role: String,
    pub display: String,
    pub declaration_kind: String,
    pub package: String,
    pub attached: bool,
    pub state: String,
    pub topology: Option<String>,
}

///
/// DeclaredFleetRole
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredFleetRole {
    pub fleet: String,
    pub role: String,
    pub display: String,
    pub package: String,
}

///
/// AttachedFleetRole
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttachedFleetRole {
    pub fleet: String,
    pub role: String,
    pub display: String,
    pub subnet: String,
    pub kind: String,
    pub topology: String,
}

///
/// RenamedFleetRole
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenamedFleetRole {
    pub fleet: String,
    pub old_role: String,
    pub new_role: String,
    pub old_display: String,
    pub new_display: String,
    pub package_manifest: Option<PathBuf>,
    pub package_manifest_note: Option<String>,
}
