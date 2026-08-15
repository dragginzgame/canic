# Canic 0.102 Template Manifest Ops Leaves

Date: 2026-08-15

## Status

This evidence-only B1 ledger closes all thirteen live
`TemplateManifestOpsError` variants shared by the root bootstrap buffer and the
separate Wasm Store. It assigns no number and changes no runtime behavior.

Three variants reuse already-qualified Store identities. The other ten add
exact meanings. The aggregate conversion and current four broad `ErrorCode`
groups receive no identity.

## Approved Manifest Authority

| Exact candidate | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `WASM_STORE_APPROVED_MANIFEST_MISSING` | `ApprovedManifestMissing` | `Unavailable` / root release-set projection | self | Publish or restore the exact admitted role's approved manifest before resolving it |
| `WASM_STORE_APPROVED_MANIFEST_CONFLICT` | `ApprovedManifestConflict` | `Invariant` / root release-set projection | self | Preserve the conflicting rows and repair the one-role/one-manifest authority; do not select one arbitrarily |

These feature-gated variants are root-control-plane authority failures, not
application role lookup errors. The immutable release-set role remains the
request/status correlation owner and disappears from the compact diagnostic.

## Chunk And Payload Integrity

| Exact candidate or disposition | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `WASM_STORE_CHUNK_SET_MISSING` | `TemplateChunkSetMissing` | `NotFound` / Store release inventory | self | Prepare or restore the exact release before reading/publishing chunks |
| `WASM_STORE_CHUNK_MISSING` | `TemplateChunkMissing` | `NotFound` / Store chunk inventory | self | Publish the missing exact chunk; never substitute another index or release |
| reuse `WASM_STORE_CHUNK_SET_EMPTY` | `TemplateChunkSetEmpty` | `Invariant` / canonical chunk set | self | Submit/restage at least one chunk for the exact release |
| `WASM_STORE_PAYLOAD_HASH_MISMATCH` | `PayloadHashMismatch` | `Invariant` / canonical payload integrity | self | Preserve the chunk set and protected manifest; recompute or restage rather than accepting different bytes |
| `WASM_STORE_PAYLOAD_SIZE_MISMATCH` | `PayloadSizeMismatch` | `Invariant` / canonical payload accounting | self | Preserve the chunk set and repair its exact payload-size authority |
| reuse `WASM_STORE_CHUNK_INDEX_OVERFLOW` | `ChunkIndexOverflow` | `ResourceExhausted` / chunk index representation | self | Rebuild the release within the bounded `u32` chunk-index contract |
| `WASM_STORE_CHUNK_INDEX_OUT_OF_RANGE` | `TemplateChunkIndexOutOfRange` | `InvalidInput` / chunk selection | self | Select an index present in the exact release's canonical chunk set |
| reuse `WASM_STORE_CHUNK_HASH_MISMATCH` | `TemplateChunkHashMismatch` | `Invariant` / chunk integrity | self | Preserve the observed and approved evidence; never install or publish mismatched bytes |

The empty-set identity remains shared between caller-supplied preparation and
protected release traversal because both require restaging the same exact
release with a nonempty canonical chunk set. The request or guarded release
status owns which boundary observed it; the diagnostic does not infer that
authority.

## Store Capacity

| Exact candidate | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `WASM_STORE_BYTE_CAPACITY_EXCEEDED` | `WasmStoreCapacityExceeded` | `ResourceExhausted` / encoded Store bytes | self | Free retained Store capacity or publish a smaller exact release after inspecting the canonical projection |
| `WASM_STORE_TEMPLATE_CAPACITY_EXCEEDED` | `WasmStoreTemplateLimitExceeded` | `ResourceExhausted` / distinct-template count | self | Remove an eligible retained template or increase the immutable Store limit before retry |
| `WASM_STORE_VERSION_CAPACITY_EXCEEDED` | `WasmStoreVersionLimitExceeded` | `ResourceExhausted` / per-template retained versions | self | Reclaim an eligible version or increase the immutable per-template limit before retry |

`WASM_STORE_BYTE_CAPACITY_EXCEEDED` is also the exact meaning for publication
workflow capacity rejection. The ops and workflow producers share the same
encoded-byte authority and retry action; they do not receive layer-specific
wrapper codes.

The rejected canonical byte projection is not reconstructible from aggregate
status when an existing encoded entry is replaced. Before B4 removes the prose,
the exact request must therefore own a guarded
`WasmStorePublicationCapacityProjectionResponse` calculated by the same ops
path. It includes the rejected projected Store bytes and effective limit. The
workflow may add its same-operation remaining-byte observation. This is one
request-scoped capacity authority, not a global last-error slot.

## Dynamic Public Context

Slices 2 and 3 of
[dynamic-public-context.md](dynamic-public-context.md) classify every current
field. Release keys, roles and indices are caller-derived from the exact
request or protected manifest. Configured maxima remain in guarded Store
status. Only the rejected canonical byte projection is caller-required but
unowned, and it gains the narrow owner above.

## Reconciliation

All thirteen variants have one disposition. Ten add exact meanings and three
reuse existing identities. No variant is sediment, no wrapper receives a code
and no projection is added.

The qualified semantic set moves from 2,746 to 2,756 exact candidates. The 31
safe projections are unchanged, producing 2,787 current symbolic identities.

## Required Tests

- exhaustive typed mapping for all feature combinations;
- approved-manifest absence versus duplicate authority;
- missing set, missing chunk and out-of-range chunk remain distinct;
- empty set, hash mismatch and size mismatch remain distinct;
- index overflow is never treated as an ordinary missing chunk;
- byte, template-count and per-template-version capacity remain distinct;
- canonical rejected-byte projection is computed by the same checked path as
  admission and is bound to the exact request;
- replacing an existing encoded entry proves aggregate status alone cannot be
  used as the rejected projection; and
- no compact identity is selected from `Display`, a release key or a broad
  current `ErrorCode`.

## Next Slice

Close `PublicationWorkflowError`, expanding every `InvalidState(String)`
construction into its exact binding, GC, reclamation, deletion or release
authority rather than assigning a generic publication-state code.
