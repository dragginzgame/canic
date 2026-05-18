//! Persistent allocation-governance primitives for Internet Computer stable
//! memory.
//!
//! The crate models durable ownership as `stable_key -> allocation_slot
//! forever`. It intentionally does not own framework namespaces, controller
//! authorization, endpoint lifecycle, schema migrations, or Canic-specific
//! memory ID policy.

pub mod bootstrap;
pub mod declaration;
pub mod diagnostics;
pub mod generation;
pub mod key;
pub mod ledger;
pub mod physical;
pub mod policy;
pub mod schema;
pub mod session;
pub mod slot;
pub mod substrate;
pub mod validation;

pub use bootstrap::{
    AllocationBootstrap, BootstrapCommit, BootstrapError, BootstrapReservationError,
    BootstrapRetirementError,
};
pub use declaration::{
    AllocationDeclaration, DeclarationCollector, DeclarationSnapshot, DeclarationSnapshotError,
};
pub use diagnostics::{DiagnosticExport, DiagnosticGeneration, DiagnosticRecord};
pub use generation::{GenerationCommit, GenerationMutation, StagedGeneration};
pub use key::{StableKey, StableKeyError};
pub use ledger::{
    AllocationHistory, AllocationLedger, AllocationRecord, AllocationReservationError,
    AllocationRetirement, AllocationRetirementError, AllocationState,
    CURRENT_LEDGER_SCHEMA_VERSION, CURRENT_PHYSICAL_FORMAT_ID, GenerationRecord, LedgerCodec,
    LedgerCommitError, LedgerCommitStore, LedgerCompatibility, LedgerCompatibilityError,
    LedgerIntegrityError, SchemaMetadataRecord,
};
pub use physical::{
    CommitRecoveryError, CommitSlotDiagnostic, CommitStoreDiagnostic, CommittedGenerationBytes,
    DualCommitStore,
};
pub use policy::{AllocationPolicy, NamespaceAuthority, RangeAuthority};
pub use schema::{SchemaMetadata, SchemaMetadataError};
pub use session::{AllocationSession, ValidatedAllocations};
pub use slot::{AllocationSlot, AllocationSlotDescriptor};
pub use substrate::{LedgerAnchor, StorageSubstrate};
pub use validation::{AllocationValidationError, validate_allocations};
