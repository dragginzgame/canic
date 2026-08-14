# Canic 0.102 Dynamic Public Context Inventory

Date: 2026-08-13

## Status

This is a required evidence-only B1 ledger. It allocates no diagnostic code and
changes no endpoint. The current-source census and row-by-row classification
are not yet complete, so mutating batches B2-B6 remain blocked.

Diagnostic labels in this ledger are reconciliation links or provisional row
anchors. They do not add to the qualified symbolic count until their producer,
action, projection and reuse have been reconciled in the semantic ledgers.

The purpose is to prevent the `{ code : nat16 }` hard cut from silently losing
a dynamic value that a caller needs for correctness, recovery or remediation.
The durable-string inventory in [inventory.md](inventory.md) answers a different
question: whether text stored across interruption owns recovery state. A value
may require an entry in both ledgers.

## Census Boundary

Inventory every dynamic value interpolated into a public `Error.message`,
including values introduced by helper constructors or typed-to-public
conversions. At minimum the scan covers:

- operation, request, receipt and attempt identities;
- expected and observed generations, versions, hashes and counts;
- limits, balances, charges, retry times and capacity values;
- conflicting Canister, principal, Subnet, parent and authority identities;
- missing field, role, Component, pool and endpoint names; and
- nested typed causes whose dynamic fields survive only through formatting.

Static prose alone does not receive a row. When one message contains multiple
dynamic values, classify each value independently because they may have
different sensitivity and ownership.

## Required Classification

Every row has exactly one classification:

1. **Caller-derivable.** The caller already supplied or can deterministically
   derive the value from the request and maintained contract; discarding the
   interpolation does not remove authoritative information.
2. **Sensitive operator-only.** The public boundary must not expose the value.
   If it remains operationally useful, name an existing access-controlled log,
   receipt or status owner.
3. **Authoritatively typed.** An existing typed response, view, receipt or
   status record owns the value. Record that exact owner and retrieval path.
4. **Caller-required but unowned.** Correctness, recovery or remediation needs
   the value, but no typed owner currently exists. The owning endpoint must gain
   a specific typed response/view/receipt/status field before its message is
   removed.

Category 4 is an implementation prerequisite, not permission to add a generic
`detail : text` field. The proposed owner must have endpoint-specific semantics
and an explicit sensitivity review. A closed semantic discriminator may instead
become an exact registered diagnostic identity; numeric, identity, hash and
other data values still require a typed response/view/receipt/status owner.

## Required Row

The completed inventory records at least:

| Field | Requirement |
| --- | --- |
| source | Production file and constructor/conversion owner |
| endpoint or public route | Boundary through which the message is observable |
| diagnostic meaning | Exact or provisional symbolic identity |
| dynamic value | The individual interpolated datum, not the whole prose string |
| classification | One of the four classes above |
| authoritative owner | Existing typed owner and retrieval path, if any |
| proposed owner | Endpoint-specific typed owner required for category 4 |
| sensitivity | Public, guarded operator or prohibited public disclosure |
| hard-cut disposition | Discard, mask, retain outside the diagnostic, or add typed owner before removal |

## Approval Invariants

B1 is not complete until:

1. every production public-message interpolation has exactly one row;
2. every category-3 owner exists on the maintained current surface and is
   retrievable by the affected caller or operator;
3. every category-4 value has an approved endpoint-specific typed owner before
   the public hard cut;
4. no sensitive value becomes newly public through a typed replacement;
5. no recovery-, correctness- or remediation-significant dynamic value is
   unintentionally discarded; and
6. the completed ledger reconciles with the public-construction and conversion
   inventories without unexplained sites.

The eventual residue guard should scan constructors and conversion helpers for
new public formatting sites. It supplements review of typed values; it must not
classify behavior by matching prose.

## Classified Slice 1: Facade And Wasm Store GC

This first bounded slice closes four direct public-construction sites and five
individual dynamic values. It covers the only dynamic direct construction in
the shared Canic endpoint macros, the direct GC construction in the Wasm Store
API and both GC constructions in the Store ops layer. It does not yet classify
the typed `TemplateManifestOpsError` conversion or any transitive
`InternalError` projection.

| ID | Source and owner | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-001` | `crates/canic/src/macros/endpoints/shared.rs`; `canic_emit_memory_ledger_diagnostic_endpoint!` | `canic_memory_ledger` | existing `ACCESS_CONTROLLER_REQUIRED` | transport caller principal | 1 — caller-derivable | IC ingress caller; the rejected caller already knows its own principal | none | public to the affected caller; never catalogue identity | discard the interpolation and return only the exact controller-required code |
| `DPC-002` | `crates/canic-control-plane/src/api/template/mod.rs`; `WasmStoreCanisterApi::complete_gc` | `canic_wasm_store_complete_gc` | provisional Wasm Store GC transition-invalid meaning | persisted `current.mode` | 3 — authoritatively typed | `WasmStoreStatusResponse.gc.mode` from `canic_wasm_store_status` on the same Store | none | guarded operator evidence; root-only | remove it from the diagnostic; retain it in the existing typed status query |
| `DPC-003` | `crates/canic-control-plane/src/ops/storage/template/gc/mod.rs`; `WasmStoreGcOps::require_writable` | `canic_wasm_store_prepare`, `canic_wasm_store_stage_manifest`, `canic_wasm_store_publish_chunk` | provisional Wasm Store GC write-fenced meaning | persisted `current.mode` | 3 — authoritatively typed | `WasmStoreStatusResponse.gc.mode` from `canic_wasm_store_status` on the same Store | none | guarded operator evidence; root-only | remove it from the diagnostic; use the exact write-fenced code plus the existing typed status query |
| `DPC-004` | `crates/canic-control-plane/src/ops/storage/template/gc/mod.rs`; `transition_record` | `canic_wasm_store_prepare_gc`, `canic_wasm_store_begin_gc`, `canic_wasm_store_complete_gc` | provisional Wasm Store GC transition-invalid meaning | persisted `current.mode` | 3 — authoritatively typed | `WasmStoreStatusResponse.gc.mode` from `canic_wasm_store_status` on the same Store | none | guarded operator evidence; root-only | remove it from the diagnostic; retain it in the existing typed status query |
| `DPC-005` | `crates/canic-control-plane/src/ops/storage/template/gc/mod.rs`; `transition_record` | `canic_wasm_store_prepare_gc`, `canic_wasm_store_begin_gc`, `canic_wasm_store_complete_gc` | provisional Wasm Store GC transition-invalid meaning | requested transition target `next` | 1 — caller-derivable | invoked GC action and the maintained state-machine transition determine the target | none | guarded operator context; root-only | discard the interpolation; the exact transition-invalid code and invoked operation retain the action |

Slice totals are two caller-derivable values, no sensitive operator-only value,
three authoritatively typed values and no caller-required unowned value. No new
DTO or generic detail field is justified by this slice.

The second explicit Canic-macro construction is the static disabled-metrics
error and therefore correctly has no dynamic-value row. The remaining Store
ops construction is `TemplateManifestOpsError -> Error` and stays open because
its typed variants and formatted fields must be classified individually rather
than treating the aggregate `message` as one value.

## Classified Slice 2: Wasm Store Manifest And Capacity Conversion

This slice follows the maintained `TemplateManifestOpsError -> InternalError`
conversion through the variants shared by the root bootstrap buffer and the
separate Wasm Store. The two root-control-plane-only approved-manifest variants
remain open for the next root-publication pass; they are not silently included
here.

The provisional diagnostic names below are row anchors only. They do not join
the qualified symbolic set or reserve a number until the complete producer and
action review reconciles them with the Store publication workflow.

| ID | Source and owner | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-006` | `crates/canic-control-plane/src/ops/storage/template/mod.rs`; `TemplateManifestOpsError::TemplateChunkSetMissing` conversion | Store `info`, `chunk` or `publish_chunk`; root bootstrap/publication routes | provisional `WASM_STORE_CHUNK_SET_MISSING` | requested template release key | 1 — caller-derivable | exact `template_id` and `version` in the request or protected manifest | none | public | discard the formatted key; preserve the exact missing-set code |
| `DPC-007` | same conversion; `TemplateChunkMissing` | Store `chunk`; root bootstrap/publication routes | provisional `WASM_STORE_CHUNK_MISSING` | requested template release plus chunk index | 1 — caller-derivable | exact release and index in the request or protected manifest traversal | none | public | discard the formatted key; preserve the exact missing-chunk code |
| `DPC-008` | same conversion; `TemplateChunkSetEmpty` | Store `prepare`; root `template_prepare_admin` and bootstrap routes | provisional `WASM_STORE_CHUNK_SET_EMPTY` | requested template release key | 1 — caller-derivable | exact `template_id` and `version` in the submitted prepare request or protected manifest | none | public | discard the formatted key; preserve the empty-set code |
| `DPC-009` | same conversion; `PayloadHashMismatch` | root bootstrap/status and Store publication-admin routes | provisional `WASM_STORE_PAYLOAD_HASH_MISMATCH` | protected template release key | 1 — caller-derivable | exact release in the immutable root release-set manifest | none | guarded operator | discard the formatted key; keep the mismatch identity and protected manifest evidence |
| `DPC-010` | same conversion; `PayloadSizeMismatch` | root bootstrap/status and Store publication-admin routes | provisional `WASM_STORE_PAYLOAD_SIZE_MISMATCH` | protected template release key | 1 — caller-derivable | exact release in the immutable root release-set manifest | none | guarded operator | discard the formatted key; keep the mismatch identity and protected manifest evidence |
| `DPC-011` | same conversion; `ChunkIndexOverflow` | Store or root `prepare`; root bootstrap/publication routes | provisional `WASM_STORE_CHUNK_INDEX_OVERFLOW` | requested template release key | 1 — caller-derivable | exact release in the prepare request or protected manifest | none | public for request input; otherwise guarded operator | discard the formatted key; preserve the exact index-overflow code |
| `DPC-012` | same conversion; `TemplateChunkIndexOutOfRange` | Store `chunk` or `publish_chunk`; root `template_publish_chunk_admin` | provisional `WASM_STORE_CHUNK_INDEX_OUT_OF_RANGE` | requested template release key | 1 — caller-derivable | exact `template_id` and `version` in the request | none | public | discard the formatted key; preserve the exact range code |
| `DPC-013` | same conversion; `TemplateChunkIndexOutOfRange` | same routes as `DPC-012` | provisional `WASM_STORE_CHUNK_INDEX_OUT_OF_RANGE` | requested chunk index | 1 — caller-derivable | exact `chunk_index` in the request | none | public | discard the interpolation; preserve the exact range code |
| `DPC-014` | same conversion; `TemplateChunkHashMismatch` | Store `chunk` or `publish_chunk`; root bootstrap/publication routes | provisional `WASM_STORE_CHUNK_HASH_MISMATCH` | template release plus chunk index | 1 — caller-derivable | exact request or protected manifest traversal identifies the chunk | none | guarded operator; safe to the exact root caller | discard the formatted key; keep the exact hash-mismatch code and protected hash evidence |
| `DPC-015` | same conversion; `WasmStoreCapacityExceeded` | Store `prepare`, `stage_manifest` or `publish_chunk` | provisional `WASM_STORE_BYTE_CAPACITY_EXCEEDED` | rejected canonical `projected_bytes` | 4 — caller-required but unowned | none; current status exposes occupied, maximum and remaining bytes but not the rejected canonical projection | request-scoped `WasmStorePublicationCapacityProjectionResponse.projected_store_bytes`, computed by the same canonical Store ops path without a global last-error slot | guarded operator; root-only | add the exact typed preflight owner before removing this value from the public message |
| `DPC-016` | same conversion; `WasmStoreCapacityExceeded` | same routes as `DPC-015` | provisional `WASM_STORE_BYTE_CAPACITY_EXCEEDED` | configured `max_store_bytes` | 3 — authoritatively typed | `WasmStoreStatusResponse.max_store_bytes` via `canic_wasm_store_status` | none | guarded operator; root-only | remove it from the diagnostic and use the existing Store status |
| `DPC-017` | same conversion; `WasmStoreTemplateLimitExceeded` | Store `prepare` or `stage_manifest` | provisional `WASM_STORE_TEMPLATE_CAPACITY_EXCEEDED` | prospective distinct-template count | 1 — caller-derivable | submitted template identity plus `WasmStoreStatusResponse.templates` | none | guarded operator; root-only | discard the interpolation; the caller can derive the prospective count from request plus status |
| `DPC-018` | same conversion; `WasmStoreTemplateLimitExceeded` | same routes as `DPC-017` | provisional `WASM_STORE_TEMPLATE_CAPACITY_EXCEEDED` | configured maximum template count | 3 — authoritatively typed | `WasmStoreStatusResponse.max_templates` via `canic_wasm_store_status` | none | guarded operator; root-only | remove it from the diagnostic and use the existing Store status |
| `DPC-019` | same conversion; `WasmStoreVersionLimitExceeded` | Store `prepare` or `stage_manifest` | provisional `WASM_STORE_VERSION_CAPACITY_EXCEEDED` | submitted template identity | 1 — caller-derivable | exact `template_id` in the request | none | guarded operator; root-only | discard the interpolation; preserve the exact version-capacity code |
| `DPC-020` | same conversion; `WasmStoreVersionLimitExceeded` | same routes as `DPC-019` | provisional `WASM_STORE_VERSION_CAPACITY_EXCEEDED` | prospective retained-version count | 1 — caller-derivable | submitted release plus the matching `WasmStoreStatusResponse.templates[].versions` count | none | guarded operator; root-only | discard the interpolation; the caller can derive the prospective count from request plus status |
| `DPC-021` | same conversion; `WasmStoreVersionLimitExceeded` | same routes as `DPC-019` | provisional `WASM_STORE_VERSION_CAPACITY_EXCEEDED` | configured per-template version maximum | 3 — authoritatively typed | `WasmStoreStatusResponse.max_template_versions_per_template` via `canic_wasm_store_status` | none | guarded operator; root-only | remove it from the diagnostic and use the existing Store status |

