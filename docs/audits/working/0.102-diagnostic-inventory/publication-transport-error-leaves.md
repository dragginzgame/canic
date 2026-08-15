# Canic 0.102 Publication Transport Error Leaves

Date: 2026-08-15

## Status

This evidence-only B1 ledger closes both production
`PublicationWorkflowError::TransportUnavailable` constructions: management
`stored_chunks` and management `upload_chunk`. It expands each static surface
through every reachable typed `IcInfraError` leaf at the pinned `ic-cdk 0.20.2`
boundary. It assigns no number and changes no runtime behavior.

The aggregate wrapper receives no identity. Each surface has eleven exact
leaves: request encoding, insufficient liquid cycles, local call-performance
failure, six recognized rejection classes, unknown rejection code and response
decoding. The two surfaces therefore add 22 exact meanings and no projection.

## Typed Source Boundary

Both management adapters have the same typed journey:

1. `Call::with_arg` can return `IcInfraError::Candid`;
2. `Call::execute` can return insufficient liquid cycles,
   `CallPerformFailed` or `CallRejected`;
3. `CallRejected` has six recognized `RejectCode` values or an unrecognized
   raw numeric value; and
4. `candid_tuple` can return `IcInfraError::CandidDecode`.

`MgmtOps` currently converts that shape to formatted `InternalError` before the
publication workflow sees it. B4 must preserve the finite typed cause to the
surface-specific classifier. Parsing the formatted error is forbidden.

## Stored-Chunks Observation

| Exact candidate | Producer function/typed branch | Typed source | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `WASM_STORE_STORED_CHUNKS_REQUEST_ENCODE_FAILED` | `PublicationStoreSnapshot::ensure_stored_chunk_hashes` / `IcInfraError::Candid` | request Candid encoder | `IC_PLATFORM_PROTOCOL_INVALID` | Fix the maintained management DTO; unchanged runtime retry cannot help |
| `WASM_STORE_STORED_CHUNKS_LIQUID_CYCLES_INSUFFICIENT` | `PublicationStoreSnapshot::ensure_stored_chunk_hashes` / `IcInfraError::InsufficientLiquidCycles` | insufficient liquid cycles | self | Top up the root before retrying the exact observation |
| `WASM_STORE_STORED_CHUNKS_CALL_PERFORM_FAILED` | `PublicationStoreSnapshot::ensure_stored_chunk_hashes` / `IcInfraError::CallPerformFailed` | local call performance failure | `IC_PLATFORM_EFFECT_FAILED` | Retry only through the bounded publication attempt |
| `WASM_STORE_STORED_CHUNKS_REJECTED_SYSTEM_FATAL` | `PublicationStoreSnapshot::ensure_stored_chunk_hashes` / `RejectCode::SysFatal` | `SysFatal` | `IC_PLATFORM_EFFECT_FAILED` | Stop blind retry and inspect platform state |
| `WASM_STORE_STORED_CHUNKS_REJECTED_SYSTEM_TRANSIENT` | `PublicationStoreSnapshot::ensure_stored_chunk_hashes` / `RejectCode::SysTransient` | `SysTransient` | self | Bounded retry through the same publication attempt |
| `WASM_STORE_STORED_CHUNKS_REJECTED_DESTINATION_INVALID` | `PublicationStoreSnapshot::ensure_stored_chunk_hashes` / `RejectCode::DestinationInvalid` | `DestinationInvalid` | self | Re-observe the exact adopted Store; this read rejection is not physical-deletion evidence |
| `WASM_STORE_STORED_CHUNKS_REJECTED_BY_CANISTER` | `PublicationStoreSnapshot::ensure_stored_chunk_hashes` / `RejectCode::CanisterReject` | `CanisterReject` | `IC_PLATFORM_EFFECT_FAILED` | Inspect the maintained management/Store contract without exposing reject prose |
| `WASM_STORE_STORED_CHUNKS_REJECTED_CANISTER_ERROR` | `PublicationStoreSnapshot::ensure_stored_chunk_hashes` / `RejectCode::CanisterError` | `CanisterError` | `IC_PLATFORM_EFFECT_FAILED` | Inspect target health and retry only through the owning attempt |
| `WASM_STORE_STORED_CHUNKS_REJECTED_SYSTEM_UNKNOWN` | `PublicationStoreSnapshot::ensure_stored_chunk_hashes` / `RejectCode::SysUnknown` | `SysUnknown` | `IC_PLATFORM_EFFECT_FAILED` | Preserve the attempt and reconcile outcome before retry |
| `WASM_STORE_STORED_CHUNKS_REJECT_CODE_UNKNOWN` | `PublicationStoreSnapshot::ensure_stored_chunk_hashes` / `IcInfraError::CallRejected` unknown-code branch | unrecognized raw reject code | `IC_PLATFORM_EFFECT_FAILED` | Fail closed and review the pinned CDK/IC boundary |
| `WASM_STORE_STORED_CHUNKS_RESPONSE_DECODE_FAILED` | `PublicationStoreSnapshot::ensure_stored_chunk_hashes` / `IcInfraError::CandidDecode` | response Candid decoder | `IC_PLATFORM_PROTOCOL_INVALID` | Preserve response context and repair the qualified interface |

