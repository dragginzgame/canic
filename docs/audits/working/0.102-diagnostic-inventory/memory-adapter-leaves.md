# Canic 0.102 `ic-memory` Adapter Diagnostic Leaves

Date: 2026-08-15

## Status And Pin

This provisional B1 ledger covers the exact memory runtime surface reached by
`MemoryRegistryOpsError` at immutable Canic baseline `v0.101.53`. It allocates
no numbers.

The audited dependency is `ic-memory 0.12.3`, locked from crates.io with
checksum:

```text
7c66256c945be302111688ebcf1d19d505b272f43f897a62739903b476c65236
```

The dependency owns memory safety and its rich Rust errors. Canic owns the
compact diagnostic protocol. B4 must implement an explicit adapter and must
not format or parse dependency errors into diagnostic identity.

## Reachable Boundary

The current Canic wrapper reaches:

```text
MemoryRegistryError
RuntimeBootstrapError<MemoryRegistryError>
RuntimeDiagnosticError
RuntimeStateError
```

Their recursive known graph contains **131 structural leaves** across Canic
policy, runtime bootstrap/state/construction, declaration registry/snapshot,
range authority, validation/staging, ledger integrity/commit/recovery/payload
and stable-cell storage.

The dependency also exposes `RuntimeOpenError`, `BootstrapError`, reservation
and retirement error families. They are not reached by the current
`MemoryRegistryOpsError` conversion or current memory ops facade and therefore
receive no 0.102 code on this path. A public type existing in the dependency is
not current producer evidence.

## Canic Policy

| Candidate label | Current source | Action and retry |
| --- | --- | --- |
| `MEMORY_DECLARATION_INVALID` | `MemoryRegistryError::InvalidDeclaration` and `RuntimePolicyError::Custom` | Correct linked declarations and rebuild; no unchanged retry |
| `MEMORY_RANGE_AUTHORITY_VIOLATION` | `MemoryRegistryError::RangeAuthorityViolation` and `RuntimePolicyError::Custom` | Put the stable key in its owned ID range; rebuild |

Stable keys, IDs and free-form reasons never enter the compact result. The
current wildcard conversion from future `MemoryManagerSlotError` variants to
`InvalidDeclaration` is not sufficient forward handling; dependency-unknown
mapping is explicit below.

## Bootstrap, Readiness And Runtime State

| Candidate label | Dependency source(s) | Action and retry |
| --- | --- | --- |
| `MEMORY_POLICY_IDENTITY_INVALID` | all five `PolicyIdentityError` variants | Correct the bounded policy identity and rebuild |
| `MEMORY_BOOTSTRAP_DECLARATION_SNAPSHOT_MISMATCH` | `RuntimeBootstrapError::DeclarationSnapshotMismatch` | Reject contradictory bootstrap retry; use the established linked snapshot |
| `MEMORY_BOOTSTRAP_POLICY_IDENTITY_MISMATCH` | `RuntimeBootstrapError::PolicyIdentityMismatch` | Reject contradictory bootstrap retry; use the established policy |
| `MEMORY_LEDGER_RECORD_CAPACITY_EXCEEDED` | `RuntimeBootstrapError::StableCellLedgerWriteTooLarge` | Reduce the bounded ledger record before retry |
| `MEMORY_DIAGNOSTIC_NOT_READY` | `RuntimeDiagnosticError::NotBootstrapped` | Complete bootstrap; diagnostic reads must not mutate readiness |
| `MEMORY_RUNTIME_REENTRANT` | `RuntimeStateError::ReentrantAccess` | End the current borrow/operation before bounded retry |
| `MEMORY_RUNTIME_UNAVAILABLE` | `RuntimeStateError::Unavailable` | Retry only after TLS destruction/lifecycle state changes |
| `MEMORY_RUNTIME_STATE_INVALID` | `RuntimeStateError::InconsistentLifecycle` | Fail closed; inspect runtime lifecycle |
| `MEMORY_BACKING_MEMORY_FOREIGN` | `RuntimeConstructionError::ForeignMemory` | Fail closed; do not overwrite unrecognized stable memory |
| `MEMORY_MANAGER_LAYOUT_UNSUPPORTED` | `RuntimeConstructionError::UnsupportedMemoryManagerVersion` | Use the pinned supported layout; no fallback decoder |