Slice totals are twelve caller-derivable values, no sensitive operator-only
value, three authoritatively typed values and one caller-required unowned
value. `DPC-015` is a real B1 gate: the rejected canonical byte projection
cannot be reconstructed exactly from aggregate status when a publication
replaces an existing encoded entry. The replacement must be a request-scoped
typed projection using the same canonical calculation, not a persisted global
"last error" and not generic detail text.

## Classified Slice 3: Root Approved-Manifest Selection

The two feature-gated `TemplateManifestOpsError` variants excluded from Slice 2
are produced by root-local approved-role selection. Their current endpoint-
reachable path is the controller-guarded `canic_root_store_bootstrap` and
`canic_root_store_bootstrap_status` journey. The same selector also backs the
registered control-plane install-source resolver, although there is no current
in-repository endpoint caller of that support facade.

| ID | Source and owner | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-022` | `crates/canic-control-plane/src/ops/storage/template/mod.rs`; `TemplateManifestOpsError::ApprovedManifestMissing` | `canic_root_store_bootstrap`, `canic_root_store_bootstrap_status`; control-plane install resolver if used | provisional `WASM_STORE_APPROVED_MANIFEST_MISSING` | admitted Canister role | 1 — caller-derivable | exact role in the protected root release-set manifest or resolver request | none | guarded operator; safe to the exact provisioning caller | discard the interpolation; preserve the exact approved-manifest-missing code |
| `DPC-023` | same conversion; `ApprovedManifestConflict` | same routes as `DPC-022` | provisional `WASM_STORE_APPROVED_MANIFEST_CONFLICT` | admitted Canister role | 1 — caller-derivable | exact role in the protected root release-set manifest or resolver request | none | guarded operator; safe to the exact provisioning caller | discard the interpolation; preserve the exact approved-manifest-conflict code |

Slice totals are two caller-derivable values and no values in the other three
classes. The role remains typed in the immutable release-set authority; it does
not need a new error-detail field.

## Classified Slice 4: Runtime Store Source Resolution

Two dynamic values in `workflow/runtime/template` are on maintained endpoint
paths independent of the otherwise dormant generic install-source resolver.

| ID | Source and owner | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-024` | `crates/canic-control-plane/src/workflow/runtime/template/mod.rs`; `resolved_root_store_module_source` | `canic_root_component_install`, `canic_root_peer_component_install`, `canic_root_component_child_install` | provisional root Store artifact chunk-metadata-invalid meaning | protected root Store artifact template identity | 1 — caller-derivable | deterministic artifact-template prefix plus the admitted role in the protected Component allocation | none | safe to the exact provisioning caller | discard the interpolation; preserve the exact invalid-metadata code and protected artifact evidence |
| `DPC-025` | same file; `store_pid_for_binding` | `canic_wasm_store_admin` publication-source resolution | provisional root Store publication binding-not-registered meaning | unresolved Store binding | 2 — sensitive operator-only | controller-guarded `canic_wasm_store_overview` publication and Store-binding rows | none | prohibited on an unguarded public boundary; guarded operator only | mask the binding from the public code; retain it in the existing guarded overview and correlated publication operation evidence |

Slice totals are one caller-derivable and one sensitive operator-only value.
No typed replacement is required.

The other ten dynamic values in this file occur only below
`resolved_approved_module_source_for_role`: inline role and template identity,
four template identities, one Store principal, one chunk index, the local root
principal and the uploaded-chunk template identity. The resolver is registered,
but current repository source has no call to the exported
`resolve_approved_module_source` support facade. Those values therefore have no
maintained endpoint route at this baseline and do not receive public-context
rows. They remain explicit direct-constructor-frontier sites; adding a caller
must either classify them here first or delete the dormant path.

## Classified Slice 5: Explicit Component Registry Denials

The Component Registry workflow has twelve direct formatted public denials
containing 20 individual values. All are exact request or transport identities;
none is an observed authority value that would be lost with the prose.

