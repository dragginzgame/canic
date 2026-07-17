//! Module: backup::create::executor::registry
//!
//! Responsibility: query typed registry entries needed by backup preflight.
//! Does not own: receipt construction or topology comparison.
//! Boundary: ICP/replica calls for subnet registry lookup.

use crate::{backup::BackupCommandError, support::candid::role_candid_path};
use canic_host::{
    icp::IcpCli,
    registry::RegistryEntry,
    subnet_registry::{SubnetRegistryQueryError, query_subnet_registry},
};
use std::path::Path;

pub(super) fn call_subnet_registry(
    options: &crate::backup::BackupCreateOptions,
    icp_root: &Path,
    root: &str,
) -> Result<Vec<RegistryEntry>, BackupCommandError> {
    let icp = IcpCli::new(&options.icp, None, Some(options.network.clone())).with_cwd(icp_root);
    let candid_path = role_candid_path(Some(icp_root), &options.network, "root");
    query_subnet_registry(
        &icp,
        root,
        &options.network,
        Some(icp_root),
        candid_path.as_deref(),
    )
    .map(|query| query.entries)
    .map_err(backup_subnet_registry_error)
}

pub(super) fn backup_subnet_registry_error(error: SubnetRegistryQueryError) -> BackupCommandError {
    match error {
        SubnetRegistryQueryError::Replica(err) => BackupCommandError::ReplicaQuery(err),
        SubnetRegistryQueryError::Icp(err) => BackupCommandError::Icp(err),
        SubnetRegistryQueryError::Registry(err) => BackupCommandError::Registry(err),
    }
}
