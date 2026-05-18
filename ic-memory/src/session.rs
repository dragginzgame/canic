use crate::{
    declaration::AllocationDeclaration, key::StableKey, slot::AllocationSlotDescriptor,
    substrate::StorageSubstrate,
};
use serde::{Deserialize, Serialize};

///
/// ValidatedAllocations
///
/// Allocation declarations accepted by policy and historical ledger validation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidatedAllocations {
    /// Committed generation that validated these allocations.
    pub generation: u64,
    /// Validated declarations.
    pub declarations: Vec<AllocationDeclaration>,
}

impl ValidatedAllocations {
    /// Find a validated slot by stable key.
    #[must_use]
    pub fn slot_for(&self, key: &StableKey) -> Option<&AllocationSlotDescriptor> {
        self.declarations
            .iter()
            .find(|declaration| &declaration.stable_key == key)
            .map(|declaration| &declaration.slot)
    }
}

///
/// AllocationSession
///
/// Validated capability required before opening allocation slots.
pub struct AllocationSession<S: StorageSubstrate> {
    substrate: S,
    validated: ValidatedAllocations,
}

impl<S: StorageSubstrate> AllocationSession<S> {
    /// Construct a session from a substrate and validated allocation set.
    #[must_use]
    pub const fn new(substrate: S, validated: ValidatedAllocations) -> Self {
        Self {
            substrate,
            validated,
        }
    }

    /// Borrow the validated allocation set.
    #[must_use]
    pub const fn validated(&self) -> &ValidatedAllocations {
        &self.validated
    }

    /// Open an allocation by stable key.
    pub fn open(
        &self,
        key: &StableKey,
    ) -> Result<S::MemoryHandle, AllocationSessionError<S::Error>> {
        let slot = self
            .validated
            .slot_for(key)
            .ok_or_else(|| AllocationSessionError::UnknownStableKey(key.clone()))?;
        self.substrate
            .open_slot(slot)
            .map_err(AllocationSessionError::Substrate)
    }
}

///
/// AllocationSessionError
///
/// Failure to open through a validated allocation session.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum AllocationSessionError<E> {
    /// Stable key was not part of the validated allocation snapshot.
    #[error("stable key '{0}' was not validated for this allocation session")]
    UnknownStableKey(StableKey),
    /// Storage substrate failed to open the validated slot.
    #[error("storage substrate failed to open allocation slot")]
    Substrate(E),
}
