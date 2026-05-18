use crate::slot::AllocationSlotDescriptor;

///
/// LedgerAnchor
///
/// Substrate-defined location of the allocation ledger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerAnchor {
    /// Substrate identifier.
    pub substrate: String,
    /// Ledger storage descriptor.
    pub descriptor: AllocationSlotDescriptor,
}

///
/// StorageSubstrate
///
/// Physical storage provider for the ledger anchor and allocation slots.
pub trait StorageSubstrate {
    /// Native slot type accepted by this substrate.
    type Slot;
    /// Ledger memory handle.
    type LedgerMemory;
    /// Allocation memory handle.
    type MemoryHandle;
    /// Substrate error type.
    type Error;

    /// Open the ledger anchor.
    fn open_ledger(&self) -> Result<Self::LedgerMemory, Self::Error>;

    /// Open an allocation slot after validation has produced a session.
    fn open_slot(&self, slot: &AllocationSlotDescriptor)
    -> Result<Self::MemoryHandle, Self::Error>;

    /// Describe a native slot as a durable allocation slot descriptor.
    fn describe_slot(&self, slot: &Self::Slot) -> AllocationSlotDescriptor;
}
