mod attach;
mod declare;
mod rename;
mod support;

use super::model::{AttachedAppRole, DeclaredAppRole, RenamedAppRole};
use std::path::PathBuf;

pub(in crate::release_set) use attach::attach_app_role_source;
pub(in crate::release_set) use declare::declare_app_role_source;
pub(in crate::release_set) use rename::rename_app_role_source;

///
/// DeclaredAppRoleSource
///
#[derive(Debug)]
pub(in crate::release_set) struct DeclaredAppRoleSource {
    pub(in crate::release_set) source: String,
    pub(in crate::release_set) role: DeclaredAppRole,
}

///
/// AttachedAppRoleSource
///
#[derive(Debug)]
pub(in crate::release_set) struct AttachedAppRoleSource {
    pub(in crate::release_set) source: String,
    pub(in crate::release_set) role: AttachedAppRole,
}

///
/// RenamedAppRoleSource
///
#[derive(Debug)]
pub(in crate::release_set) struct RenamedAppRoleSource {
    pub(in crate::release_set) source: String,
    pub(in crate::release_set) package_manifest: Option<PathBuf>,
    pub(in crate::release_set) package_source: Option<String>,
    pub(in crate::release_set) role: RenamedAppRole,
}