| ID | Source owner | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-026` | `peer_component_requester` unregistered branch | `canic_root_peer_component_allocate` | existing `ACCESS_ACTIVE_COMPONENT_REQUIRED` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard interpolation; return exact membership-required code |
| `DPC-027` | `peer_component_requester` Component-Child branch | `canic_root_peer_component_allocate` | provisional `PEER_COMPONENT_TOP_LEVEL_REQUESTER_REQUIRED` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard interpolation; return exact top-level-requester code |
| `DPC-028` | `reserve_child_allocation` parent lookup | `canic_root_component_child_allocate` | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard interpolation; return exact parent-required code |
| `DPC-029` | same branch as `DPC-028` | same route | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | requested Component instance identity | 1 — caller-derivable | `RootComponentChildAllocationRequest.component` | none | public | discard interpolation; request retains Component identity |
| `DPC-030` | `child_allocation_status` parent lookup | `canic_root_component_child_allocation_status` | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard interpolation; return exact parent-required code |
| `DPC-031` | same branch as `DPC-030` | same route | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | requested Component instance identity | 1 — caller-derivable | `RootComponentChildAllocationStatusRequest.component` | none | public | discard interpolation; request retains Component identity |
| `DPC-032` | `create_child_allocation` parent lookup | `canic_root_component_child_create` | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard interpolation; return exact parent-required code |
| `DPC-033` | same branch as `DPC-032` | same route | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | requested Component instance identity | 1 — caller-derivable | `RootComponentChildCreationRequest.component` | none | public | discard interpolation; request retains Component identity |
| `DPC-034` | `install_child_allocation` parent lookup | `canic_root_component_child_install` | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard interpolation; return exact parent-required code |
| `DPC-035` | same branch as `DPC-034` | same route | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | requested Component instance identity | 1 — caller-derivable | `RootComponentChildInstallRequest.component` | none | public | discard interpolation; request retains Component identity |
| `DPC-036` | `commit_child_allocation` parent lookup | `canic_root_component_child_commit` | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard interpolation; return exact parent-required code |
| `DPC-037` | same branch as `DPC-036` | same route | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | requested Component instance identity | 1 — caller-derivable | `RootComponentChildCommitRequest.component` | none | public | discard interpolation; request retains Component identity |
| `DPC-038` | `active_component_member_authority` missing identity | root `canic_response_capability_v1` caller-authority resolution | existing `ACCESS_ACTIVE_COMPONENT_REQUIRED` | candidate member Canister principal | 1 — caller-derivable | transport caller passed to the resolver | none | public to affected caller | discard interpolation; return exact active-member-required code |
| `DPC-039` | `active_component_member_authority` inactive identity | same route as `DPC-038` | existing `ACCESS_ACTIVE_COMPONENT_REQUIRED` | candidate member Canister principal | 1 — caller-derivable | transport caller passed to the resolver | none | public to affected caller | discard interpolation; return exact active-member-required code |
| `DPC-040` | `directory_page` member lookup | `canic_root_component_directory_page` | provisional `COMPONENT_DIRECTORY_MEMBER_REQUIRED` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard interpolation; return exact Directory-member code |
| `DPC-041` | same branch as `DPC-040` | same route | provisional `COMPONENT_DIRECTORY_MEMBER_REQUIRED` | Component instance identity from requested Directory provenance | 1 — caller-derivable | `ComponentDirectoryPageRequest.directory` | none | public | discard interpolation; request retains Component identity |
| `DPC-042` | `prepared_child_runtime_plan` parent lookup | child Directory-prepare, runtime-activate and membership-activate routes | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard interpolation; return exact parent-required code |
| `DPC-043` | same branch as `DPC-042` | same routes | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | requested Component instance identity | 1 — caller-derivable | exact child-lifecycle request `component` | none | public | discard interpolation; request retains Component identity |
| `DPC-044` | `validate_requesting_parent_still_active` missing-member branch | child Directory-prepare, runtime-activate and membership-activate routes after awaits | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard interpolation; preserve exact post-await parent revalidation code |
| `DPC-045` | same branch as `DPC-044` | same routes | existing `COMPONENT_CHILD_CALLER_NOT_PARENT` | requested Component instance identity | 1 — caller-derivable | exact child-lifecycle request `component` | none | public | discard interpolation; request retains Component identity |

Slice totals are 20 caller-derivable values and no values in the other three
classes. The repeated principal and Component interpolations do not justify a
detail DTO: the transport caller and exact request already own them. This slice
does not classify the same module's broad transitive `InternalError` messages.

## Zero-Row Closure: Component RPC

The four explicit public constructions in
`crates/canic-control-plane/src/workflow/component_rpc/mod.rs` contain static
prose only. The recycle-target, recycle-parent, structural-parent-selector and
root-cannot-provision conditions therefore add no dynamic-value row. Their
exact diagnostic meanings remain allocation work, but this public-context
ledger has no unexplained Component RPC interpolation.

## Classified Slice 6: Typed Store Publication Causes

`PublicationWorkflowError` currently formats every typed variant into one
public message. This slice classifies its 20 scalar fields other than the
free-form `InvalidState(String)` payload and the transitive fields of
`TransportUnavailable.cause`. Those two nested frontiers are explicitly open
below.

| ID | Typed source field | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-046` | `CapacityExceeded.release` | root Store bootstrap/publication routes | provisional `WASM_STORE_BYTE_CAPACITY_EXCEEDED` | protected template release label | 1 — caller-derivable | release-set manifest template identity and version | none | guarded operator | discard label; preserve exact capacity code |
| `DPC-047` | `CapacityExceeded.target` | same routes as `DPC-046` | same meaning | selected Store binding | 3 — authoritatively typed | `WasmStoreOverviewStoreResponse.binding` | none | guarded operator | remove from diagnostic; retrieve through overview |
| `DPC-048` | `CapacityExceeded.payload_size_bytes` | same routes as `DPC-046` | same meaning | protected artifact payload bytes | 1 — caller-derivable | `TemplateManifestResponse.payload_size_bytes` from the release-set projection | none | guarded operator | discard interpolation; manifest retains value |
| `DPC-049` | `CapacityExceeded.remaining_store_bytes` | same routes as `DPC-046` | same meaning | observed live encoded Store headroom | 4 — caller-required but unowned | sibling `WasmStoreStatusResponse.remaining_store_bytes` is root-readable, but the affected root controller cannot retrieve it; root overview exposes different approved-payload accounting | request-scoped `WasmStorePublicationCapacityProjectionResponse.remaining_store_bytes` | guarded operator | add exact live-Store capacity preflight before removing this value from the public message |
| `DPC-050` | `ChunkHashMismatch.template_id` | root Store publication routes | provisional `WASM_STORE_CHUNK_HASH_MISMATCH` | protected template identity | 1 — caller-derivable | exact release-set manifest | none | guarded operator | discard interpolation; manifest retains identity |
| `DPC-051` | `ChunkHashMismatch.chunk_index` | same routes as `DPC-050` | same meaning | protected traversal index | 1 — caller-derivable | deterministic position in the manifest chunk-hash vector | none | guarded operator | discard interpolation; exact code and manifest traversal retain action |
| `DPC-052` | `ChunkHashMismatch.store_pid` | same routes as `DPC-050` | same meaning | sibling Store principal | 2 — sensitive operator-only | controller-guarded `WasmStoreOverviewStoreResponse.pid` | none | prohibited on unguarded public route | mask from diagnostic; retain in guarded overview and correlated operation evidence |
| `DPC-053` | `ChunkIndexOverflow.template_id` | root Store publication routes | provisional `WASM_STORE_CHUNK_INDEX_OVERFLOW` | protected template identity | 1 — caller-derivable | exact release-set manifest | none | guarded operator | discard interpolation; preserve exact overflow code |
| `DPC-054` | `ExactReleaseMissing.role` | root Store bootstrap/publication reconciliation | provisional `WASM_STORE_EXACT_RELEASE_MISSING` | admitted Canister role | 1 — caller-derivable | exact release-set manifest | none | guarded operator | discard interpolation; manifest retains role |
| `DPC-055` | `ExactReleaseMissing.template_id` | same routes as `DPC-054` | same meaning | expected template identity | 1 — caller-derivable | exact release-set manifest | none | guarded operator | discard interpolation; manifest retains identity |
| `DPC-056` | `ExactReleaseMissing.version` | same routes as `DPC-054` | same meaning | expected template version | 1 — caller-derivable | exact release-set manifest | none | guarded operator | discard interpolation; manifest retains version |
| `DPC-057` | `ExactReleaseMissing.expected_binding` | same routes as `DPC-054` | same meaning | expected Store binding | 1 — caller-derivable | protected manifest Store binding | none | guarded operator | discard interpolation; manifest retains binding |
| `DPC-058` | `ReleaseConflict.template_id` | root Store bootstrap/publication routes | provisional `WASM_STORE_RELEASE_CONFLICT` | submitted template identity | 1 — caller-derivable | exact manifest being published | none | guarded operator | discard interpolation; request authority retains identity |
| `DPC-059` | `ReleaseConflict.version` | same routes as `DPC-058` | same meaning | submitted template version | 1 — caller-derivable | exact manifest being published | none | guarded operator | discard interpolation; request authority retains version |
| `DPC-060` | `ReleaseConflict.binding` | same routes as `DPC-058` | same meaning | selected Store binding | 1 — caller-derivable | protected publication target and root Store state | none | guarded operator | discard interpolation; publication authority retains binding |
| `DPC-061` | `ReleaseConflict.existing_payload_hash` | same routes as `DPC-058` | same meaning | conflicting live catalog payload hash | 4 — caller-required but unowned | root-internal `PublicationStoreSnapshot.catalog`; not retrievable by the controller after this failure | request-scoped `WasmStoreReleaseInspectionResponse.observed_payload_hash` | guarded operator | add exact release inspection before removing the hash from the public message |
| `DPC-062` | `ReleaseConflict.existing_payload_size_bytes` | same routes as `DPC-058` | same meaning | conflicting live catalog payload size | 4 — caller-required but unowned | root-internal `PublicationStoreSnapshot.catalog`; not retrievable by the controller after this failure | request-scoped `WasmStoreReleaseInspectionResponse.observed_payload_size_bytes` | guarded operator | add exact release inspection before removing the size from the public message |
| `DPC-063` | `StoreNotWritable.binding` | root Store publication/removal routes | provisional `WASM_STORE_GC_WRITE_FENCED` | selected Store binding | 3 — authoritatively typed | `WasmStoreOverviewStoreResponse.binding` | none | guarded operator | remove from diagnostic; retrieve through overview |
| `DPC-064` | `StoreNotWritable.mode` | same routes as `DPC-063` | same meaning | current Store GC mode | 3 — authoritatively typed | `WasmStoreOverviewStoreResponse.gc.mode` and Store status | none | guarded operator | remove from diagnostic; retrieve through overview/status |
| `DPC-065` | `TransportUnavailable.surface` | root Store publication/removal routes | provisional exact stored-chunks or upload-chunk transport meaning | one of two static management-call surfaces | 4 — caller-required but unowned | none; a free-form `&'static str` is the current discriminator | exact registered diagnostic selected at each of the two construction sites | guarded operator | split the two static surfaces into exact codes; do not add a text field |

Slice totals are twelve caller-derivable values, one sensitive operator-only
value, three authoritatively typed values and four caller-required unowned
values. `DPC-049` joins the request-scoped publication capacity preflight
required by `DPC-015`; approved-payload overview accounting cannot substitute
for the Store's live encoded-byte status. `DPC-061` and `DPC-062` need one
guarded, request-scoped release inspection query. `DPC-065` is better owned by
two exact registered diagnostic identities than by another surface enum or
detail string.

`PublicationWorkflowError::InvalidState(String)` remains open: its producers
span publication binding, Store reclamation/deletion and release validation,
and each formatted field must be classified at its construction site. The
`TransportUnavailable.cause` fields also remain open and must preserve the
exact internal code through operation-correlated evidence. Formatting either
nested value as one row would conceal the very identity loss 0.102 removes.

## Classified Slice 7: Delegated-Session Bootstrap Boundary

