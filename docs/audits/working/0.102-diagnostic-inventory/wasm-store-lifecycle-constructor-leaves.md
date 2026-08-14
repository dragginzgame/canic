# Canic 0.102 Wasm Store Lifecycle Constructor Leaves

Date: 2026-08-14

## Status

This B1 evidence ledger classifies all 22 production `InternalError`
constructor sites in the remaining Wasm Store lifecycle owner group. It assigns
no number and changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/wasm_store/mod.rs` | 3 |
| `workflow/runtime/template/mod.rs` | 8 |
| `workflow/runtime/template/client/mod.rs` | 4 |
| `workflow/runtime/template/publication/lifecycle/creation.rs` | 1 |
| `workflow/runtime/template/publication/lifecycle/inventory.rs` | 2 |
| `workflow/runtime/template/publication/lifecycle/gc.rs` | 4 |
| **Total** | **22** |

Inline test tails are excluded. Publication lifecycle errors already converted
from the closed `PublicationWorkflowError` owner remain in their existing typed
and dynamic ledgers; this document does not count them again merely because
they share the same modules.

## Store Deletion-Cycle Reclamation

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `WASM_STORE_DELETION_RETAINED_TARGET_ZERO` | 1 | Root requests a zero retained-cycle target | self | Supply the positive target frozen by root deletion authority | public to the authenticated root |
| `WASM_STORE_DELETION_RETAINED_TARGET_BELOW_CALL_COST` | 1 | Retained target cannot cover the exact `deposit_cycles` call cost | self | Recalculate and supply a sufficient protected target before reserving cycles | public to the authenticated root |
| `WASM_STORE_DELETION_GC_MODE_INCOMPLETE` / `WASM_STORE_DELETION_GC_RUN_COUNT_INVALID` / `WASM_STORE_DELETION_OCCUPIED_BYTES_NONZERO` / `WASM_STORE_DELETION_TEMPLATE_COUNT_NONZERO` / `WASM_STORE_DELETION_RELEASE_COUNT_NONZERO` / `WASM_STORE_DELETION_TEMPLATE_ROWS_PRESENT` / `WASM_STORE_DELETION_APPROVED_CATALOG_PRESENT` | 1 | One aggregate predicate merges the terminal GC mode, exact completed-run count, occupied-byte ledger, template/release counts, template rows and approved catalog | self for each exact leaf | Complete or reconcile the independently named Store authority before reclaiming cycles | guarded Store/root status |

The three sites add nine exact meanings. The seven-predicate branch must become
named policy predicates or typed state validation during B4; one broad
`Store not empty` code would conceal whether GC is unfinished, accounting is
nonzero or independently retained inventory remains.

## Approved Module-Source Resolution

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `WASM_STORE_CHUNK_SET_EMPTY` / `WASM_STORE_CHUNK_HASH_LENGTH_INVALID` | 1 | Root Store artifact metadata has no chunks or contains a non-SHA-256-sized chunk hash | self for empty-set input; `COMPONENT_REGISTRY_STATE_INVALID` for malformed protected metadata | Restage exact qualified chunk metadata; do not install | guarded root/operator; empty-set identity reused |
| `WASM_STORE_INLINE_SOURCE_UNSUPPORTED` | 1 | An approved manifest still selects the removed inline module-source path | self | Publish the role through the maintained chunked Store path | guarded root/operator |
| `WASM_STORE_CHUNK_SET_EMPTY` | 2 | Bootstrap-local or adopted-Store metadata resolves an empty approved chunk set | self; existing exact identity | Restage and publish the exact approved release before install | guarded root/operator |
| `WASM_STORE_BOOTSTRAP_SOURCE_PATH_FORBIDDEN` | 1 | A normal Component install attempts to consume the root-only bootstrap binding | self | Use the root control-plane bootstrap path or the admitted sibling Store binding | guarded root/operator |
| `WASM_STORE_CHUNK_INDEX_OVERFLOW` | 1 | Bootstrap chunk traversal cannot represent its index as `u32` | self; existing exact identity | Reject the oversized set and rebuild within the chunk-index contract | guarded root/operator |
| `WASM_STORE_CHUNK_HASH_MISMATCH` | 1 | Management upload returns a hash different from protected chunk metadata | self; existing exact identity | Preserve the mismatch and do not install or blindly retry | guarded root/operator |
| `WASM_STORE_BINDING_NOT_REGISTERED` | 1 | Approved manifest selects no current root-owned Store binding | self | Reconcile protected publication binding before module resolution | guarded root/operator |

The eight sites add four new exact meanings. They reuse the already-qualified
empty-set, chunk-index-overflow and chunk-hash-mismatch identities. Template,
role, index, binding and Store-principal prose is removed; the protected
manifest, publication binding and Store overview retain those facts.

## Internal Store Client

`WasmStoreInternalClient::call_result` has four constructor sites shared by its
eleven fixed Store methods.

| Disposition | Sites | Current meaning | Required hard cut |
| --- | ---: | --- | --- |
| transparent: typed IC request-encoding cause | 1 | `CallOps::with_args` already returns the exact typed Candid/IC cause, which is formatted into a new broad invariant | Return the nested registered `IC_CALL_REQUEST_ENCODING_FAILED` cause without copying prose |
| transparent: typed IC call cause | 1 | `CallOps::execute` already returns the exact call-admission/rejection cause, which is formatted into broad unavailability | Preserve the complete typed IC call leaf and owning workflow retry policy |
| transparent: typed IC response-decoding cause | 1 | `CallResult::candid` already returns the exact response contract cause, which is formatted into a new broad invariant | Return the nested registered `IC_CALL_RESPONSE_DECODING_FAILED` cause without copying prose |
| transparent: remote Store public diagnostic | 1 | A successfully decoded Store result contains its exact current public error | Propagate the remote registered diagnostic unchanged; do not wrap or renumber it |

These four sites allocate no Store-client code. The selected fixed method and
protected operation retain route context; a generic wrapper diagnostic would
duplicate the IC or remote Store authority and lose the typed absence/retry
distinctions.

## Bootstrap And Activation Inventory

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `WASM_STORE_SINGLE_ADOPTED_STORE_REQUIRED` | 1 | Bootstrap publication does not observe exactly one adopted sibling Store | self; existing exact identity | Reconcile root Store inventory before publication | controller-only Store overview |
| `FLEET_ACTIVATION_SINGLE_STORE_REQUIRED` | 1 | Fresh Fleet activation does not observe exactly one root-owned Store | self; existing exact identity | Reconcile root infrastructure before activation | controller-only Store overview |
| `WASM_STORE_SINGLE_ADOPTED_STORE_REQUIRED` | 1 | Publication snapshot does not observe exactly one adopted sibling Store | self; existing exact identity | Reconcile root Store inventory before snapshot/publication | controller-only Store overview |

All three sites reuse identities already qualified by dynamic-context rows
`DPC-079`, `DPC-084` and `DPC-085`. Store counts remain in the guarded overview
and disappear from diagnostic prose.

## Physical Store Stop And Deletion

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ROOT_STORE_DELETION_STOP_IN_PROGRESS` | 2 | Exact Store status is `Stopping` before the stop call or after response-loss observation | self | Wait and re-observe; never issue another stop while stopping | guarded root deletion operation |
| `ROOT_STORE_DELETION_STOP_NOT_EFFECTIVE` | 1 | A successful stop response is followed by an exact observation that the Store is still running | self | Preserve the durable deletion intent and retry only after re-observation | guarded root deletion operation |
| `ROOT_STORE_DELETION_NOT_ABSENT` | 1 | A successful delete response is followed by an exact observation that the Store still exists | self | Preserve the intent and re-observe typed status; never infer absence from the response | guarded root deletion operation |

