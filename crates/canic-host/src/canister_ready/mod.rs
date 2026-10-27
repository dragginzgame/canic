//! Module: canister_ready
//!
//! Responsibility: query the maintained Canic readiness endpoint.
//! Does not own: readiness state, local replica transport, or install orchestration.
//! Boundary: selects one transport and decodes the canonical boolean response.

use crate::{
    icp::{IcpCli, IcpCommandError, IcpJsonResponseError},
    icp_config::IcpConfigError,
    protocol_binding::RegistryProtocolBinding,
    replica_query,
    replica_query::ReplicaQueryError,
};
use canic_contracts::protocol::CANIC_PUBLIC_OVERVIEW;
use std::path::Path;
use thiserror::Error as ThisError;

///
/// CanisterReadyQueryError
///

#[derive(Debug, ThisError)]
pub enum CanisterReadyQueryError {
    #[error(transparent)]
    IcpConfig(#[from] IcpConfigError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(transparent)]
    Overview(#[from] crate::canic_metadata::CanicMetadataQueryError),

    #[error(transparent)]
    Replica(#[from] ReplicaQueryError),

    #[error(transparent)]
    Response(#[from] IcpJsonResponseError),
}

/// Query role-owned readiness, using the local replica API for local targets.
pub fn query_canister_ready(
    icp: &IcpCli,
    canister_id: &str,
    environment: &str,
    icp_root: Option<&Path>,
    binding: &RegistryProtocolBinding,
) -> Result<bool, CanisterReadyQueryError> {
    if replica_query::uses_local_replica_transport(Some(environment), icp_root)? {
        return query_local_canister_ready(environment, canister_id, icp_root).map_err(Into::into);
    }

    query_canister_ready_with_icp(icp, canister_id, binding)
}

/// Query role-owned readiness directly through the local replica API.
pub fn query_local_canister_ready(
    environment: &str,
    canister_id: &str,
    icp_root: Option<&Path>,
) -> Result<bool, ReplicaQueryError> {
    replica_query::query_ready(
        Some(environment),
        canister_id,
        CANIC_PUBLIC_OVERVIEW,
        icp_root,
    )
}

fn query_canister_ready_with_icp(
    icp: &IcpCli,
    canister_id: &str,
    binding: &RegistryProtocolBinding,
) -> Result<bool, CanisterReadyQueryError> {
    Ok(
        crate::canic_metadata::query_canic_overview(icp, canister_id, binding)?
            .bootstrap
            .ready,
    )
}
