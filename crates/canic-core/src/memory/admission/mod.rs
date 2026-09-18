//! Module: memory::admission
//!
//! Responsibility: seal one composed consumer admission hook before Canic memory bootstrap.
//! Does not own: allocation grants, placement, memory handles or lifecycle restoration.
//! Boundary: static registration supplies policy identity and a fallible pre-commit callback.

use crate::memory::registry::MemoryRegistryError;
use ic_memory::BootstrapAdmission;
use std::sync::Mutex;

pub use ic_memory::PolicyIdentity;

/// One host-owned admission over the complete sealed declaration snapshot.
///
/// The identity must cover the callback's semantics and configuration. The callback
/// must not open memory or start a second bootstrap. Compose multiple consumers
/// inside this single callback; return the original typed failure on rejection.
#[derive(Clone, Debug)]
pub struct MemoryBootstrapAdmission {
    pub(super) identity: PolicyIdentity,
    pub(super) prepare: fn(&mut BootstrapAdmission<'_>) -> Result<(), MemoryRegistryError>,
}

impl MemoryBootstrapAdmission {
    /// Bind the callback to its semantic identity, including any configuration digest.
    #[must_use]
    pub const fn new(
        identity: PolicyIdentity,
        prepare: fn(&mut BootstrapAdmission<'_>) -> Result<(), MemoryRegistryError>,
    ) -> Self {
        Self { identity, prepare }
    }
}

#[derive(Default)]
struct Registration {
    participant: Option<MemoryBootstrapAdmission>,
    sealed: bool,
}

impl Registration {
    fn register(
        &mut self,
        participant: MemoryBootstrapAdmission,
    ) -> Result<(), MemoryRegistryError> {
        if self.sealed {
            return Err(MemoryRegistryError::AdmissionRegistrationSealed);
        }
        if self.participant.is_some() {
            return Err(MemoryRegistryError::AdmissionAlreadyRegistered);
        }
        self.participant = Some(participant);
        Ok(())
    }

    fn seal(&mut self) -> Option<MemoryBootstrapAdmission> {
        self.sealed = true;
        self.participant.clone()
    }
}

static ADMISSION: Mutex<Registration> = Mutex::new(Registration {
    participant: None,
    sealed: false,
});

/// Macro registration boundary; applications should use `memory_bootstrap_admission!`.
///
/// # Errors
/// Rejects duplicate or late registration and poisoned registry access.
#[doc(hidden)]
pub fn register(participant: MemoryBootstrapAdmission) -> Result<(), MemoryRegistryError> {
    ADMISSION
        .lock()
        .map_err(|_| MemoryRegistryError::AdmissionRegistryPoisoned)?
        .register(participant)
}

pub(super) fn seal() -> Result<Option<MemoryBootstrapAdmission>, MemoryRegistryError> {
    Ok(ADMISSION
        .lock()
        .map_err(|_| MemoryRegistryError::AdmissionRegistryPoisoned)?
        .seal())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn participant() -> MemoryBootstrapAdmission {
        MemoryBootstrapAdmission::new(
            PolicyIdentity::new("test.admission", 1).unwrap(),
            |_| Ok(()),
        )
    }

    #[test]
    fn registration_is_single_and_sealed_even_without_a_participant() {
        let mut registration = Registration::default();
        registration.register(participant()).unwrap();
        assert!(matches!(
            registration.register(participant()),
            Err(MemoryRegistryError::AdmissionAlreadyRegistered)
        ));
        assert!(registration.seal().is_some());
        assert!(registration.seal().is_some());
        assert!(matches!(
            registration.register(participant()),
            Err(MemoryRegistryError::AdmissionRegistrationSealed)
        ));
        let mut absent = Registration::default();
        assert!(absent.seal().is_none());
        assert!(matches!(
            absent.register(participant()),
            Err(MemoryRegistryError::AdmissionRegistrationSealed)
        ));
    }
}