The four sites add three exact meanings. Typed destination-invalid absence and
every non-absence IC failure remain owned by the existing IC adapter; none of
these progress codes is evidence that physical deletion occurred.

## Dynamic Public Context

The formatted values in this owner group are classified as dynamic-context
rows `DPC-139` through `DPC-154`:

- nine template, role and chunk-index values are caller-derivable from the
  protected release-set manifest or exact traversal;
- two Store principals, one Store binding and one Store count are retained by
  the protected Store overview/publication authority; and
- three nested internal-client errors are already authoritatively typed and
  must propagate without formatting.

No new general status DTO is required. The internal-client transport path must
retain the existing typed IC observability owner where its exact leaf is
masked; this ledger does not invent a second Store-call journal.

## Reconciliation

All 22 direct sites have dispositions. Four are transparent. The remaining
sites add 16 new exact meanings, reuse five existing exact identities and add no
safe projection. The effective whole-program constructor frontier therefore
moves from 2,053 to 2,075 classified sites and from 446 to 424 open sites.

The qualified semantic ledgers move from 2,333 to 2,349 provisional exact
candidates. Their 31 additional safe projections remain unchanged, producing
2,380 current symbolic identities before the final whole-program reuse and
allocation review.

## Required Tests

- distinguish zero retained target from target-below-call-cost before any
  reservation or transfer;
- independently reject all seven empty/GC-complete predicates;
- distinguish empty chunk sets, malformed chunk-hash length, removed inline
  sources, root-only bootstrap-path misuse and missing binding;
- retain the existing exact overflow and uploaded-hash mismatch identities;
- prove all four internal-client adapters propagate typed causes without
  formatted text or wrapper codes;
- reject zero/multiple Stores independently at bootstrap, activation and
  publication snapshot boundaries; and
- cover pre-stop `Stopping`, response-loss `Stopping`, ineffective stop and
  post-delete presence without treating any non-absence transport result as
  deletion success.

## Next Slice

Continue the effective frontier with Fleet Registry Mirror, Component Directory
synchronization and Fleet-service peer owners, preserving their separate
Registry, Directory and requester authority boundaries.
