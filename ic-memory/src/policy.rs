use crate::{key::StableKey, slot::AllocationSlotDescriptor};

///
/// AllocationPolicy
///
/// Framework-supplied rules for whether a key may claim a slot.
pub trait AllocationPolicy {
    /// Policy error type.
    type Error;

    /// Validate a stable key against framework naming rules.
    fn validate_key(&self, key: &StableKey) -> Result<(), Self::Error>;

    /// Validate a stable-key to allocation-slot claim.
    fn validate_slot(
        &self,
        key: &StableKey,
        slot: &AllocationSlotDescriptor,
    ) -> Result<(), Self::Error>;

    /// Validate a reserved stable-key to allocation-slot claim.
    fn validate_reserved_slot(
        &self,
        key: &StableKey,
        slot: &AllocationSlotDescriptor,
    ) -> Result<(), Self::Error>;
}

///
/// NamespaceAuthority
///
/// Policy-owned stable-key namespace ownership.
pub trait NamespaceAuthority {
    /// Return true when this authority owns `key`.
    fn owns(&self, key: &StableKey) -> bool;
}

///
/// RangeAuthority
///
/// Optional substrate-specific range authority validation.
pub trait RangeAuthority {
    /// Range validation error type.
    type Error;

    /// Validate one allocation slot against the authority.
    fn validate_slot(&self, slot: &AllocationSlotDescriptor) -> Result<(), Self::Error>;
}