The operation is read-only, but its result is publication authority. A stale or
fabricated chunk set is never substituted when observation fails.

## Upload-Chunk Effect

| Exact candidate | Producer function/typed branch | Typed source | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `WASM_STORE_UPLOAD_CHUNK_REQUEST_ENCODE_FAILED` | `WasmStorePublicationWorkflow::ensure_target_store_upload_cache` / `IcInfraError::Candid` | request Candid encoder | `IC_PLATFORM_PROTOCOL_INVALID` | Fix the maintained management DTO before retry |
| `WASM_STORE_UPLOAD_CHUNK_LIQUID_CYCLES_INSUFFICIENT` | `WasmStorePublicationWorkflow::ensure_target_store_upload_cache` / `IcInfraError::InsufficientLiquidCycles` | insufficient liquid cycles | self | Top up the root and exact-retry the same protected chunk attempt |
| `WASM_STORE_UPLOAD_CHUNK_CALL_PERFORM_FAILED` | `WasmStorePublicationWorkflow::ensure_target_store_upload_cache` / `IcInfraError::CallPerformFailed` | local call performance failure | `IC_PLATFORM_EFFECT_FAILED` | Reconcile the retained attempt before retrying the effect |
| `WASM_STORE_UPLOAD_CHUNK_REJECTED_SYSTEM_FATAL` | `WasmStorePublicationWorkflow::ensure_target_store_upload_cache` / `RejectCode::SysFatal` | `SysFatal` | `IC_PLATFORM_EFFECT_FAILED` | Stop blind retry and inspect platform state |
| `WASM_STORE_UPLOAD_CHUNK_REJECTED_SYSTEM_TRANSIENT` | `WasmStorePublicationWorkflow::ensure_target_store_upload_cache` / `RejectCode::SysTransient` | `SysTransient` | self | Bounded exact retry through the same attempt |
| `WASM_STORE_UPLOAD_CHUNK_REJECTED_DESTINATION_INVALID` | `WasmStorePublicationWorkflow::ensure_target_store_upload_cache` / `RejectCode::DestinationInvalid` | `DestinationInvalid` | self | Re-observe the exact target Store; never infer successful upload |
| `WASM_STORE_UPLOAD_CHUNK_REJECTED_BY_CANISTER` | `WasmStorePublicationWorkflow::ensure_target_store_upload_cache` / `RejectCode::CanisterReject` | `CanisterReject` | `IC_PLATFORM_EFFECT_FAILED` | Inspect the typed Store/management contract; discard reject prose |
| `WASM_STORE_UPLOAD_CHUNK_REJECTED_CANISTER_ERROR` | `WasmStorePublicationWorkflow::ensure_target_store_upload_cache` / `RejectCode::CanisterError` | `CanisterError` | `IC_PLATFORM_EFFECT_FAILED` | Preserve attempt identity and inspect target health |
| `WASM_STORE_UPLOAD_CHUNK_REJECTED_SYSTEM_UNKNOWN` | `WasmStorePublicationWorkflow::ensure_target_store_upload_cache` / `RejectCode::SysUnknown` | `SysUnknown` | `IC_PLATFORM_EFFECT_FAILED` | Treat the effect outcome as unresolved until exact reconciliation |
| `WASM_STORE_UPLOAD_CHUNK_REJECT_CODE_UNKNOWN` | `WasmStorePublicationWorkflow::ensure_target_store_upload_cache` / `IcInfraError::CallRejected` unknown-code branch | unrecognized raw reject code | `IC_PLATFORM_EFFECT_FAILED` | Fail closed and review the pinned adapter surface |
| `WASM_STORE_UPLOAD_CHUNK_RESPONSE_DECODE_FAILED` | `WasmStorePublicationWorkflow::ensure_target_store_upload_cache` / `IcInfraError::CandidDecode` | response Candid decoder | `IC_PLATFORM_PROTOCOL_INVALID` | Preserve the attempt and repair the response contract |

