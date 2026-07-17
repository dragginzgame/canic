//! Module: replica_query::wire
//!
//! Responsibility: decode typed Candid responses returned by direct replica queries.
//! Does not own: HTTP transport, registry projection, or operator rendering.
//! Boundary: preserves canonical endpoint DTOs and typed Canic rejections.

#[cfg(test)]
mod tests;

use super::ReplicaQueryError;
use candid::Decode;
use canic_core::dto::{
    error::Error as CanicError, state::BootstrapStatusResponse, topology::SubnetRegistryResponse,
};

pub(super) fn decode_bootstrap_status_response(
    bytes: &[u8],
) -> Result<BootstrapStatusResponse, ReplicaQueryError> {
    Decode!(bytes, BootstrapStatusResponse).map_err(ReplicaQueryError::Candid)
}

pub(super) fn decode_cycle_balance_response(bytes: &[u8]) -> Result<u128, ReplicaQueryError> {
    let result = Decode!(bytes, Result<u128, CanicError>).map_err(ReplicaQueryError::Candid)?;
    result.map_err(ReplicaQueryError::Canister)
}

pub(super) fn decode_subnet_registry_response(
    bytes: &[u8],
) -> Result<SubnetRegistryResponse, ReplicaQueryError> {
    let result = Decode!(bytes, Result<SubnetRegistryResponse, CanicError>)
        .map_err(ReplicaQueryError::Candid)?;
    result.map_err(ReplicaQueryError::Canister)
}
