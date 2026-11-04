//! Module: memory::policy
//!
//! Responsibility: compose Canic memory bootstrap admission and policy identity.
//! Does not own: Storage allocation, stable schemas, or diagnostics rendering.
//! Boundary: memory bootstrap passes this policy into `ic-storage` validation.

use crate::memory::{admission::MemoryBootstrapAdmission, registry::MemoryRegistryError};
use ic_storage::{PolicyIdentity, RuntimeBootstrapPolicy, StableKey};
use sha2::{Digest as _, Sha256};

const CANIC_MEMORY_BOOTSTRAP_POLICY_NAME: &str = "canic.memory-bootstrap-policy";
const CANIC_MEMORY_BOOTSTRAP_POLICY_VERSION: u32 = 1;

///
/// CanicStoragePolicy
///
/// Canic policy adapter for the `ic-storage` directory and namespace admission.
/// Owned by memory policy and supplied to Storage bootstrap.
///

#[derive(Clone, Debug, Default)]
pub struct CanicStoragePolicy {
    admission: Option<MemoryBootstrapAdmission>,
}

impl CanicStoragePolicy {
    /// Use Canic's bootstrap identity without a consumer admission callback.
    #[must_use]
    pub const fn new() -> Self {
        Self { admission: None }
    }

    /// Compose consumer admission with Canic's bootstrap identity.
    #[must_use]
    pub const fn with_admission(mut self, admission: MemoryBootstrapAdmission) -> Self {
        self.admission = Some(admission);
        self
    }
}

impl RuntimeBootstrapPolicy for CanicStoragePolicy {
    type Error = MemoryRegistryError;

    fn validate_allocation(&self, _key: &StableKey) -> Result<(), Self::Error> {
        Ok(())
    }

    fn prepare_bootstrap(
        &self,
        admission: &mut ic_storage::BootstrapAdmission<'_>,
    ) -> Result<(), Self::Error> {
        match &self.admission {
            Some(participant) => (participant.prepare)(admission),
            None => Ok(()),
        }
    }

    fn runtime_bootstrap_identity(&self) -> PolicyIdentity {
        let mut hash = Sha256::new();
        hash.update(CANIC_MEMORY_BOOTSTRAP_POLICY_NAME.as_bytes());
        hash.update(CANIC_MEMORY_BOOTSTRAP_POLICY_VERSION.to_be_bytes());
        match &self.admission {
            Some(participant) => {
                hash.update([1]);
                hash.update(participant.identity);
            }
            None => hash.update([0]),
        }
        PolicyIdentity::new(hash.finalize().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_bootstrap_policy_binds_optional_admission_token() {
        let base = CanicStoragePolicy::new().runtime_bootstrap_identity();
        let policy = |token| {
            CanicStoragePolicy::new()
                .with_admission(MemoryBootstrapAdmission::new(token, |_| Ok(())))
                .runtime_bootstrap_identity()
        };
        assert_eq!(base, CanicStoragePolicy::new().runtime_bootstrap_identity());
        assert_ne!(base, policy([0; 32]));
        let token = [7; 32];
        assert_eq!(policy(token), policy(token));
        for index in 0..32 {
            let mut changed = token;
            changed[index] ^= 1;
            assert_ne!(policy(token), policy(changed));
        }
    }
}