`AuthApi::set_delegated_session_subject` is a public SDK helper rather than a
Canic-emitted endpoint. Its dynamic messages become observable only when a
consumer exports an application endpoint, as the checked-in test application
does. This slice classifies all 13 individual values in the helper's explicit
formatted constructions. The broad `AuthApi::map_auth_error` forwarding path
remains a separate transitive frontier because one `err.to_string()` can carry
many typed auth causes.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-066` | wallet-caller `validate_delegated_session_subject` rejection | consumer endpoint calling `AuthApi::set_delegated_session_subject` | provisional exact delegated-session principal-rejection reason | closed `DelegatedSessionSubjectRejection` variant | 4 — caller-required but unowned | internal access-policy enum only; no boundary owner | exact registered diagnostic selected for each of the eight rejection variants | public to the affected caller; reveals no foreign identity | replace formatted reason with the exact rejection diagnostic; do not add reason text |
| `DPC-067` | requested-subject `validate_delegated_session_subject` rejection | same route as `DPC-066` | same meaning | closed `DelegatedSessionSubjectRejection` variant | 4 — caller-required but unowned | internal access-policy enum only; no boundary owner | same exact rejection-diagnostic family as `DPC-066`; request and transport caller distinguish the rejected principal | public to the affected caller; reveals no foreign identity | replace formatted reason with the exact rejection diagnostic; do not add reason text |
| `DPC-068` | delegated-session subject mismatch | same route as `DPC-066` | provisional `AUTH_DELEGATED_SESSION_SUBJECT_MISMATCH` | requested delegated-subject principal | 1 — caller-derivable | endpoint request `delegated_subject` | none | public to the affected caller | discard interpolation; request retains the principal |
| `DPC-069` | delegated-session subject mismatch | same route as `DPC-066` | same meaning | verified bootstrap-token subject principal | 1 — caller-derivable | caller-supplied `DelegatedToken.claims.subject` after verification | none | public to the affected caller | discard interpolation; the submitted token retains the claim |
| `DPC-070` | `delegated_session_bootstrap_token_fingerprint` encode failure | same route as `DPC-066` | provisional `AUTH_DELEGATED_SESSION_BOOTSTRAP_TOKEN_INVALID` | dependency-owned Candid encode cause | 2 — sensitive operator-only | no exact public owner; `AuthMetricReason::TokenInvalid` records the bounded aggregate class | none | prohibited on the public application boundary | mask the dependency cause and retain only the exact token-invalid diagnostic plus aggregate operator metric |
| `DPC-071` | bootstrap replay-conflict binding | same route as `DPC-066` | provisional `AUTH_DELEGATED_SESSION_BOOTSTRAP_REPLAY_CONFLICT` | previously bound wallet principal | 2 — sensitive operator-only | private `DelegatedSessionBootstrapBindingRecord`; aggregate `AuthMetricReason::ReplayConflict` | none | prohibited public disclosure of another binding | remove the identity from the diagnostic; stable replay authority and aggregate metric remain |
| `DPC-072` | bootstrap replay-conflict binding | same route as `DPC-066` | same meaning | previously bound delegated-subject principal | 2 — sensitive operator-only | private `DelegatedSessionBootstrapBindingRecord`; aggregate `AuthMetricReason::ReplayConflict` | none | prohibited public disclosure of another binding | remove the identity from the diagnostic; stable replay authority and aggregate metric remain |
| `DPC-073` | `SessionCapacityReached.capacity` | same route as `DPC-066` | provisional `AUTH_DELEGATED_SESSION_CAPACITY_EXCEEDED` | fixed global active-session limit | 4 — caller-required but unowned | private `DELEGATED_SESSION_CAPACITY`; no boundary status owner | guarded `DelegatedSessionCapacityStatusResponse.session_limit` with current count | guarded operator only | add typed capacity status before removing the numeric value; public failure returns only the exact capacity code |
| `DPC-074` | `SessionSubjectCapacityReached.delegated_pid` | same route as `DPC-066` | provisional `AUTH_DELEGATED_SESSION_SUBJECT_CAPACITY_EXCEEDED` | requested delegated-subject principal | 1 — caller-derivable | endpoint request `delegated_subject` | none | public to the affected caller | discard interpolation; request retains the principal |
| `DPC-075` | `SessionSubjectCapacityReached.capacity` | same route as `DPC-066` | same meaning | fixed per-subject active-session limit | 4 — caller-required but unowned | private `DELEGATED_SESSION_SUBJECT_CAPACITY`; no boundary status owner | guarded request-scoped `DelegatedSessionCapacityStatusResponse.subject_session_limit` and selected-subject count | guarded operator only | add typed capacity status before removing the numeric value; public failure returns only the exact subject-capacity code |
| `DPC-076` | `BootstrapBindingCapacityReached.capacity` | same route as `DPC-066` | provisional `AUTH_DELEGATED_SESSION_BOOTSTRAP_BINDING_CAPACITY_EXCEEDED` | fixed global live bootstrap-binding limit | 4 — caller-required but unowned | private `DELEGATED_SESSION_BOOTSTRAP_BINDING_CAPACITY`; no boundary status owner | guarded `DelegatedSessionCapacityStatusResponse.bootstrap_binding_limit` with current count | guarded operator only | add typed capacity status before removing the numeric value; public failure returns only the exact binding-capacity code |
| `DPC-077` | `BootstrapBindingSubjectCapacityReached.delegated_pid` | same route as `DPC-066` | provisional `AUTH_DELEGATED_SESSION_BOOTSTRAP_BINDING_SUBJECT_CAPACITY_EXCEEDED` | requested delegated-subject principal | 1 — caller-derivable | endpoint request `delegated_subject` | none | public to the affected caller | discard interpolation; request retains the principal |
| `DPC-078` | `BootstrapBindingSubjectCapacityReached.capacity` | same route as `DPC-066` | same meaning | fixed per-subject live bootstrap-binding limit | 4 — caller-required but unowned | private `DELEGATED_SESSION_BOOTSTRAP_BINDING_SUBJECT_CAPACITY`; no boundary status owner | guarded request-scoped `DelegatedSessionCapacityStatusResponse.subject_bootstrap_binding_limit` and selected-subject count | guarded operator only | add typed capacity status before removing the numeric value; public failure returns only the exact subject binding-capacity code |

Slice totals are four caller-derivable values, three sensitive operator-only
values, no authoritatively typed value and six caller-required unowned values.
The two rejection-reason rows are closed semantic discriminators and therefore
need exact diagnostics rather than another response field. The four numeric
capacity rows need one bounded, guarded status projection containing global
counts/limits and request-scoped counts/limits for the selected subject. It
must not expose session wallets, bootstrap fingerprints or foreign subjects.

The `DelegatedSessionUpsertResult::Upserted` prose arm is unreachable from the
only caller because the helper is invoked only after excluding `Upserted`; it
adds no dynamic row and should be removed as implementation sediment in B5.
All other explicit `Error` constructions in `api/auth/session` are static.
`AuthApi::map_auth_error` and the typed causes reaching it remain open for the
transitive auth pass; this slice does not misclassify their formatted aggregate
as one datum.

## Zero-Row Closure: Runtime Introspection

`crates/canic-core/src/api/runtime/mod.rs` has no explicit dynamic public-error
construction. Its formatted receipt-capacity failure is deliberately stored in
the controller-guarded `CanicRuntimeStatus.recent_failures` projection and is
already classified as bounded operational text in [inventory.md](inventory.md),
not as an `Error.message` interpolation. `MemoryRuntimeApi::bootstrap_registry`
still forwards a typed memory-registry failure through `Error::from`; that is a
transitive conversion frontier, not an unexplained explicit runtime
interpolation.

## Classified Slice 8: Store Publication Binding And Inventory

This slice closes every dynamic invalid-state field in publication
`release/managed.rs`, `lifecycle/binding.rs` and `lifecycle/inventory.rs`.
Binding also contains two static invalid-state constructions, which add no row.
All seven values already belong to the root's controller-guarded Store overview;
none needs another detail DTO.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-079` | `single_store_catalog` cardinality failure | `canic_root_store_bootstrap_status` and bootstrap verification | provisional `WASM_STORE_SINGLE_ADOPTED_STORE_REQUIRED` | current root-owned Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` via controller-guarded `canic_wasm_store_overview` | none | controller-only | remove count from diagnostic; overview retains exact inventory |
| `DPC-080` | `require_active_publication_store` sole-binding failure | root Store bootstrap and `canic_wasm_store_admin` publication | provisional `WASM_STORE_SOLE_ACTIVE_PUBLICATION_BINDING_REQUIRED` | selected Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.publication` plus `stores[].binding` | none | controller-only | remove binding from diagnostic; overview retains binding and slots |
| `DPC-081` | `pin_initial_publication_store` cardinality failure | root Store bootstrap | provisional `WASM_STORE_SINGLE_ADOPTED_STORE_REQUIRED` | current adopted Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; overview retains exact inventory |
| `DPC-082` | `pin_initial_publication_store` binding mismatch | root Store bootstrap | provisional `WASM_STORE_ADOPTED_BINDING_MISMATCH` | selected snapshot binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.publication` plus the selected `stores[].binding` | none | controller-only | remove selected binding from diagnostic; overview retains publication authority |
| `DPC-083` | same branch as `DPC-082` | root Store bootstrap | same meaning | observed adopted Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove observed binding from diagnostic; overview retains current inventory |
| `DPC-084` | `root_activation_wasm_store` cardinality failure | controller-guarded `canic_prepare_fleet_activation` | provisional `FLEET_ACTIVATION_SINGLE_STORE_REQUIRED` | current root-owned Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; overview remains the Store inventory owner |
| `DPC-085` | `snapshot_adopted_wasm_store` cardinality failure | root Store bootstrap and `canic_wasm_store_admin` publication | provisional `WASM_STORE_SINGLE_ADOPTED_STORE_REQUIRED` | current adopted Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; overview remains the Store inventory owner |

Slice totals are seven authoritatively typed values and no values in the other
three classes. The same cardinality predicate appears at distinct journeys, so
the rows remain separate even if allocation later reuses one exact meaning.
The overview is observation only; recording it as the retrieval owner does not
move Store inventory or publication authority out of root stable state.

Publication GC still owns the remaining dynamic `InvalidState(String)`
producers. Its static invalid-state leaves still require exact diagnostic
allocation, while each formatted GC field must be classified independently.
The `TransportUnavailable.cause` and transitive auth formatter also remain open.

## Classified Slice 9: Store GC Fence And Reclamation Authority

This bounded GC slice covers final-inventory quiescence, removal reverification,
Store reclamation, reclaimed-binding verification and their shared runtime-GC
reconciliation/lookup helpers. It does not yet cover binding-slot finalization,
cycle reclamation or physical deletion.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-086` | final-inventory quiescence cardinality failure | controller root final-inventory journey | provisional `ROOT_FINAL_INVENTORY_SINGLE_STORE_REQUIRED` | current root-owned Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; root overview retains inventory |
| `DPC-087` | final-inventory GC-lineage mismatch | same route as `DPC-086` | provisional `ROOT_FINAL_INVENTORY_STORE_GC_LINEAGE_MISMATCH` | persisted runtime GC mode | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].gc.mode` | none | controller-only | remove runtime mode from diagnostic; overview retains it |
| `DPC-088` | same branch as `DPC-087` | same route | same meaning | live sibling Store GC mode | 4 — caller-required but unowned | internal root-to-Store `WasmStoreStatusResponse.gc.mode`; not retrievable by the external root controller | guarded root `WasmStoreLifecycleInspectionResponse.live_gc` bound to exact Store binding and principal | controller-only | add exact root-proxied live GC inspection before removing this value |
| `DPC-089` | prepared-GC authority persistence failure | same route as `DPC-086` | provisional `ROOT_FINAL_INVENTORY_STORE_GC_PERSIST_FAILED` | root-owned Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove binding from diagnostic; overview retains it |
| `DPC-090` | post-persist runtime/live GC mismatch | same route as `DPC-086` | provisional `ROOT_FINAL_INVENTORY_STORE_GC_AUTHORITY_MISMATCH` | root-owned Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove binding from diagnostic; overview retains it |
| `DPC-091` | removal reverification cardinality failure | `canic_fleet_subnet_root_removal_publish` | provisional `ROOT_REMOVAL_SINGLE_STORE_REQUIRED` | current root-owned Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; overview retains inventory |
| `DPC-092` | removal runtime/live GC mismatch | same route as `DPC-091` | provisional `ROOT_REMOVAL_STORE_GC_AUTHORITY_MISMATCH` | root-owned Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove binding from diagnostic; overview retains it |
| `DPC-093` | Store reclamation cardinality failure | `canic_fleet_subnet_root_store_reclaim` | provisional `ROOT_STORE_RECLAMATION_SINGLE_STORE_REQUIRED` | current root-owned Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; overview retains inventory |
| `DPC-094` | Store reclamation terminal-mode failure | same route as `DPC-093` | provisional `ROOT_STORE_RECLAMATION_GC_INCOMPLETE` | live sibling Store GC mode | 4 — caller-required but unowned | internal root-to-Store `WasmStoreStatusResponse.gc.mode`; not retrievable by the external root controller | same guarded root `WasmStoreLifecycleInspectionResponse.live_gc` as `DPC-088` | controller-only | add exact root-proxied live GC inspection before removing this value |
| `DPC-095` | reclaimed-binding verification cardinality failure | `canic_fleet_subnet_root_store_binding_finalize` | provisional `ROOT_STORE_BINDING_FINALIZATION_SINGLE_STORE_REQUIRED` | current root-owned Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; overview retains inventory |
| `DPC-096` | `reconcile_single_root_store_gc` persistence failure | Store reclamation route | provisional `ROOT_STORE_GC_RECONCILIATION_PERSIST_FAILED` | root-owned Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove binding from diagnostic; overview retains it |
| `DPC-097` | post-reconciliation runtime/live mismatch | Store reclamation route | provisional `ROOT_STORE_GC_RECONCILIATION_MISMATCH` | root-owned Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove binding from diagnostic; overview retains it |
| `DPC-098` | `runtime_store` missing-binding lookup | root Store reclamation, binding-finalization and deletion journeys | provisional `ROOT_STORE_RUNTIME_BINDING_MISSING` | requested root-owned Store binding | 3 — authoritatively typed | protected lifecycle intent plus `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove binding from diagnostic; request/status and overview retain it |

