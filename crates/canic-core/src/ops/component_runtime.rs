//! Module: ops::component_runtime
//!
//! Responsibility: hash managed runtime evidence and project Component Group members.
//! Does not own: Directory validation, storage mutation, endpoint authorization, or activation.
//! Boundary: callers supply passive authority for hashing or validated member discovery.

use crate::{
    InternalError,
    dto::{
        component_provisioning::ComponentGroupDirectory,
        component_registry::{ComponentRuntimeDirectChild, ComponentRuntimeDirectoryAuthority},
    },
};
use canic_contracts::ids::CanisterRole;
use ic_platform::principal::ExternalPrincipal as Principal;
use sha2::{Digest, Sha256};

///
/// ComponentRuntimeOps
///

pub struct ComponentRuntimeOps;

impl ComponentRuntimeOps {
    /// Project every exact role match, preserving the validated Directory's member order.
    pub(crate) fn group_members_by_role(
        directory: Option<&ComponentGroupDirectory>,
        role: &CanisterRole,
    ) -> Vec<Principal> {
        directory
            .into_iter()
            .flat_map(|directory| &directory.members)
            .filter(|member| &member.binding.role == role)
            .map(|member| member.binding.canister_id)
            .collect()
    }

    /// Hash one exact Fleet and Component Directory authority.
    pub fn directory_authority_hash(
        authority: &ComponentRuntimeDirectoryAuthority,
    ) -> Result<[u8; 32], InternalError> {
        const DOMAIN: &[u8] = b"canic.component-runtime.directory-authority.v1";
        let payload = canic_contracts::protocol::authority::encode(authority)
            .map_err(|_error| InternalError::invariant())?;
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
        let payload = canic_contracts::protocol::authority::encode(direct_children)
            .map_err(|_error| InternalError::invariant())?;
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN);
        hasher.update((payload.len() as u64).to_be_bytes());
        hasher.update(payload);
        Ok(hasher.finalize().into())
    }
}