All wrapper variants in `RuntimeBootstrapError` and
`RuntimeDiagnosticError` preserve their nested mapped cause. They receive no
aggregate code.

## Static Declaration Registry

| Candidate label | `StaticMemoryDeclarationError` source | Action and retry |
| --- | --- | --- |
| `MEMORY_DECLARATION_REGISTRY_POISONED` | `RegistryPoisoned` | Stop bootstrap; correct the panicked/poisoned process state |
| `MEMORY_DECLARATION_REGISTRY_SEALED` | `RegistrySealed` | Register only before sealing; reject late declaration |
| `MEMORY_DECLARATION_REGISTRY_REENTRANT` | `ReentrantSealing` | Remove recursive eager registration |
| `MEMORY_DECLARATION_REGISTRY_STATE_INVALID` | `InconsistentLifecycle` | Fail closed; inspect registry lifecycle |
| `MEMORY_DECLARATION_EAGER_INIT_FAILED` | `EagerInitPanicked` | Fix the eager hook; do not continue with a partial snapshot |
| `MEMORY_DECLARATION_FINGERPRINT_ENCODE_FAILED` | `SnapshotFingerprintEncoding` | Fix canonical fingerprint encoding |
| `MEMORY_DECLARATION_AUTHORITY_INVALID` | `InvalidAuthority` | Use a bounded valid linked-code authority name |
| `MEMORY_DECLARATION_AUTHORITY_RESERVED` | `ReservedAuthority` | Do not impersonate the internal authority |
| `MEMORY_DECLARATION_KEY_RESERVED` | `ReservedStableKey` and `RuntimePolicyError::ReservedStableKeyAuthority` | Move the declaration to its owned namespace |

`Declaration` and `Range` are transparent nested cause edges.

## Declaration, Key, Slot And Schema

| Candidate label | Dependency source(s) | Action and retry |
| --- | --- | --- |
| `MEMORY_STABLE_KEY_INVALID` | `StableKeyError`, `DeclarationSnapshotError::Key` | Correct the stable-key grammar and rebuild |
| `MEMORY_SLOT_INVALID` | all `MemoryManagerSlotError::InvalidMemoryManagerId` and range invalid-ID paths | Use a valid non-sentinel MemoryManager ID |
| `MEMORY_SCHEMA_METADATA_INVALID` | `SchemaMetadataError::InvalidVersion` and nested staging/integrity paths | Supply current nonzero schema metadata |
| `MEMORY_DECLARATION_KEY_DUPLICATED` | `DeclarationSnapshotError::DuplicateStableKey` | Declare each stable key exactly once |
| `MEMORY_DECLARATION_SLOT_DUPLICATED` | `DeclarationSnapshotError::DuplicateSlot` | Give each allocation slot one owner |
| `MEMORY_DECLARATION_LABEL_INVALID` | `DeclarationSnapshotError::{EmptyLabel,LabelTooLong,NonAsciiLabel,ControlCharacterLabel}` | Use optional bounded printable ASCII metadata |
| `MEMORY_RUNTIME_FINGERPRINT_INVALID` | `DeclarationSnapshotError::{EmptyRuntimeFingerprint,RuntimeFingerprintTooLong,NonAsciiRuntimeFingerprint,ControlCharacterRuntimeFingerprint}` | Use optional bounded printable ASCII metadata |

The snapshot wrapper routes its key, slot and schema variants to the same
value-owner codes; it does not allocate path duplicates.

## Range Authority

| Candidate label | Dependency source(s) | Action and retry |
| --- | --- | --- |
| `MEMORY_RANGE_INVALID` | `MemoryManagerRangeError::InvalidRange` | Correct reversed/invalid bounds |
| `MEMORY_RANGE_OVERLAP` | `OverlappingRanges` | Give every physical ID one range authority |
| `MEMORY_RANGE_METADATA_INVALID` | `InvalidDiagnosticString` | Use bounded printable range metadata |
| `MEMORY_RANGE_UNCLAIMED_ID` | `UnclaimedId` | Declare authority coverage for the requested ID |
| `MEMORY_RANGE_AUTHORITY_MISMATCH` | `AuthorityMismatch` | Use the authority that owns the ID |
| `MEMORY_RANGE_MODE_MISMATCH` | `ModeMismatch` | Use the configured Reserved/Allowed mode |
| `MEMORY_RANGE_COVERAGE_MISSING` | `MissingCoverage` | Complete the required authority coverage |
| `MEMORY_RANGE_OUTSIDE_COVERAGE` | `RangeOutsideCoverageTarget` | Keep declarations inside the governed target |