Slice totals are eleven authoritatively typed values, two caller-required
unowned values and no values in the other two classes. The missing owner is not
a general Store proxy: it is one bounded, controller-guarded lifecycle
inspection binding the persisted runtime authority to the exact live sibling
Store status used by the workflow. The ordinary root overview remains
insufficient precisely when runtime and live GC modes disagree.

## Classified Slice 10: Store Binding Finalization And Cycle Reclamation

The binding-slot driver and main cycle-reclamation workflow contain many static
invariant failures but only four dynamic values. Static failures remain exact
allocation work and do not receive rows here.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-099` | `binding_finalization_transition_error.transition` | `canic_fleet_subnet_root_store_binding_finalize` | one of four provisional Store binding-transition failures | one of four static binding transition labels | 4 — caller-required but unowned | none; free-form helper argument is the current discriminator | exact registered diagnostic selected at the four construction sites | controller-only | split clear-active, retire-detached, finalize-retired and terminal-convergence failures into exact codes |
| `DPC-100` | post-reclamation retained-target failure | `canic_fleet_subnet_root_store_delete` cycle-reclamation phase | provisional `ROOT_STORE_CYCLE_RECLAMATION_TARGET_EXCEEDED` | observed live Store cycles after reclamation | 4 — caller-required but unowned | private live status observation; terminal deletion response does not exist yet | guarded operation-scoped `FleetSubnetRootStoreDeletionProgressResponse.observed_cycles_after_reclamation` | controller-only financial evidence | add typed in-progress evidence before removing the numeric value |
| `DPC-101` | same branch as `DPC-100` | same route | same meaning | durable retained-cycle target | 4 — caller-required but unowned | private root Store deletion intent; terminal response exposes it only after later physical deletion | guarded operation-scoped `FleetSubnetRootStoreDeletionProgressResponse.retained_cycles_target` | controller-only financial evidence | add typed in-progress evidence before removing the numeric value |
| `DPC-102` | `status_cycles.label` overflow helper | Store deletion preparation, cycle reclamation and physical deletion | one of six provisional Store status numeric-overflow meanings | one of six static Canister-status field labels | 4 — caller-required but unowned | none; free-form helper argument is the current discriminator | exact registered diagnostic selected at each static status-field call site | controller-only | replace label formatting with exact per-field codes; do not add text detail |

Slice totals are four caller-required unowned values and no values in the other
three classes. `DPC-099` and `DPC-102` are closed static discriminators, so
exact codes are their typed owners. `DPC-100` and `DPC-101` are actual financial
data and therefore need a bounded operation-scoped progress projection. The
existing `canic_fleet_subnet_root_store_deletion_status` returns only a terminal
receipt and cannot recover these values while deletion is incomplete.

## Classified Slice 11: Store Physical-Deletion Inventory

The physical-deletion helpers add one final dynamic GC value. All other
physical-deletion `InvalidState` producers are static prose.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-103` | `single_finalized_runtime_store` cardinality failure | `canic_fleet_subnet_root_store_delete` preparation phase | provisional `ROOT_STORE_DELETION_SINGLE_RUNTIME_STORE_REQUIRED` | current root-owned runtime Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; root overview retains exact runtime inventory |

Slice totals are one authoritatively typed value and no values in the other
three classes. This closes every dynamic `InvalidState(String)` field in
publication GC. Its remaining static invariant branches still need exact
registered meanings during allocation, but they do not belong in this dynamic
value ledger.

## Classified Slice 12: Store Publication Management Transport

The two `PublicationWorkflowError::TransportUnavailable` construction sites
flatten `IcInfraError` through `OpsError` and `InternalError` before formatting
it into the public message. Expanding the reachable `ic-cdk 0.20.2` call error
shape produces seven data fields per surface. `CallPerformFailed` and the typed
cause discriminators contain no data field, so they remain exact allocation
work rather than receiving dynamic rows.

