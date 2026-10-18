//! Module: memory::admission
//!
//! Responsibility: describe one explicit consumer admission callback for Canic memory bootstrap.
//! Does not own: allocation grants, placement, memory handles or lifecycle restoration.
//! Boundary: host composition supplies policy identity and a fallible pre-commit callback.

use crate::memory::registry::MemoryRegistryError;
use ic_memory::BootstrapAdmission;

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
