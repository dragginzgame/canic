mod attach;
mod declare;
mod rename;
mod support;

use super::model::{AttachedFleetRole, DeclaredFleetRole, RenamedFleetRole};
use std::path::PathBuf;

pub(in crate::release_set) use attach::attach_fleet_role_source;
pub(in crate::release_set) use declare::declare_fleet_role_source;
pub(in crate::release_set) use rename::rename_fleet_role_source;

///
/// DeclaredFleetRoleSource
///
#[derive(Debug)]
pub(in crate::release_set) struct DeclaredFleetRoleSource {
    pub(in crate::release_set) source: String,
    pub(in crate::release_set) role: DeclaredFleetRole,
}

///
/// AttachedFleetRoleSource
///
#[derive(Debug)]
pub(in crate::release_set) struct AttachedFleetRoleSource {
    pub(in crate::release_set) source: String,
    pub(in crate::release_set) role: AttachedFleetRole,
}

///
/// RenamedFleetRoleSource
///
#[derive(Debug)]
pub(in crate::release_set) struct RenamedFleetRoleSource {
    pub(in crate::release_set) source: String,
    pub(in crate::release_set) package_manifest: Option<PathBuf>,
    pub(in crate::release_set) package_source: Option<String>,
    pub(in crate::release_set) role: RenamedFleetRole,
}