Neither `RootStoreBootstrapRequest` nor `WasmStoreAdminCommand` carries a
caller-retrievable publication operation ID. The cost guard allocates an
internal reservation ID, but that record owns quota settlement, is recovered
after failure and stores no publication diagnostic. The heap-only recent-
failure ring is also not durable or request-correlated. Neither is an
observability owner for these rows.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-104` | `management stored_chunks` insufficient-liquid-cycles cause | root Store bootstrap and active-release publication | provisional `WASM_STORE_STORED_CHUNKS_LIQUID_CYCLES_INSUFFICIENT` | liquid cycles available when the call was attempted | 4 — caller-required but unowned | transient `ic-cdk::call::InsufficientLiquidCycleBalance`; flattened before workflow handling | guarded operation-scoped `WasmStorePublicationAttemptStatusResponse.observed_available_cycles` | controller-only financial evidence | persist the value against the exact publication attempt before returning the exact diagnostic |
| `DPC-105` | same cause as `DPC-104` | same routes | same meaning | cycles required for the exact call | 4 — caller-required but unowned | same transient call error | `WasmStorePublicationAttemptStatusResponse.required_call_cycles` on the same attempt | controller-only financial evidence | persist the value with `DPC-104`; do not put cycle amounts in the diagnostic |
| `DPC-106` | `management stored_chunks` rejection cause | same routes | provisional exact stored-chunks rejection meaning selected by recognized IC reject class | raw IC reject code | 4 — caller-required but unowned | transient `ic-cdk::call::CallRejected`; current metrics retain only `Infra` | exact registered diagnostic for every recognized reject class; `WasmStorePublicationAttemptStatusResponse.unrecognized_reject_code` for an unknown raw value | controller-only; the numeric IC reject code is safe, raw reject prose is not | exhaustively type recognized reject classes and retain only an unrecognized raw number in the exact attempt status |
| `DPC-107` | same rejection as `DPC-106` | same routes | same rejection meaning | replica reject message | 2 — sensitive operator-only | no safe typed owner; current public message is the only retention | none | prohibited public raw platform text | discard the text; exact rejection diagnostic and optional unknown numeric code are sufficient |
| `DPC-108` | `management stored_chunks` request encoding failure | same routes | provisional `WASM_STORE_STORED_CHUNKS_REQUEST_ENCODE_FAILED` | dependency-owned Candid encode cause | 2 — sensitive operator-only | no typed owner | none | prohibited public implementation detail | discard the dependency prose; the surface-specific exact diagnostic identifies the failed contract phase |
| `DPC-109` | `management stored_chunks` response decoding failure | same routes | provisional `WASM_STORE_STORED_CHUNKS_RESPONSE_DECODE_FAILED` | Rust `type_name` of the expected response | 2 — sensitive operator-only | no boundary owner; the maintained adapter source statically selects the response DTO | none | prohibited public implementation/package detail | discard the Rust type name; the surface-specific exact diagnostic identifies the maintained response contract |
| `DPC-110` | same decode failure as `DPC-109` | same routes | same meaning | dependency-owned Candid decode cause | 2 — sensitive operator-only | no typed owner | none | prohibited public implementation detail and possibly remote payload context | discard the dependency prose; retain only the exact response-decode diagnostic |
| `DPC-111` | `management upload_chunk` insufficient-liquid-cycles cause | root Store bootstrap and active-release publication | provisional `WASM_STORE_UPLOAD_CHUNK_LIQUID_CYCLES_INSUFFICIENT` | liquid cycles available when the chunk call was attempted | 4 — caller-required but unowned | transient `ic-cdk::call::InsufficientLiquidCycleBalance`; flattened before workflow handling | guarded operation-scoped `WasmStorePublicationAttemptStatusResponse.observed_available_cycles` | controller-only financial evidence | persist the value against the exact release and chunk attempt before returning the exact diagnostic |
| `DPC-112` | same cause as `DPC-111` | same routes | same meaning | cycles required for the exact chunk call | 4 — caller-required but unowned | same transient call error | `WasmStorePublicationAttemptStatusResponse.required_call_cycles` on the same attempt | controller-only financial evidence | persist the value with `DPC-111`; the exact chunk identity is required because request size affects cost |
| `DPC-113` | `management upload_chunk` rejection cause | same routes | provisional exact upload-chunk rejection meaning selected by recognized IC reject class | raw IC reject code | 4 — caller-required but unowned | transient `ic-cdk::call::CallRejected`; current metrics retain only `Infra` | exact registered diagnostic for every recognized reject class; `WasmStorePublicationAttemptStatusResponse.unrecognized_reject_code` for an unknown raw value | controller-only; the numeric IC reject code is safe, raw reject prose is not | exhaustively type recognized reject classes and retain only an unrecognized raw number in the exact attempt status |
| `DPC-114` | same rejection as `DPC-113` | same routes | same rejection meaning | replica reject message | 2 — sensitive operator-only | no safe typed owner; current public message is the only retention | none | prohibited public raw platform text | discard the text; exact rejection diagnostic and optional unknown numeric code are sufficient |
| `DPC-115` | `management upload_chunk` request encoding failure | same routes | provisional `WASM_STORE_UPLOAD_CHUNK_REQUEST_ENCODE_FAILED` | dependency-owned Candid encode cause | 2 — sensitive operator-only | no typed owner | none | prohibited public implementation detail | discard the dependency prose; the surface-specific exact diagnostic identifies the failed contract phase |
| `DPC-116` | `management upload_chunk` response decoding failure | same routes | provisional `WASM_STORE_UPLOAD_CHUNK_RESPONSE_DECODE_FAILED` | Rust `type_name` of the expected response | 2 — sensitive operator-only | no boundary owner; the maintained adapter source statically selects the response DTO | none | prohibited public implementation/package detail | discard the Rust type name; the surface-specific exact diagnostic identifies the maintained response contract |
| `DPC-117` | same decode failure as `DPC-116` | same routes | same meaning | dependency-owned Candid decode cause | 2 — sensitive operator-only | no typed owner | none | prohibited public implementation detail and possibly remote payload context | discard the dependency prose; retain only the exact response-decode diagnostic |

Slice totals are eight sensitive operator-only values and six caller-required
unowned values. No value is caller-derivable or already authoritatively typed.
The attempt status must be narrow rather than a generic IC-effect journal: it
binds a nonzero caller-supplied publication operation ID to the protected Store
binding and principal, release identity, transport surface and optional chunk
index, then retains the exact numeric diagnostic and only the three approved
numeric fields above. Bootstrap and admin commands must expose that operation
identity before the hard cut. Known reject classes become safe exact public
diagnostics; raw reject messages and Candid implementation prose are discarded.

The infra and ops layers must preserve an exhaustive typed call cause until the
publication workflow performs this classification. Parsing the current
`InternalError` message would recreate the string authority that 0.102 removes.

## Classified Slice 13: Coordinator Root-Deletion Closed Labels

The dedicated Coordinator root-deletion owner has two generic helpers with
dynamic message values. Both helpers receive only closed local discriminators;
neither justifies public detail text.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-118` | `fleet_coordinator::root_deletion::find_root_deletion_record`; conflicting lookup | Coordinator root-deletion intent, readiness, execution, completion and status routes | one of four exact root-deletion record-family identity conflicts | static record-family `label` selected by the typed caller | 1 — caller-derivable | invoked endpoint and maintained lookup wrapper determine the record family | none | guarded root/operator context | discard the label; select the exact intent/readiness/execution/deletion identity-conflict diagnostic at the typed wrapper |
| `DPC-119` | `fleet_coordinator::root_deletion::response_hash`; Candid encoding failure | Coordinator root-deletion intent, readiness, execution and completion routes | one of four exact root-deletion record-family encoding failures | static record-family `label` selected with the hash domain | 1 — caller-derivable | invoked endpoint and maintained hash domain determine the record family | none | guarded root/operator context | discard the label; select the exact intent/readiness/execution/deletion encoding diagnostic at the typed caller |
| `DPC-120` | same encoding failure as `DPC-119` | same routes | same record-family encoding failure | dependency-owned Candid encoder cause | 2 — sensitive operator-only | no safe typed public owner | structured diagnostic log carrying the exact numeric code and cause outside the public response | prohibited public implementation detail | remove the cause from the public diagnostic; retain it only in structured operator logging |

Slice totals are two caller-derivable values and one sensitive operator-only
value. No value is already authoritatively typed or caller-required unowned.
The generic helpers must accept a closed internal record-family discriminator
or move to typed wrappers during B2; they must not choose a diagnostic by
matching the current label text.

## Classified Slice 14: Fleet Coordinator Workflow

The Coordinator workflow introduces one direct dynamic value. Root-returned
errors are transparent propagation and remain owned by their root producers.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-121` | `FleetCoordinatorWorkflow::initialize`; non-controller denial | Coordinator initialization | existing `ACCESS_CONTROLLER_REQUIRED` | transport caller principal | 1 — caller-derivable | IC ingress caller; the rejected caller already knows its principal | none | public to the affected caller; never catalogue identity | discard the interpolation and return only the exact controller-required code |

The slice adds one caller-derivable value and no sensitive, typed or unowned
value. It reuses the same disposition as the memory-ledger controller boundary
without allocating another diagnostic identity.

## Classified Slice 15: Canister Pool Inventory And Recycling

The first Canister pool ops range has two interpolated principals. Each is
copied directly from the request being rejected.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-122` | `CanisterPoolOps::initialize_imports`; existing-asset conflict | root pool initialization | provisional `CANISTER_POOL_IMPORT_ASSET_CONFLICT` | imported physical Canister principal | 1 — caller-derivable | exact import request | none | public to the controller that supplied it | discard the interpolation; request plus exact code identify the asset |
| `DPC-123` | `CanisterPoolOps::register_recycled_pending`; workload-state conflict | Component removal/recycling | provisional `CANISTER_POOL_RECYCLE_WORKLOAD_REQUIRED` | physical workload Canister principal | 1 — caller-derivable | exact recycling request | none | public to the authenticated root workflow | discard the interpolation; request plus exact code identify the asset |

The slice adds two caller-derivable values and no sensitive, typed or unowned
value. The reset-failure reason stored by this owner is operational state, not
an error-message interpolation, and remains in the durable-string ledger.

## Classified Slice 16: Canister Pool Required Asset

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-124` | `required_asset`; inventory lookup miss | pool reset, claim, handoff, Store and recycling routes | provisional `CANISTER_POOL_ASSET_NOT_REGISTERED` | requested physical Canister principal | 1 — caller-derivable | exact caller request or retained operation being reconciled | none | public to the authenticated pool/root workflow | discard the interpolation; request/operation plus exact code identify the asset |

The slice adds one caller-derivable value and no sensitive, typed or unowned
value.

## Classified Slice 17: Canister Pool Workflow Inputs And Routing

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-125` | `status`; page-limit denial | pool status | provisional `CANISTER_POOL_STATUS_LIMIT_INVALID` | maximum page limit 256 | 1 — caller-derivable | maintained endpoint contract | none | public | discard the interpolation; exact code and contract define the bound |
| `DPC-126` | `validate_import_subnet`; missing route | pool import | provisional `CANISTER_POOL_IMPORT_SUBNET_ROUTE_MISSING` | requested physical Canister principal | 1 — caller-derivable | exact import request | none | public to the controller | discard the interpolation; request plus code identify the target |
| `DPC-127` | `validate_import_subnet`; route mismatch | pool import | provisional `CANISTER_POOL_IMPORT_SUBNET_MISMATCH` | requested physical Canister principal | 1 — caller-derivable | exact import request | none | public to the controller | discard the interpolation; request plus code identify the target |
| `DPC-128` | same route mismatch | pool import | same meaning | observed NNS Registry Subnet | 4 — caller-required but unowned | transient typed NNS route result | guarded `CanisterPoolImportRoutingStatusResponse.observed_subnet` bound to target and Registry version | guarded network topology | retain in the bounded routing status, not diagnostic text |
| `DPC-129` | same route mismatch | pool import | same meaning | protected root placement Subnet | 4 — caller-required but unowned | protected Fleet Subnet Root binding, not currently exposed by an import-result owner | guarded `CanisterPoolImportRoutingStatusResponse.expected_subnet` on the same query | guarded Fleet placement authority | retain beside `DPC-128`; both values must use one Registry-versioned observation |

The slice adds three caller-derivable and two caller-required unowned values.
The proposed response is a narrow read-only routing inspection keyed by the
requested Canister; it is not a generic last-error record and cannot become
placement authority.

