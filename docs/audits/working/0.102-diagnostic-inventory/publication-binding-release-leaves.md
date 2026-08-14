# Canic 0.102 Publication Binding And Release Leaves

Date: 2026-08-14

## Status

This evidence-only B1 ledger closes the twelve non-transport production
constructions in `PublicationWorkflowError`'s binding, release and chunk
owners. One combined release-reconciliation construction expands to two
independent predicates, producing thirteen semantic dispositions. It assigns
no number and changes no runtime behavior.

Seven exact meanings are newly qualified. Six dispositions reuse an exact
Store identity. The two management-transport constructions and all 56 GC
lifecycle constructions remain explicitly open; this document therefore does
not claim that the aggregate enum is closed.

## Publication Binding Authority

| Exact candidate or disposition | Source predicate | Public projection | Action and retry |
| --- | --- | --- | --- |
| `WASM_STORE_SOLE_ACTIVE_PUBLICATION_BINDING_REQUIRED` | requested binding is not the sole active slot, or a detached/retired slot remains | self | Reconcile the controller-guarded publication slots; never select another Store implicitly |
| reuse `WASM_STORE_SINGLE_ADOPTED_STORE_REQUIRED` | initial pin or bootstrap catalog observes zero or multiple root-owned Stores | self | Reconcile the exact adopted sibling Store inventory before publication |
| `WASM_STORE_ADOPTED_BINDING_MISMATCH` | requested initial binding differs from the sole adopted Store binding | self | Use the protected adopted Store binding; never substitute caller-selected authority |
| `WASM_STORE_GC_WRITE_FENCED` | adopted Store GC mode is not `Normal` | self | Stop publication; complete the owning lifecycle action rather than bypassing the one-way write fence |
| `WASM_STORE_INITIAL_PUBLICATION_AUTHORITY_PRESENT` | initial pin encounters any existing active, detached or retired publication authority | self | Preserve the existing authority; initial bootstrap may not replace or rotate it |
| `WASM_STORE_INITIAL_PUBLICATION_BINDING_COMMIT_FAILED` | the empty-to-active stable-state transition refuses the exact adopted binding | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve the pre-transition state and inspect root Store authority; do not infer commitment |

The Store-cardinality identity is shared because both callers require the same
one adopted sibling Store and have the same repair. The two initial-binding
state failures remain distinct: pre-existing authority is a replay/ordering
conflict, while refusal of the validated empty-state commit is a stable-state
contradiction.

## Release Reconciliation And Publication

| Exact candidate or disposition | Source predicate | Public projection | Action and retry |
| --- | --- | --- | --- |
| reuse `WASM_STORE_GC_WRITE_FENCED` | catalog reconciliation observes a Store outside GC `Normal` | self | Complete lifecycle reconciliation before importing the release catalog |
| `WASM_STORE_EXACT_RELEASE_MISSING` | writable adopted Store lacks the exact role/template/version release | self | Publish or restore the exact admitted release; never accept another version or binding |
| `WASM_STORE_RELEASE_CONFLICT` | the same template/version key has different retained hash or size authority | self | Preserve both manifest and observed catalog evidence; do not overwrite or select one arbitrarily |
| reuse `WASM_STORE_BYTE_CAPACITY_EXCEEDED` | canonical release admission exceeds live Store byte capacity | self | Free retained capacity or publish a smaller exact release after inspecting the guarded projection |

The current `ExactReleaseMissing` constructor combines the first two rows with
`||`. B4 must split that branch before compact mapping. A lifecycle write fence
is not missing artifact data and cannot share its retry instruction with an
absent release.

## Chunk Traversal And Integrity

| Exact candidate or disposition | Source predicate | Public projection | Action and retry |
| --- | --- | --- | --- |
| reuse `WASM_STORE_CHUNK_INDEX_OVERFLOW` | manifest chunk position cannot fit the maintained `u32` index | self | Rebuild the release within the bounded chunk-index contract |
| reuse `WASM_STORE_CHUNK_HASH_MISMATCH` | management upload returns a hash different from protected manifest authority | self | Preserve both values, reject publication and never install the mismatched bytes |

Both meanings are already shared with Store preparation and bootstrap module
resolution. Layer-specific wrapper codes would add no action or authority.

## Transparent Transport Frontier

`management stored_chunks` and `management upload_chunk` each wrap a typed
`InternalError` in `TransportUnavailable`. The wrapper receives no identity.
The two static surfaces and every reachable typed IC leaf must select their
surface-specific registered identity, while operation-scoped publication
status retains any approved numeric reject/cycle evidence. This source
expansion remains open and is not included in the seven additions above.

## Dynamic Public Context

Slices 6, 8 and 12 of
[dynamic-public-context.md](dynamic-public-context.md) classify every field in
this scope. Manifest and release keys are derivable from protected inputs;
Store binding, GC and cardinality remain in the controller-guarded overview;
capacity and release-conflict observations require the already-proposed narrow
request-scoped projections. No diagnostic retains a principal, hash, size,
binding, mode, count or dependency-owned prose.

## Reconciliation

The twelve non-transport constructors expand to thirteen source predicates.
Seven add exact meanings. Six reuse qualified Store identities. No safe public
projection is added, and neither transport nor the GC lifecycle is counted.

The qualified semantic set moves from 2,755 to 2,762 exact candidates. The 31
safe projections are unchanged, producing 2,793 current symbolic identities.

## Required Tests

- exhaustive binding mapping for sole-slot, cardinality, adopted-binding,
  write-fence, pre-existing-authority and commit-refusal predicates;
- the two Store-cardinality routes reuse one identity without merging their
  guarded status correlation;
- catalog reconciliation distinguishes GC fencing from exact-release absence;
- conflicting release authority remains distinct from byte capacity;
- chunk overflow and uploaded-hash mismatch reuse the existing Store meanings;
- binding/release fields disappear from compact diagnostics while the guarded
  overview and request-scoped projections retain their exact evidence; and
- neither transport wrapper nor broad `InvalidState(String)` can select a
  generic publication diagnostic.

## Next Slice

Close the 56 production GC lifecycle constructions by exact reclamation,
binding-finalization, cycle-transfer and physical-deletion predicate. Then
expand the two publication transport surfaces through their typed IC leaves.