`MemoryManagerRangeAuthorityError::Range` and `Slot` preserve the range/slot
value-owner codes above.

## Ledger And Stable Cell

| Candidate label | Dependency source(s) | Action and retry |
| --- | --- | --- |
| `MEMORY_LEDGER_INTEGRITY_INVALID` | all 22 `LedgerIntegrityError` variants | Fail closed; inspect the durable allocation history |
| `MEMORY_LEDGER_GENERATION_MISMATCH` | `LedgerCommitError::PhysicalLogicalGenerationMismatch` | Reject disagreement between physical and logical authority |
| `MEMORY_LEDGER_CODEC_FAILED` | `LedgerCommitError::Codec` | Stop bootstrap/commit; fix the pinned codec path |
| `MEMORY_LEDGER_NO_VALID_GENERATION` | `CommitRecoveryError::NoValidGeneration` | Initialize only through the admitted genesis path; never fabricate recovery |
| `MEMORY_LEDGER_COMMIT_SLOT_INVALID` | `InvalidCommitSlots` | Fail closed on corrupt protected slots |
| `MEMORY_LEDGER_GENERATION_AMBIGUOUS` | `AmbiguousGeneration` | Fail closed; do not choose between conflicting equal generations |
| `MEMORY_LEDGER_GENERATION_EXHAUSTED` | `CommitRecoveryError::GenerationOverflow` and `AllocationStageError::GenerationOverflow` | Stop commits; operator intervention is required |
| `MEMORY_LEDGER_GENERATION_CONFLICT` | `UnexpectedGeneration` | Commit only the exact next protected generation |
| `MEMORY_LEDGER_PAYLOAD_INVALID` | all six `LedgerPayloadEnvelopeError` variants | Reject corrupt, foreign or unsupported current-format payload bytes |
| `MEMORY_LEDGER_RECORD_INVALID` | `StableCellLedgerError::Record` | Reject undecodable ledger records |
| `MEMORY_LEDGER_STABLE_CELL_INVALID` | all four `StableCellPayloadError` variants | Reject corrupt/unsupported stable-cell envelopes |

`LedgerCommitError::{Recovery,PayloadEnvelope,Integrity}` and
`StableCellLedgerError::Payload` are transparent edges. Opaque codec and CBOR
text may be retained only as bounded operational context in an approved owner;
it never enters the code or controls recovery.

## Allocation Validation And Staging

| Candidate label | Dependency source(s) | Action and retry |
| --- | --- | --- |
| `MEMORY_ALLOCATION_STABLE_KEY_CONFLICT` | `AllocationValidationError::StableKeySlotConflict` and matching staging conflict | Preserve the historical slot; correct the declaration |
| `MEMORY_ALLOCATION_SLOT_CONFLICT` | `AllocationValidationError::SlotStableKeyConflict` and matching staging conflict | Preserve the historical key; correct the declaration |
| `MEMORY_ALLOCATION_RETIRED` | `AllocationValidationError::RetiredAllocation` and matching staging conflict | Never resurrect the retired allocation |
| `MEMORY_ALLOCATION_ACTIVE_CONFLICT` | `AllocationValidationError::UnexpectedActiveAllocationConflict` | Fail closed; inspect declaration/ledger disagreement |
| `MEMORY_DECLARATION_METADATA_MISSING` | `RuntimePolicyError::MissingDeclarationMetadata` | Restore the linked declaration metadata |
| `MEMORY_ALLOCATION_STAGE_STALE` | `AllocationStageError::StaleValidatedAllocations` | Revalidate against the exact current generation |
| `MEMORY_ALLOCATION_COUNT_EXCEEDED` | `AllocationStageError::TooManyDeclarations` | Reduce declarations to the durable diagnostic bound |