## Classified Slice 18: Root Store Bootstrap

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-130` | `load_and_validate_manifest`; size bound | root Store bootstrap/status | provisional `ROOT_STORE_RELEASE_SET_MANIFEST_SIZE_INVALID` | maximum manifest bytes | 1 — caller-derivable | maintained root Store manifest contract | none | guarded root/host | discard interpolation; contract plus exact code owns the bound |
| `DPC-131` | manifest JSON decoding | same routes | provisional `ROOT_STORE_RELEASE_SET_JSON_INVALID` | dependency parser cause | 2 — sensitive operator-only | no safe typed public owner | structured diagnostic log | prohibited public implementation detail | remove from public response; retain with exact code only in operator log |
| `DPC-132` | manifest canonical encoding | same routes | provisional `ROOT_STORE_RELEASE_SET_CANONICALIZATION_FAILED` | dependency serializer cause | 2 — sensitive operator-only | no safe typed public owner | structured diagnostic log | prohibited public implementation detail | remove from public response; retain with exact code only in operator log |
| `DPC-133` | protected Store capacity denial | same routes | provisional `ROOT_STORE_BYTE_CAPACITY_EXCEEDED` | deduplicated required payload bytes | 1 — caller-derivable | exact host-produced release-set manifest | none | guarded root/host | discard interpolation; caller can reproduce the canonical sum |
| `DPC-134` | same capacity denial | same routes | same meaning | protected maximum Store bytes | 1 — caller-derivable | exact root plan/configuration supplied by the host | none | guarded root/host | discard interpolation; caller already owns the frozen limit |
| `DPC-135` | staged-artifact authority mismatch | same routes | provisional `ROOT_STORE_STAGED_ARTIFACT_AUTHORITY_MISMATCH` | artifact role | 1 — caller-derivable | exact release-set manifest entry | none | guarded root/host | discard interpolation; manifest plus exact code identifies the role |

The slice adds four caller-derivable and two sensitive operator-only values.
The two currently stringified typed topology causes are not counted again here:
they become transparent and remain decomposed by the existing transitive
topology/configuration inventories.

## Classified Slice 19: Root Bootstrap And Store Adoption State

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-136` | root Store-state begin-adoption adapter | root Store adoption | one of three exact adoption state diagnostics | `SiblingWasmStoreAdoptionError` variant | 3 — authoritatively typed | closed stable-state error enum | exact exhaustive mapping | guarded root/host | discard Debug text and map the variant directly |
| `DPC-137` | root Store-state commit-adoption adapter | same route | one of four exact adoption state diagnostics | `SiblingWasmStoreAdoptionError` variant | 3 — authoritatively typed | closed stable-state error enum | exact exhaustive mapping | guarded root/host | discard Debug text and map the variant directly |
| `DPC-138` | `root_set_subnet_id`; current-Subnet discovery error | root lifecycle bootstrap | exact nested IC/Registry discovery diagnostic | typed nested `InternalError` | 3 — authoritatively typed | maintained `IcWorkflow::try_get_current_subnet_pid` cause | transparent registered-code propagation | guarded root/operator | remove workflow prose and propagate the exact nested diagnostic |

The slice adds three authoritatively typed values and no caller-derivable,
sensitive or unowned value.

## Classified Slice 20: Wasm Store Lifecycle And Module Resolution

This slice closes every direct public-message interpolation in the remaining
Wasm Store lifecycle constructor group. Static Store-deletion progress messages
add no dynamic row.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-139` | `resolved_root_store_module_source`; invalid artifact chunk metadata | root Store artifact resolution | exact `WASM_STORE_CHUNK_SET_EMPTY` or `WASM_STORE_CHUNK_HASH_LENGTH_INVALID` selected by the failed predicate | protected root-artifact template identity | 1 — caller-derivable | exact release-set manifest and selected role | none | guarded root/operator | discard template prose and select the exact failed metadata predicate |
| `DPC-140` | `approved_module_source_from_manifest`; removed inline path | Component install-source resolution | `WASM_STORE_INLINE_SOURCE_UNSUPPORTED` | selected role | 1 — caller-derivable | exact approved manifest | none | guarded root/operator | discard role prose; manifest plus exact code identifies the release |
| `DPC-141` | same branch as `DPC-140` | same route | same meaning | selected template identity | 1 — caller-derivable | exact approved manifest | none | guarded root/operator | discard template prose; manifest retains identity |
| `DPC-142` | `resolved_bootstrap_chunk_set_for_manifest`; empty chunk metadata | root bootstrap install-source resolution | existing `WASM_STORE_CHUNK_SET_EMPTY` | protected template identity | 1 — caller-derivable | exact approved manifest | none | guarded root/operator | discard template prose and preserve the existing empty-set identity |
| `DPC-143` | `resolved_store_chunk_set_for_manifest`; bootstrap binding on ordinary path | ordinary Component install-source resolution | `WASM_STORE_BOOTSTRAP_SOURCE_PATH_FORBIDDEN` | protected template identity | 1 — caller-derivable | exact approved manifest | none | guarded root/operator | discard template prose; exact code identifies the wrong path |
| `DPC-144` | same function; empty adopted-Store metadata | ordinary Component install-source resolution | existing `WASM_STORE_CHUNK_SET_EMPTY` | protected template identity | 1 — caller-derivable | exact approved manifest | none | guarded root/operator | discard template prose and preserve the existing empty-set identity |
| `DPC-145` | same branch as `DPC-144` | same route | same meaning | selected Store principal | 3 — authoritatively typed | protected binding plus `WasmStoreOverviewResponse.stores[].pid` | none | controller-only | remove principal from diagnostic; overview retains Store identity |
| `DPC-146` | `ensure_bootstrap_chunk_hashes_present`; index conversion | root bootstrap install-source resolution | existing `WASM_STORE_CHUNK_INDEX_OVERFLOW` | protected template identity | 1 — caller-derivable | exact approved manifest | none | guarded root/operator | discard template prose and preserve the exact overflow identity |
| `DPC-147` | same function; uploaded-hash mismatch | same route | existing `WASM_STORE_CHUNK_HASH_MISMATCH` | protected template identity | 1 — caller-derivable | exact approved manifest | none | guarded root/operator | discard template prose; manifest retains identity |
| `DPC-148` | same branch as `DPC-147` | same route | same meaning | exact chunk index | 1 — caller-derivable | deterministic manifest traversal | none | guarded root/operator | discard index prose; protected traversal identifies the chunk |
| `DPC-149` | same branch as `DPC-147` | same route | same meaning | root principal used as management chunk store | 3 — authoritatively typed | protected receiver/root identity | none | controller-only | remove principal from diagnostic; root binding retains it |
| `DPC-150` | `store_pid_for_binding`; missing registered binding | Component install-source resolution | `WASM_STORE_BINDING_NOT_REGISTERED` | selected Store binding | 3 — authoritatively typed | approved manifest plus root Store publication/overview state | none | controller-only | remove binding prose; protected manifest and overview retain it |
| `DPC-151` | `ensure_bootstrap_wasm_store`; Store cardinality failure | root Store bootstrap/publication | existing `WASM_STORE_SINGLE_ADOPTED_STORE_REQUIRED` | current root-owned Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; overview retains exact inventory |
| `DPC-152` | `WasmStoreInternalClient::call_result`; request adapter | every root-to-Store internal method | exact nested `IC_CALL_REQUEST_ENCODING_FAILED` | typed nested `InternalError` with formatted dependency cause | 3 — authoritatively typed | maintained IC request adapter | transparent registered-code propagation | sensitive dependency prose | remove wrapper text and propagate the exact nested diagnostic |
| `DPC-153` | same helper; call execution adapter | every root-to-Store internal method | exact nested IC call-admission or rejection diagnostic | typed nested `InternalError` with call cause | 3 — authoritatively typed | maintained IC call adapter and owning operation | transparent registered-code propagation plus existing observability owner | rejection prose may be sensitive | remove wrapper text; retain typed absence/retry distinctions |
| `DPC-154` | same helper; response adapter | every root-to-Store internal method | exact nested `IC_CALL_RESPONSE_DECODING_FAILED` | typed nested `InternalError` with formatted decoder cause | 3 — authoritatively typed | maintained IC response adapter | transparent registered-code propagation | sensitive dependency prose | remove wrapper text and propagate the exact nested diagnostic |

The slice adds nine caller-derivable and seven authoritatively typed values. It
adds no sensitive-only or caller-required-unowned value because sensitive
dependency prose is nested inside an already typed cause and is discarded while
that exact cause propagates.

Across all twenty classified slices, the dynamic ledger now contains 154
values: 75 caller-derivable, sixteen sensitive operator-only, 38
authoritatively typed and 25 caller-required but unowned. The 25 unowned values
and their proposed narrow retrieval owners remain unchanged.

## Classified Slice 21: Fleet-Service Peer Binding Adapter

The root-level Component Directory synchronization owner uses only static
messages. Cross-root Fleet-service requester resolution has one formatted
typed topology cause.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-155` | `FleetServicePeerOps::resolve`; protected requester-binding validation | cross-root peer Component provisioning | exact reachable `ComponentTopologyError` selected by `validate_component_binding` | typed protected-binding validation cause | 3 — authoritatively typed | compiled Component Topology and exact derived root/Component bindings | transparent registered-code propagation with the source cause's approved public projection | protected Fleet/root/Spec authority; formatted detail may contain principals or role names | remove the formatter and propagate the exact typed diagnostic; do not allocate a generic peer-binding wrapper |

The slice adds one authoritatively typed value and no caller-derivable,
sensitive-only or caller-required-unowned value.

Across all twenty-one classified slices, the dynamic ledger now contains 155
values: 75 caller-derivable, sixteen sensitive operator-only, 39
authoritatively typed and 25 caller-required but unowned. The 25 unowned values
and their proposed narrow retrieval owners remain unchanged.

## Classified Slice 22: Component Provisioning-Plan Typed Funnel

