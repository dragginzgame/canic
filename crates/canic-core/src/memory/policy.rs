//! Module: memory::policy
//!
//! Responsibility: compose Canic memory bootstrap admission and policy identity.
//! Does not own: memory-manager storage, stable schemas, or diagnostics rendering.
//! Boundary: memory bootstrap passes this policy into `ic-memory` validation.

use crate::memory::{admission::MemoryBootstrapAdmission, registry::MemoryRegistryError};
use ic_memory::{
    MemoryManagerSlot, PolicyIdentity, PolicyIdentityError, RuntimeBootstrapPolicy, StableKey,
};
use sha2::{Digest as _, Sha256};

const CANIC_MEMORY_BOOTSTRAP_POLICY_NAME: &str = "canic.memory-bootstrap-policy";
const CANIC_MEMORY_BOOTSTRAP_POLICY_VERSION: u32 = 1;

///
/// CanicMemoryManagerPolicy
///
/// Canic policy adapter for the `ic-memory` MemoryManager substrate
/// allocation slots.
/// Owned by memory policy and supplied to memory-manager bootstrap.
///

#[derive(Clone, Debug, Default)]
pub struct CanicMemoryManagerPolicy {
    admission: Option<MemoryBootstrapAdmission>,
}

impl CanicMemoryManagerPolicy {
    /// Use Canic's bootstrap identity without a consumer admission callback.
    #[must_use]
    pub const fn new() -> Self {
        Self { admission: None }
    }

    /// Compose consumer admission with Canic's bootstrap identity.
    #[must_use]
    pub fn with_admission(mut self, admission: MemoryBootstrapAdmission) -> Self {
        self.admission = Some(admission);
        self
    }
}

impl RuntimeBootstrapPolicy for CanicMemoryManagerPolicy {
    type Error = MemoryRegistryError;

    fn validate_allocation(
        &self,
        _key: &StableKey,
        _slot: &MemoryManagerSlot,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn prepare_bootstrap(
        &self,
        admission: &mut ic_memory::BootstrapAdmission<'_>,
    ) -> Result<(), Self::Error> {
        match &self.admission {
            Some(participant) => (participant.prepare)(admission),
            None => Ok(()),
        }
    }

    fn runtime_bootstrap_identity(&self) -> Result<PolicyIdentity, PolicyIdentityError> {
        let identity = PolicyIdentity::new(
            CANIC_MEMORY_BOOTSTRAP_POLICY_NAME,
            CANIC_MEMORY_BOOTSTRAP_POLICY_VERSION,
        )?;
        let Some(participant) = &self.admission else {
            return Ok(identity);
        };
        let mut hash = Sha256::new();
        hash.update(b"canic:memory-admission:v1");
        hash.update((participant.identity.name().len() as u64).to_be_bytes());
        hash.update(participant.identity.name().as_bytes());
        hash.update(participant.identity.version().to_be_bytes());
        if let Some(configuration) = participant.identity.configuration_digest() {
            hash.update([1]);
            hash.update(configuration);
        } else {
            hash.update([0]);
        }
        Ok(identity.with_configuration_digest(hash.finalize().into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_bootstrap_policy_has_explicit_v1_identity() {
        assert_eq!(
            CanicMemoryManagerPolicy::new()
                .runtime_bootstrap_identity()
                .unwrap(),
            PolicyIdentity::new("canic.memory-bootstrap-policy", 1).unwrap()
        );
    }
}