Validation wrappers preserve ledger-integrity, declaration-snapshot and
runtime-policy causes. Staging schema/generation/conflict variants reuse the
value, ledger-generation and allocation-conflict codes above.

## Non-Exhaustive Dependency Boundary

Twenty reachable dependency enums are `#[non_exhaustive]`. B4 requires one
explicit Canic-owned unknown code at each match boundary:

| Exact identity | Dependency match boundary |
| --- | --- |
| `MEMORY_POLICY_IDENTITY_UNKNOWN` | wildcard of `PolicyIdentityError` |
| `MEMORY_RUNTIME_BOOTSTRAP_UNKNOWN` | wildcard of `RuntimeBootstrapError<MemoryRegistryError>` |
| `MEMORY_RUNTIME_DIAGNOSTIC_UNKNOWN` | wildcard of `RuntimeDiagnosticError` |
| `MEMORY_RUNTIME_STATE_UNKNOWN` | wildcard of `RuntimeStateError` |
| `MEMORY_RUNTIME_CONSTRUCTION_UNKNOWN` | wildcard of `RuntimeConstructionError` |
| `MEMORY_STATIC_DECLARATION_UNKNOWN` | wildcard of `StaticMemoryDeclarationError` |
| `MEMORY_DECLARATION_SNAPSHOT_UNKNOWN` | wildcard of `DeclarationSnapshotError` |
| `MEMORY_RANGE_AUTHORITY_UNKNOWN` | wildcard of `MemoryManagerRangeAuthorityError` |
| `MEMORY_RANGE_UNKNOWN` | wildcard of `MemoryManagerRangeError` |
| `MEMORY_SLOT_UNKNOWN` | wildcard of `MemoryManagerSlotError` |
| `MEMORY_LEDGER_INTEGRITY_UNKNOWN` | wildcard of `LedgerIntegrityError` |
| `MEMORY_SCHEMA_METADATA_UNKNOWN` | wildcard of `SchemaMetadataError` |
| `MEMORY_LEDGER_COMMIT_UNKNOWN` | wildcard of `LedgerCommitError` |
| `MEMORY_COMMIT_RECOVERY_UNKNOWN` | wildcard of `CommitRecoveryError` |
| `MEMORY_LEDGER_PAYLOAD_UNKNOWN` | wildcard of `LedgerPayloadEnvelopeError` |
| `MEMORY_STABLE_CELL_LEDGER_UNKNOWN` | wildcard of `StableCellLedgerError` |
| `MEMORY_STABLE_CELL_PAYLOAD_UNKNOWN` | wildcard of `StableCellPayloadError` |
| `MEMORY_ALLOCATION_VALIDATION_UNKNOWN` | wildcard of `AllocationValidationError` |
| `MEMORY_RUNTIME_POLICY_UNKNOWN` | wildcard of `RuntimePolicyError` |
| `MEMORY_ALLOCATION_STAGE_UNKNOWN` | wildcard of `AllocationStageError` |

These unknown leaves are deliberately boundary-specific so a dependency
upgrade cannot silently reclassify a new variant under an unrelated retry
policy. They are safe public identities as written, always fail closed, and
require adapter review before the new dependency can qualify.

## Current Count

The 131 known structural dependency/Canic leaves group by identical semantic
owner and action into **54 known exact candidates**. The 20 required
non-exhaustive boundary leaves bring this pass to **74 exact candidates**.

Every candidate label above is safe as its own public projection because it
contains no key, ID, range, generation, policy identity, fingerprint, codec
text or stored bytes. This pass introduces no additional broad projection.

## Required Tests

- an exhaustive adapter test for every known pinned 0.12.3 variant;
- one exercised wildcard/unknown mapping per non-exhaustive owner, using an
  adapter seam where external construction is impossible;
- dependency wrappers preserve nested Canic codes without path duplication;
- foreign/unsupported/corrupt memory is never overwritten or decoded through a
  fallback;
- ambiguous/missing/invalid commit authority never becomes successful
  recovery;
- declaration and range conflicts retain historical authority;
- diagnostic-not-ready is read-only and distinct from runtime unavailability;
- no dependency `Display` text enters the compact result or recovery logic;
  and
- changing the pinned dependency version/checksum requires an explicit adapter
  inventory update and focused qualification.