The core provisioning-plan adapter converts a closed typed error through
`OpsError` into string-first `InternalError`. Bounds and selected identities
are already present in the exact plan/configuration; nested causes must remain
typed instead of entering `Configuration(String)` or `FleetRegistry(String)`.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-156` | plan canonical-byte bound | Component provisioning planning/acceptance | existing `COMPONENT_PROVISIONING_CANONICAL_BYTES_EXCEEDED` | actual canonical bytes | 1 — caller-derivable | exact canonical plan | none | guarded Coordinator/root | discard value; plan reproduces it |
| `DPC-157` | same bound | same routes | same meaning | maximum canonical bytes | 1 — caller-derivable | maintained plan contract | none | public contract | discard value; contract owns it |
| `DPC-158` | plan batch-count bound | same routes | existing `COMPONENT_PROVISIONING_BATCH_COUNT_EXCEEDED` | actual batch count | 1 — caller-derivable | exact plan | none | guarded Coordinator/root | discard value; plan retains it |
| `DPC-159` | same bound | same routes | same meaning | maximum batch count | 1 — caller-derivable | maintained plan contract | none | public contract | discard value |
| `DPC-160` | confirmation-root bound | same routes | existing `COMPONENT_PROVISIONING_CONFIRMATION_ROOT_COUNT_EXCEEDED` | actual confirmation-root count | 1 — caller-derivable | exact plan | none | guarded Coordinator/root | discard value |
| `DPC-161` | same bound | same routes | same meaning | maximum confirmation-root count | 1 — caller-derivable | maintained plan contract | none | public contract | discard value |
| `DPC-162` | placement-count bound | same routes | existing `COMPONENT_PROVISIONING_PLACEMENT_COUNT_EXCEEDED` | actual placement count | 1 — caller-derivable | exact plan | none | guarded Coordinator/root | discard value |
| `DPC-163` | same bound | same routes | same meaning | maximum placement count | 1 — caller-derivable | maintained plan contract | none | public contract | discard value |
| `DPC-164` | Component-entry bound | same routes | existing `COMPONENT_PROVISIONING_COMPONENT_COUNT_EXCEEDED` | actual Component count | 1 — caller-derivable | exact plan | none | guarded Coordinator/root | discard value |
| `DPC-165` | same bound | same routes | same meaning | maximum Component count | 1 — caller-derivable | maintained plan contract | none | public contract | discard value |
| `DPC-166` | duplicate placement | same routes | existing `COMPONENT_PROVISIONING_PLACEMENT_DUPLICATED` | placement ID | 1 — caller-derivable | exact plan | none | guarded Coordinator/root | discard value; plan identifies it |
| `DPC-167` | unknown deployment | same routes | existing `COMPONENT_PROVISIONING_DEPLOYMENT_UNKNOWN` | deployment ID | 1 — caller-derivable | exact plan and checked-in configuration | none | guarded Coordinator/root | discard value; rejected plan identifies it |
| `DPC-168` | `Configuration(String)` from configuration compilation | same routes | exact nested deployment-configuration/topology diagnostic | typed compiler cause flattened to `String` | 3 — authoritatively typed | `ComponentDeploymentConfigurationError` before conversion | transparent registered-code propagation | may contain protected role/Spec/config fields | delete the String variant and retain the typed cause |
| `DPC-169` | `Configuration(String)` undeclared Fleet-service branch | root-batch validation | existing `COMPONENT_PROVISIONING_FLEET_SERVICE_UNKNOWN` | service ID | 1 — caller-derivable | exact checked-in Fleet service/configuration | none | guarded Coordinator/root | replace formatted string with the exact typed decision; configuration identifies the service |
| `DPC-170` | `FleetRegistry(String)` | planning and root-batch validation | exact nested Fleet Registry diagnostic | typed Registry cause flattened to `String` | 3 — authoritatively typed | `FleetRegistryOpsError` before conversion | transparent registered-code propagation | protected Registry details follow source projection | delete the String variant and retain the typed cause |

The slice adds thirteen caller-derivable and two authoritatively typed values.
It adds no sensitive-only or caller-required-unowned value.

## Classified Slice 23: Fleet Registry Typed Funnel

Fleet Registry adapters similarly convert one typed family through
`OpsError`. All Registry fields are retained by the exact input/snapshot or
maintained contract; the nested topology cause remains typed.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-171` | Registry canonical-byte bound | Coordinator Registry compilation/validation | existing `FLEET_REGISTRY_CANONICAL_BYTES_EXCEEDED` | actual canonical bytes | 1 — caller-derivable | exact Registry snapshot | none | guarded Coordinator/root | discard value; snapshot reproduces it |
| `DPC-172` | same bound | same routes | same meaning | maximum canonical bytes | 1 — caller-derivable | maintained Registry contract | none | public contract | discard value |
| `DPC-173` | duplicate root | same routes | existing `FLEET_REGISTRY_ROOT_DUPLICATED` | root principal | 1 — caller-derivable | exact Registry snapshot | none | guarded Registry authority | discard value; snapshot identifies it |
| `DPC-174` | Fleet admission ceiling | same routes | existing `FLEET_REGISTRY_ADMISSIONS_EXCEED_FLEET_MAXIMUM` | Component Spec ID | 1 — caller-derivable | snapshot and compiled topology | none | guarded Registry authority | discard value |
| `DPC-175` | same ceiling | same routes | same meaning | admitted count | 1 — caller-derivable | exact Registry snapshot | none | guarded Registry authority | discard value |
| `DPC-176` | same ceiling | same routes | same meaning | Fleet maximum | 1 — caller-derivable | compiled Component Topology | none | guarded Registry authority | discard value |
| `DPC-177` | admission count overflow | same routes | existing `FLEET_REGISTRY_ADMISSION_COUNT_OVERFLOW` | Component Spec ID | 1 — caller-derivable | exact traversal and compiled topology | none | guarded Registry authority | discard value |
| `DPC-178` | Registry/topology Spec mismatch | same routes | existing `FLEET_REGISTRY_COMPONENT_SPEC_MISMATCH` | Component Spec ID | 1 — caller-derivable | snapshot and compiled topology | none | guarded Registry authority | discard value |
| `DPC-179` | missing draining target | root draining publication | existing `FLEET_REGISTRY_ROOT_DRAIN_TARGET_MISSING` | target root principal | 1 — caller-derivable | exact transition request | none | guarded root/Coordinator | discard value |
| `DPC-180` | missing removal target | logical root removal | existing `FLEET_REGISTRY_ROOT_REMOVE_TARGET_MISSING` | target root principal | 1 — caller-derivable | exact transition request | none | guarded root/Coordinator | discard value |
| `DPC-181` | genesis App mismatch | genesis compilation | existing `FLEET_REGISTRY_GENESIS_APP_MISMATCH` | received App ID | 1 — caller-derivable | exact genesis authority | none | guarded host/Coordinator | discard value |
| `DPC-182` | same mismatch | same route | same meaning | configured expected App ID | 1 — caller-derivable | checked-in App configuration | none | guarded host/Coordinator | discard value |
| `DPC-183` | genesis epoch invalid | same route | existing `FLEET_REGISTRY_GENESIS_AUTHORITY_EPOCH_INVALID` | received epoch | 1 — caller-derivable | exact genesis authority | none | guarded host/Coordinator | discard value |
| `DPC-184` | service-member order invalid | service publication | existing `FLEET_REGISTRY_SERVICE_MEMBER_ORDER_NONCANONICAL` | service ID | 1 — caller-derivable | exact Registry service row | none | guarded Registry authority | discard value |
| `DPC-185` | empty service | same route | existing `FLEET_REGISTRY_SERVICE_EMPTY` | service ID | 1 — caller-derivable | exact Registry service row | none | guarded Registry authority | discard value |
| `DPC-186` | service Spec mismatch | same route | existing `FLEET_REGISTRY_SERVICE_SPEC_MISMATCH` | service ID | 1 — caller-derivable | exact Registry service row | none | guarded Registry authority | discard value |
| `DPC-187` | service mode mismatch | same route | existing `FLEET_REGISTRY_SERVICE_MODE_MISMATCH` | service ID | 1 — caller-derivable | exact Registry service row | none | guarded Registry authority | discard value |
| `DPC-188` | service placement mismatch | same route | existing `FLEET_REGISTRY_SERVICE_PLACEMENT_MISMATCH` | service ID | 1 — caller-derivable | exact Registry service row | none | guarded Registry authority | discard value |
| `DPC-189` | service root mismatch | same route | existing `FLEET_REGISTRY_SERVICE_ROOT_MISMATCH` | service ID | 1 — caller-derivable | exact Registry service row | none | guarded Registry authority | discard value |
| `DPC-190` | duplicate service Component | same route | existing `FLEET_REGISTRY_SERVICE_COMPONENT_DUPLICATED` | Component instance ID | 1 — caller-derivable | exact Registry service membership | none | guarded Registry authority | discard value |
| `DPC-191` | duplicate service Canister | same route | existing `FLEET_REGISTRY_SERVICE_CANISTER_DUPLICATED` | Canister principal | 1 — caller-derivable | exact Registry service membership | none | guarded Registry authority | discard value |
| `DPC-192` | root release-build mismatch | root validation/publication | existing `FLEET_REGISTRY_ROOT_RELEASE_BUILD_MISMATCH` | expected release build ID | 1 — caller-derivable | canonical first root row | none | guarded Registry authority | discard value |
| `DPC-193` | same mismatch | same routes | same meaning | received release build ID | 1 — caller-derivable | exact conflicting root row | none | guarded Registry authority | discard value |
| `DPC-194` | `FleetRegistryOpsError::Topology` | every Registry validation adapter | exact path-qualified topology diagnostic | typed `ComponentTopologyError` | 3 — authoritatively typed | compiled topology validator | transparent registered-code propagation with the source cause's approved projection | protected topology details follow source projection | preserve the typed cause; never format it into Registry prose |

The slice adds twenty-three caller-derivable and one authoritatively typed
value. It adds no sensitive-only or caller-required-unowned value.

Across all twenty-three classified slices, the dynamic ledger now contains 194
values: 111 caller-derivable, sixteen sensitive operator-only, 42
authoritatively typed and 25 caller-required but unowned. The 25 unowned values
and their proposed narrow retrieval owners remain unchanged.