Upload is an external effect. Successful-response loss, rejection and local
transport failure are not interchangeable. The publication attempt remains the
recovery authority; a compact diagnostic is never proof that a chunk exists.

## Dynamic Public Context

Slice 12 of
[dynamic-public-context.md](dynamic-public-context.md) classifies all fourteen
current formatted fields. Raw reject messages, Rust type names and dependency
codec prose are discarded. The guarded operation-scoped
`WasmStorePublicationAttemptStatusResponse` binds the exact operation, Store,
release, optional chunk and surface while retaining only the approved numeric
diagnostic, available/required cycles and an unrecognized raw reject number.
Known reject classes need no raw number.

Bootstrap and admin requests must expose a nonzero caller-supplied publication
operation ID before B4 removes prose. The cost-guard reservation ID and
heap-only recent-failure ring are not substitutes for a retrievable effect
authority.

## Reconciliation

Both wrapper constructions are transparent. Their typed source expansion adds
eleven exact meanings per surface: 22 total. Generic IC leaves remain the
lower-layer source taxonomy but are not reused as public publication identity,
because this boundary requires a surface-specific action and operation owner.
No new safe projection is introduced; the existing IC platform projections
remain the explicit masked targets above.

The qualified semantic set moves from 2,823 to 2,845 exact candidates. The 31
safe projections remain unchanged, producing 2,876 current symbolic identities.
Those are the corrected publication-pass checkpoint counts. Later source-to-
ledger reconciliation adds nineteen previously omitted delegated-session
identities; the current whole-program total is maintained by
[ledger-reconciliation.md](ledger-reconciliation.md).

## Required Tests

- exhaustive typed mapping for both surfaces;
- request encode and response decode remain distinct;
- insufficient cycles and local call-performance failure remain distinct;
- all six recognized reject codes plus unknown raw code are exhaustive at the
  pinned dependency boundary;
- raw reject/codec/type prose cannot enter the compact error or host catalog;
- available/required cycles and unknown reject code are retained only by the
  exact guarded publication attempt;
- upload response loss never proves that the chunk is present;
- stored-chunks failure never fabricates an empty or cached observation; and
- no `TransportUnavailable`, broad unavailable or generic IC wrapper identity
  remains selectable.

## Next Slice

Reconcile the now-complete `PublicationWorkflowError` aggregate against the
conversion, dynamic-context, projection and permanent-ledger gates. If no
other live transitive owner is open, close this documentation release batch and
prepare its changelog/push-readiness handoff.
