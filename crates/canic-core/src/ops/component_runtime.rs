//! Module: ops::component_runtime
//!
//! Responsibility: derive canonical evidence hashes for managed Component runtime authority.
//! Does not own: Directory validation, storage mutation, endpoint authorization, or activation.
//! Boundary: callers supply a complete passive authority and receive one domain-separated hash.

use crate::{
    InternalError, InternalErrorOrigin,
    dto::component_registry::{ComponentRuntimeDirectChild, ComponentRuntimeDirectoryAuthority},
};
use sha2::{Digest, Sha256};

///
/// ComponentRuntimeOps
///

pub struct ComponentRuntimeOps;

impl ComponentRuntimeOps {
    /// Hash one exact Fleet and Component Directory authority.
    pub fn directory_authority_hash(
        authority: &ComponentRuntimeDirectoryAuthority,
    ) -> Result<[u8; 32], InternalError> {
        const DOMAIN: &[u8] = b"canic.component-runtime.directory-authority.v1";
        let payload = candid::encode_one(authority).map_err(|error| {
            InternalError::invariant(
                InternalErrorOrigin::Ops,
                format!("Component runtime Directory authority cannot be encoded: {error}"),
            )
        })?;
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN);
        hasher.update((payload.len() as u64).to_be_bytes());
        hasher.update(payload);
        Ok(hasher.finalize().into())
    }

    /// Hash one exact ordered direct-child projection.
    pub fn direct_children_hash(
        direct_children: &[ComponentRuntimeDirectChild],
    ) -> Result<[u8; 32], InternalError> {
        const DOMAIN: &[u8] = b"canic.component-runtime.direct-children.v1";
        let payload = candid::encode_one(direct_children).map_err(|error| {
            InternalError::invariant(
                InternalErrorOrigin::Ops,
                format!("Component runtime direct children cannot be encoded: {error}"),
            )
        })?;
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN);
        hasher.update((payload.len() as u64).to_be_bytes());
        hasher.update(payload);
        Ok(hasher.finalize().into())
    }
}
