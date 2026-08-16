# Canic 0.102 Dynamic Public Context Inventory

Date: 2026-08-14

## Status

This is the closed evidence-only B1 ledger. It allocates no diagnostic code and
changes no endpoint. The current-source census and row-by-row classification
were completed and approved before B2 materialized the separate permanent
allocation; this producer evidence remains repository-only.

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
error and therefore correctly has no dynamic-value row. This slice left the
Store ops conversion open for individual field review; Slices 2 and 3 plus the
later Template-manifest semantic ledger now close it without treating the
aggregate `message` as one value.

## Classified Slice 2: Wasm Store Manifest And Capacity Conversion

This slice follows the maintained `TemplateManifestOpsError -> InternalError`
conversion through the variants shared by the root bootstrap buffer and the
separate Wasm Store. The two root-control-plane-only approved-manifest variants
are classified separately in Slice 3; they are not silently included here.

The diagnostic names below are qualified by
[template-manifest-ops-leaves.md](template-manifest-ops-leaves.md). They still
reserve no number until the complete allocation receives maintainer approval.

| ID | Source and owner | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-006` | `crates/canic-control-plane/src/ops/storage/template/mod.rs`; `TemplateManifestOpsError::TemplateChunkSetMissing` conversion | Store `info`, `chunk` or `publish_chunk`; root bootstrap/publication routes | qualified `WASM_STORE_CHUNK_SET_MISSING` | requested template release key | 1 — caller-derivable | exact `template_id` and `version` in the request or protected manifest | none | public | discard the formatted key; preserve the exact missing-set code |
| `DPC-007` | same conversion; `TemplateChunkMissing` | Store `chunk`; root bootstrap/publication routes | qualified `WASM_STORE_CHUNK_MISSING` | requested template release plus chunk index | 1 — caller-derivable | exact release and index in the request or protected manifest traversal | none | public | discard the formatted key; preserve the exact missing-chunk code |
| `DPC-008` | same conversion; `TemplateChunkSetEmpty` | Store `prepare`; root `template_prepare_admin` and bootstrap routes | qualified `WASM_STORE_CHUNK_SET_EMPTY` | requested template release key | 1 — caller-derivable | exact `template_id` and `version` in the submitted prepare request or protected manifest | none | public | discard the formatted key; preserve the empty-set code |
| `DPC-009` | same conversion; `PayloadHashMismatch` | root bootstrap/status and Store publication-admin routes | qualified `WASM_STORE_PAYLOAD_HASH_MISMATCH` | protected template release key | 1 — caller-derivable | exact release in the immutable root release-set manifest | none | guarded operator | discard the formatted key; keep the mismatch identity and protected manifest evidence |
| `DPC-010` | same conversion; `PayloadSizeMismatch` | root bootstrap/status and Store publication-admin routes | qualified `WASM_STORE_PAYLOAD_SIZE_MISMATCH` | protected template release key | 1 — caller-derivable | exact release in the immutable root release-set manifest | none | guarded operator | discard the formatted key; keep the mismatch identity and protected manifest evidence |
| `DPC-011` | same conversion; `ChunkIndexOverflow` | Store or root `prepare`; root bootstrap/publication routes | qualified `WASM_STORE_CHUNK_INDEX_OVERFLOW` | requested template release key | 1 — caller-derivable | exact release in the prepare request or protected manifest | none | public for request input; otherwise guarded operator | discard the formatted key; preserve the exact index-overflow code |
| `DPC-012` | same conversion; `TemplateChunkIndexOutOfRange` | Store `chunk` or `publish_chunk`; root `template_publish_chunk_admin` | qualified `WASM_STORE_CHUNK_INDEX_OUT_OF_RANGE` | requested template release key | 1 — caller-derivable | exact `template_id` and `version` in the request | none | public | discard the formatted key; preserve the exact range code |
| `DPC-013` | same conversion; `TemplateChunkIndexOutOfRange` | same routes as `DPC-012` | qualified `WASM_STORE_CHUNK_INDEX_OUT_OF_RANGE` | requested chunk index | 1 — caller-derivable | exact `chunk_index` in the request | none | public | discard the interpolation; preserve the exact range code |
| `DPC-014` | same conversion; `TemplateChunkHashMismatch` | Store `chunk` or `publish_chunk`; root bootstrap/publication routes | qualified `WASM_STORE_CHUNK_HASH_MISMATCH` | template release plus chunk index | 1 — caller-derivable | exact request or protected manifest traversal identifies the chunk | none | guarded operator; safe to the exact root caller | discard the formatted key; keep the exact hash-mismatch code and protected hash evidence |
| `DPC-015` | same conversion; `WasmStoreCapacityExceeded` | Store `prepare`, `stage_manifest` or `publish_chunk` | qualified `WASM_STORE_BYTE_CAPACITY_EXCEEDED` | rejected canonical `projected_bytes` | 4 — caller-required but unowned | none; current status exposes occupied, maximum and remaining bytes but not the rejected canonical projection | request-scoped `WasmStorePublicationCapacityProjectionResponse.projected_store_bytes`, computed by the same canonical Store ops path without a global last-error slot | guarded operator; root-only | add the exact typed preflight owner before removing this value from the public message |
| `DPC-016` | same conversion; `WasmStoreCapacityExceeded` | same routes as `DPC-015` | qualified `WASM_STORE_BYTE_CAPACITY_EXCEEDED` | configured `max_store_bytes` | 3 — authoritatively typed | `WasmStoreStatusResponse.max_store_bytes` via `canic_wasm_store_status` | none | guarded operator; root-only | remove it from the diagnostic and use the existing Store status |
| `DPC-017` | same conversion; `WasmStoreTemplateLimitExceeded` | Store `prepare` or `stage_manifest` | qualified `WASM_STORE_TEMPLATE_CAPACITY_EXCEEDED` | prospective distinct-template count | 1 — caller-derivable | submitted template identity plus `WasmStoreStatusResponse.templates` | none | guarded operator; root-only | discard the interpolation; the caller can derive the prospective count from request plus status |
| `DPC-018` | same conversion; `WasmStoreTemplateLimitExceeded` | same routes as `DPC-017` | qualified `WASM_STORE_TEMPLATE_CAPACITY_EXCEEDED` | configured maximum template count | 3 — authoritatively typed | `WasmStoreStatusResponse.max_templates` via `canic_wasm_store_status` | none | guarded operator; root-only | remove it from the diagnostic and use the existing Store status |
| `DPC-019` | same conversion; `WasmStoreVersionLimitExceeded` | Store `prepare` or `stage_manifest` | qualified `WASM_STORE_VERSION_CAPACITY_EXCEEDED` | submitted template identity | 1 — caller-derivable | exact `template_id` in the request | none | guarded operator; root-only | discard the interpolation; preserve the exact version-capacity code |
| `DPC-020` | same conversion; `WasmStoreVersionLimitExceeded` | same routes as `DPC-019` | qualified `WASM_STORE_VERSION_CAPACITY_EXCEEDED` | prospective retained-version count | 1 — caller-derivable | submitted release plus the matching `WasmStoreStatusResponse.templates[].versions` count | none | guarded operator; root-only | discard the interpolation; the caller can derive the prospective count from request plus status |
| `DPC-021` | same conversion; `WasmStoreVersionLimitExceeded` | same routes as `DPC-019` | qualified `WASM_STORE_VERSION_CAPACITY_EXCEEDED` | configured per-template version maximum | 3 — authoritatively typed | `WasmStoreStatusResponse.max_template_versions_per_template` via `canic_wasm_store_status` | none | guarded operator; root-only | remove it from the diagnostic and use the existing Store status |

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
| `DPC-022` | `crates/canic-control-plane/src/ops/storage/template/mod.rs`; `TemplateManifestOpsError::ApprovedManifestMissing` | `canic_root_store_bootstrap`, `canic_root_store_bootstrap_status`; control-plane install resolver if used | qualified `WASM_STORE_APPROVED_MANIFEST_MISSING` | admitted Canister role | 1 — caller-derivable | exact role in the protected root release-set manifest or resolver request | none | guarded operator; safe to the exact provisioning caller | discard the interpolation; preserve the exact approved-manifest-missing code |
| `DPC-023` | same conversion; `ApprovedManifestConflict` | same routes as `DPC-022` | qualified `WASM_STORE_APPROVED_MANIFEST_CONFLICT` | admitted Canister role | 1 — caller-derivable | exact role in the protected root release-set manifest or resolver request | none | guarded operator; safe to the exact provisioning caller | discard the interpolation; preserve the exact approved-manifest-conflict code |

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
| `DPC-046` | `CapacityExceeded.release` | root Store bootstrap/publication routes | qualified `WASM_STORE_BYTE_CAPACITY_EXCEEDED` | protected template release label | 1 — caller-derivable | release-set manifest template identity and version | none | guarded operator | discard label; preserve exact capacity code |
| `DPC-047` | `CapacityExceeded.target` | same routes as `DPC-046` | same meaning | selected Store binding | 3 — authoritatively typed | `WasmStoreOverviewStoreResponse.binding` | none | guarded operator | remove from diagnostic; retrieve through overview |
| `DPC-048` | `CapacityExceeded.payload_size_bytes` | same routes as `DPC-046` | same meaning | protected artifact payload bytes | 1 — caller-derivable | `TemplateManifestResponse.payload_size_bytes` from the release-set projection | none | guarded operator | discard interpolation; manifest retains value |
| `DPC-049` | `CapacityExceeded.remaining_store_bytes` | same routes as `DPC-046` | same meaning | observed live encoded Store headroom | 4 — caller-required but unowned | sibling `WasmStoreStatusResponse.remaining_store_bytes` is root-readable, but the affected root controller cannot retrieve it; root overview exposes different approved-payload accounting | request-scoped `WasmStorePublicationCapacityProjectionResponse.remaining_store_bytes` | guarded operator | add exact live-Store capacity preflight before removing this value from the public message |
| `DPC-050` | `ChunkHashMismatch.template_id` | root Store publication routes | qualified `WASM_STORE_CHUNK_HASH_MISMATCH` | protected template identity | 1 — caller-derivable | exact release-set manifest | none | guarded operator | discard interpolation; manifest retains identity |
| `DPC-051` | `ChunkHashMismatch.chunk_index` | same routes as `DPC-050` | same meaning | protected traversal index | 1 — caller-derivable | deterministic position in the manifest chunk-hash vector | none | guarded operator | discard interpolation; exact code and manifest traversal retain action |
| `DPC-052` | `ChunkHashMismatch.store_pid` | same routes as `DPC-050` | same meaning | sibling Store principal | 2 — sensitive operator-only | controller-guarded `WasmStoreOverviewStoreResponse.pid` | none | prohibited on unguarded public route | mask from diagnostic; retain in guarded overview and correlated operation evidence |
| `DPC-053` | `ChunkIndexOverflow.template_id` | root Store publication routes | qualified `WASM_STORE_CHUNK_INDEX_OVERFLOW` | protected template identity | 1 — caller-derivable | exact release-set manifest | none | guarded operator | discard interpolation; preserve exact overflow code |
| `DPC-054` | `ExactReleaseMissing.role` | root Store bootstrap/publication reconciliation | qualified `WASM_STORE_EXACT_RELEASE_MISSING` or `WASM_STORE_GC_WRITE_FENCED`, selected by the source predicate | admitted Canister role | 1 — caller-derivable | exact release-set manifest | none | guarded operator | split the combined branch; discard interpolation and retain the exact release/GC authority |
| `DPC-055` | `ExactReleaseMissing.template_id` | same routes as `DPC-054` | same meaning | expected template identity | 1 — caller-derivable | exact release-set manifest | none | guarded operator | discard interpolation; manifest retains identity |
| `DPC-056` | `ExactReleaseMissing.version` | same routes as `DPC-054` | same meaning | expected template version | 1 — caller-derivable | exact release-set manifest | none | guarded operator | discard interpolation; manifest retains version |
| `DPC-057` | `ExactReleaseMissing.expected_binding` | same routes as `DPC-054` | same meaning | expected Store binding | 1 — caller-derivable | protected manifest Store binding | none | guarded operator | discard interpolation; manifest retains binding |
| `DPC-058` | `ReleaseConflict.template_id` | root Store bootstrap/publication routes | qualified `WASM_STORE_RELEASE_CONFLICT` | submitted template identity | 1 — caller-derivable | exact manifest being published | none | guarded operator | discard interpolation; request authority retains identity |
| `DPC-059` | `ReleaseConflict.version` | same routes as `DPC-058` | same meaning | submitted template version | 1 — caller-derivable | exact manifest being published | none | guarded operator | discard interpolation; request authority retains version |
| `DPC-060` | `ReleaseConflict.binding` | same routes as `DPC-058` | same meaning | selected Store binding | 1 — caller-derivable | protected publication target and root Store state | none | guarded operator | discard interpolation; publication authority retains binding |
| `DPC-061` | `ReleaseConflict.existing_payload_hash` | same routes as `DPC-058` | same meaning | conflicting live catalog payload hash | 4 — caller-required but unowned | root-internal `PublicationStoreSnapshot.catalog`; not retrievable by the controller after this failure | request-scoped `WasmStoreReleaseInspectionResponse.observed_payload_hash` | guarded operator | add exact release inspection before removing the hash from the public message |
| `DPC-062` | `ReleaseConflict.existing_payload_size_bytes` | same routes as `DPC-058` | same meaning | conflicting live catalog payload size | 4 — caller-required but unowned | root-internal `PublicationStoreSnapshot.catalog`; not retrievable by the controller after this failure | request-scoped `WasmStoreReleaseInspectionResponse.observed_payload_size_bytes` | guarded operator | add exact release inspection before removing the size from the public message |
| `DPC-063` | `StoreNotWritable.binding` | root Store publication/removal routes | qualified `WASM_STORE_GC_WRITE_FENCED` | selected Store binding | 3 — authoritatively typed | `WasmStoreOverviewStoreResponse.binding` | none | guarded operator | remove from diagnostic; retrieve through overview |
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

The binding/release constructions under
`PublicationWorkflowError::InvalidState(String)` are closed in
[publication-binding-release-leaves.md](publication-binding-release-leaves.md).
Its 55 GC invalid-state constructions remain open and each static predicate
must receive an exact disposition at its construction site. The
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
This slice left `AuthApi::map_auth_error` for the transitive auth pass; Slices
40–46 later close it without misclassifying the formatted aggregate as one
datum.

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
| `DPC-079` | `single_store_catalog` cardinality failure | `canic_root_store_bootstrap_status` and bootstrap verification | qualified `WASM_STORE_SINGLE_ADOPTED_STORE_REQUIRED` | current root-owned Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` via controller-guarded `canic_wasm_store_overview` | none | controller-only | remove count from diagnostic; overview retains exact inventory |
| `DPC-080` | `require_active_publication_store` sole-binding failure | root Store bootstrap and `canic_wasm_store_admin` publication | qualified `WASM_STORE_SOLE_ACTIVE_PUBLICATION_BINDING_REQUIRED` | selected Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.publication` plus `stores[].binding` | none | controller-only | remove binding from diagnostic; overview retains binding and slots |
| `DPC-081` | `pin_initial_publication_store` cardinality failure | root Store bootstrap | qualified `WASM_STORE_SINGLE_ADOPTED_STORE_REQUIRED` | current adopted Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; overview retains exact inventory |
| `DPC-082` | `pin_initial_publication_store` binding mismatch | root Store bootstrap | qualified `WASM_STORE_ADOPTED_BINDING_MISMATCH` | selected snapshot binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.publication` plus the selected `stores[].binding` | none | controller-only | remove selected binding from diagnostic; overview retains publication authority |
| `DPC-083` | same branch as `DPC-082` | root Store bootstrap | same meaning | observed adopted Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove observed binding from diagnostic; overview retains current inventory |
| `DPC-084` | `root_activation_wasm_store` cardinality failure | controller-guarded `canic_prepare_fleet_activation` | provisional `FLEET_ACTIVATION_SINGLE_STORE_REQUIRED` | current root-owned Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; overview remains the Store inventory owner |
| `DPC-085` | `snapshot_adopted_wasm_store` cardinality failure | root Store bootstrap and `canic_wasm_store_admin` publication | qualified `WASM_STORE_SINGLE_ADOPTED_STORE_REQUIRED` | current adopted Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; overview remains the Store inventory owner |

Slice totals are seven authoritatively typed values and no values in the other
three classes. The same cardinality predicate appears at distinct journeys, so
the rows remain separate even if allocation later reuses one exact meaning.
The overview is observation only; recording it as the retrieval owner does not
move Store inventory or publication authority out of root stable state.

Publication GC still owns the remaining dynamic `InvalidState(String)`
producers. Its static invalid-state leaves still require exact diagnostic
allocation, while each formatted GC field must be classified independently.
Later publication and auth slices close the nested transport causes and auth
formatter.

## Classified Slice 9: Store GC Fence And Reclamation Authority

This bounded GC slice covers final-inventory quiescence, removal reverification,
Store reclamation, reclaimed-binding verification and their shared runtime-GC
reconciliation/lookup helpers. It does not yet cover binding-slot finalization,
cycle reclamation or physical deletion.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-086` | final-inventory quiescence cardinality failure | controller root final-inventory journey | qualified `ROOT_FINAL_INVENTORY_SINGLE_STORE_REQUIRED` | current root-owned Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; root overview retains inventory |
| `DPC-087` | final-inventory GC-lineage mismatch | same route as `DPC-086` | qualified `ROOT_FINAL_INVENTORY_STORE_GC_LINEAGE_MISMATCH` | persisted runtime GC mode | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].gc.mode` | none | controller-only | remove runtime mode from diagnostic; overview retains it |
| `DPC-088` | same branch as `DPC-087` | same route | same meaning | live sibling Store GC mode | 4 — caller-required but unowned | internal root-to-Store `WasmStoreStatusResponse.gc.mode`; not retrievable by the external root controller | guarded root `WasmStoreLifecycleInspectionResponse.live_gc` bound to exact Store binding and principal | controller-only | add exact root-proxied live GC inspection before removing this value |
| `DPC-089` | prepared-GC authority persistence failure | same route as `DPC-086` | qualified `ROOT_FINAL_INVENTORY_STORE_GC_PERSIST_FAILED` | root-owned Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove binding from diagnostic; overview retains it |
| `DPC-090` | post-persist runtime/live GC mismatch | same route as `DPC-086` | qualified `ROOT_FINAL_INVENTORY_STORE_GC_AUTHORITY_MISMATCH` | root-owned Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove binding from diagnostic; overview retains it |
| `DPC-091` | removal reverification cardinality failure | `canic_fleet_subnet_root_removal_publish` | qualified `ROOT_REMOVAL_SINGLE_STORE_REQUIRED` | current root-owned Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; overview retains inventory |
| `DPC-092` | removal runtime/live GC mismatch | same route as `DPC-091` | qualified `ROOT_REMOVAL_STORE_GC_AUTHORITY_MISMATCH` | root-owned Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove binding from diagnostic; overview retains it |
| `DPC-093` | Store reclamation cardinality failure | `canic_fleet_subnet_root_store_reclaim` | qualified `ROOT_STORE_RECLAMATION_SINGLE_STORE_REQUIRED` | current root-owned Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; overview retains inventory |
| `DPC-094` | Store reclamation terminal-mode failure | same route as `DPC-093` | qualified `ROOT_STORE_RECLAMATION_GC_INCOMPLETE` | live sibling Store GC mode | 4 — caller-required but unowned | internal root-to-Store `WasmStoreStatusResponse.gc.mode`; not retrievable by the external root controller | same guarded root `WasmStoreLifecycleInspectionResponse.live_gc` as `DPC-088` | controller-only | add exact root-proxied live GC inspection before removing this value |
| `DPC-095` | reclaimed-binding verification cardinality failure | `canic_fleet_subnet_root_store_binding_finalize` | qualified `ROOT_STORE_BINDING_FINALIZATION_SINGLE_STORE_REQUIRED` | current root-owned Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; overview retains inventory |
| `DPC-096` | `reconcile_single_root_store_gc` persistence failure | Store reclamation route | qualified `ROOT_STORE_GC_RECONCILIATION_PERSIST_FAILED` | root-owned Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove binding from diagnostic; overview retains it |
| `DPC-097` | post-reconciliation runtime/live mismatch | Store reclamation route | qualified `ROOT_STORE_GC_RECONCILIATION_MISMATCH` | root-owned Store binding | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove binding from diagnostic; overview retains it |
| `DPC-098` | `runtime_store` missing-binding lookup | root Store reclamation, binding-finalization and deletion journeys | qualified `ROOT_STORE_RUNTIME_BINDING_MISSING` | requested root-owned Store binding | 3 — authoritatively typed | protected lifecycle intent plus `WasmStoreOverviewResponse.stores[].binding` | none | controller-only | remove binding from diagnostic; request/status and overview retain it |

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
| `DPC-099` | `binding_finalization_transition_error.transition` | `canic_fleet_subnet_root_store_binding_finalize` | one of four qualified Store binding-transition failures | one of four static binding transition labels | 4 — caller-required but unowned | none; free-form helper argument is the current discriminator | exact registered diagnostic selected at the four construction sites | controller-only | split clear-active, retire-detached, finalize-retired and terminal-convergence failures into exact codes |
| `DPC-100` | post-reclamation retained-target failure | `canic_fleet_subnet_root_store_delete` cycle-reclamation phase | qualified `ROOT_STORE_CYCLE_RECLAMATION_TARGET_EXCEEDED` | observed live Store cycles after reclamation | 4 — caller-required but unowned | private live status observation; terminal deletion response does not exist yet | guarded operation-scoped `FleetSubnetRootStoreDeletionProgressResponse.observed_cycles_after_reclamation` | controller-only financial evidence | add typed in-progress evidence before removing the numeric value |
| `DPC-101` | same branch as `DPC-100` | same route | same meaning | durable retained-cycle target | 4 — caller-required but unowned | private root Store deletion intent; terminal response exposes it only after later physical deletion | guarded operation-scoped `FleetSubnetRootStoreDeletionProgressResponse.retained_cycles_target` | controller-only financial evidence | add typed in-progress evidence before removing the numeric value |
| `DPC-102` | `status_cycles.label` overflow helper | Store deletion preparation, cycle reclamation and physical deletion | one of six qualified Store status numeric-overflow meanings | one of six static Canister-status field labels | 4 — caller-required but unowned | none; free-form helper argument is the current discriminator | exact registered diagnostic selected at each static status-field call site | controller-only | replace label formatting with exact per-field codes; do not add text detail |

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
| `DPC-103` | `single_finalized_runtime_store` cardinality failure | `canic_fleet_subnet_root_store_delete` preparation phase | qualified `ROOT_STORE_DELETION_SINGLE_RUNTIME_STORE_REQUIRED` | current root-owned runtime Store count | 3 — authoritatively typed | `WasmStoreOverviewResponse.stores` | none | controller-only | remove count from diagnostic; root overview retains exact runtime inventory |

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
| `DPC-104` | `management stored_chunks` insufficient-liquid-cycles cause | root Store bootstrap and active-release publication | qualified `WASM_STORE_STORED_CHUNKS_LIQUID_CYCLES_INSUFFICIENT` | liquid cycles available when the call was attempted | 4 — caller-required but unowned | transient `ic-cdk::call::InsufficientLiquidCycleBalance`; flattened before workflow handling | guarded operation-scoped `WasmStorePublicationAttemptStatusResponse.observed_available_cycles` | controller-only financial evidence | persist the value against the exact publication attempt before returning the exact diagnostic |
| `DPC-105` | same cause as `DPC-104` | same routes | same meaning | cycles required for the exact call | 4 — caller-required but unowned | same transient call error | `WasmStorePublicationAttemptStatusResponse.required_call_cycles` on the same attempt | controller-only financial evidence | persist the value with `DPC-104`; do not put cycle amounts in the diagnostic |
| `DPC-106` | `management stored_chunks` rejection cause | same routes | qualified exact stored-chunks rejection family selected by recognized IC reject class | raw IC reject code | 4 — caller-required but unowned | transient `ic-cdk::call::CallRejected`; current metrics retain only `Infra` | exact registered diagnostic for every recognized reject class; `WasmStorePublicationAttemptStatusResponse.unrecognized_reject_code` for an unknown raw value | controller-only; the numeric IC reject code is safe, raw reject prose is not | exhaustively type recognized reject classes and retain only an unrecognized raw number in the exact attempt status |
| `DPC-107` | same rejection as `DPC-106` | same routes | same rejection meaning | replica reject message | 2 — sensitive operator-only | no safe typed owner; current public message is the only retention | none | prohibited public raw platform text | discard the text; exact rejection diagnostic and optional unknown numeric code are sufficient |
| `DPC-108` | `management stored_chunks` request encoding failure | same routes | qualified `WASM_STORE_STORED_CHUNKS_REQUEST_ENCODE_FAILED` | dependency-owned Candid encode cause | 2 — sensitive operator-only | no typed owner | none | prohibited public implementation detail | discard the dependency prose; the surface-specific exact diagnostic identifies the failed contract phase |
| `DPC-109` | `management stored_chunks` response decoding failure | same routes | qualified `WASM_STORE_STORED_CHUNKS_RESPONSE_DECODE_FAILED` | Rust `type_name` of the expected response | 2 — sensitive operator-only | no boundary owner; the maintained adapter source statically selects the response DTO | none | prohibited public implementation/package detail | discard the Rust type name; the surface-specific exact diagnostic identifies the maintained response contract |
| `DPC-110` | same decode failure as `DPC-109` | same routes | same meaning | dependency-owned Candid decode cause | 2 — sensitive operator-only | no typed owner | none | prohibited public implementation detail and possibly remote payload context | discard the dependency prose; retain only the exact response-decode diagnostic |
| `DPC-111` | `management upload_chunk` insufficient-liquid-cycles cause | root Store bootstrap and active-release publication | qualified `WASM_STORE_UPLOAD_CHUNK_LIQUID_CYCLES_INSUFFICIENT` | liquid cycles available when the chunk call was attempted | 4 — caller-required but unowned | transient `ic-cdk::call::InsufficientLiquidCycleBalance`; flattened before workflow handling | guarded operation-scoped `WasmStorePublicationAttemptStatusResponse.observed_available_cycles` | controller-only financial evidence | persist the value against the exact release and chunk attempt before returning the exact diagnostic |
| `DPC-112` | same cause as `DPC-111` | same routes | same meaning | cycles required for the exact chunk call | 4 — caller-required but unowned | same transient call error | `WasmStorePublicationAttemptStatusResponse.required_call_cycles` on the same attempt | controller-only financial evidence | persist the value with `DPC-111`; the exact chunk identity is required because request size affects cost |
| `DPC-113` | `management upload_chunk` rejection cause | same routes | qualified exact upload-chunk rejection family selected by recognized IC reject class | raw IC reject code | 4 — caller-required but unowned | transient `ic-cdk::call::CallRejected`; current metrics retain only `Infra` | exact registered diagnostic for every recognized reject class; `WasmStorePublicationAttemptStatusResponse.unrecognized_reject_code` for an unknown raw value | controller-only; the numeric IC reject code is safe, raw reject prose is not | exhaustively type recognized reject classes and retain only an unrecognized raw number in the exact attempt status |
| `DPC-114` | same rejection as `DPC-113` | same routes | same rejection meaning | replica reject message | 2 — sensitive operator-only | no safe typed owner; current public message is the only retention | none | prohibited public raw platform text | discard the text; exact rejection diagnostic and optional unknown numeric code are sufficient |
| `DPC-115` | `management upload_chunk` request encoding failure | same routes | qualified `WASM_STORE_UPLOAD_CHUNK_REQUEST_ENCODE_FAILED` | dependency-owned Candid encode cause | 2 — sensitive operator-only | no typed owner | none | prohibited public implementation detail | discard the dependency prose; the surface-specific exact diagnostic identifies the failed contract phase |
| `DPC-116` | `management upload_chunk` response decoding failure | same routes | qualified `WASM_STORE_UPLOAD_CHUNK_RESPONSE_DECODE_FAILED` | Rust `type_name` of the expected response | 2 — sensitive operator-only | no boundary owner; the maintained adapter source statically selects the response DTO | none | prohibited public implementation/package detail | discard the Rust type name; the surface-specific exact diagnostic identifies the maintained response contract |
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

## Classified Slice 24: Runtime Authentication Build Contracts

Four non-root startup failures interpolate the configured Canister role. The
role is already fixed by checked-in Component topology and the exact build
target; it is not diagnostic authority.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-195` | `RuntimeAuthWorkflow::ensure_nonroot_crypto_contract`; issuer signature creation absent | managed-runtime crypto preflight | provisional `AUTH_ISSUER_CANISTER_SIGNATURE_CREATION_UNAVAILABLE` | configured Canister role | 1 — caller-derivable | checked-in Component Spec/child role and exact build target | none | visible to the App/build operator | discard role text; exact code identifies the missing compiled capability |
| `DPC-196` | `ensure_auth_proof_verifier_support_contract`; root signature verification absent | same route | provisional `AUTH_ROOT_CANISTER_SIGNATURE_VERIFICATION_UNAVAILABLE` | configured Canister role | 1 — caller-derivable | same topology/build authority | none | visible to the App/build operator | discard role text; build target identifies the affected role |
| `DPC-197` | same contract; issuer signature verification absent | same route | provisional `AUTH_ISSUER_CANISTER_SIGNATURE_VERIFICATION_UNAVAILABLE` | configured Canister role | 1 — caller-derivable | same topology/build authority | none | visible to the App/build operator | discard role text; exact code selects issuer-proof verification capability |
| `DPC-198` | same contract; chain-key proof support absent | same route | existing `AUTH_CHAIN_KEY_CRYPTO_UNAVAILABLE` | configured Canister role | 1 — caller-derivable | same topology/build authority | none | visible to the App/build operator | discard role text; exact build and code identify the missing chain-key capability |

The slice adds four caller-derivable values and no sensitive, typed or unowned
value. Root contract messages contain no interpolated public value. Caller,
Subnet, capability and time fields in RPC authorization are structured log
context rather than public error-message data and therefore add no row.

Across all twenty-four classified slices, the dynamic ledger now contains 198
values: 115 caller-derivable, sixteen sensitive operator-only, 42
authoritatively typed and 25 caller-required but unowned. The 25 unowned values
and their proposed narrow retrieval owners remain unchanged.

## Classified Slice 25: Runtime Auth Prepare Admission

Role-attestation admission formats request values together with the caller's
exact active member authority. The public delegated-token policy adapter also
formats the two fields from its one dynamic producer-reachable typed variant.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-199` | role-attestation TTL bound | role-attestation preparation | provisional `AUTH_ROLE_ATTESTATION_TTL_INVALID` | configured maximum TTL | 1 — caller-derivable | checked-in role-attestation configuration | none | visible to the affected Fleet member | discard value; the exact build/configuration retains the ceiling |
| `DPC-200` | same bound | same route | same meaning | requested TTL | 1 — caller-derivable | exact prepare request | none | public to the affected caller | discard value; request retains it |
| `DPC-201` | subject/caller mismatch | same route | existing `AUTH_ATTESTATION_SUBJECT_MISMATCH` | requested subject | 1 — caller-derivable | exact prepare request | none | public to the affected caller | discard value |
| `DPC-202` | same mismatch | same route | same meaning | transport caller | 1 — caller-derivable | IC caller | none | public to the affected caller | discard value |
| `DPC-203` | active-member caller mismatch | same route | provisional `AUTH_ATTESTATION_MEMBER_CALLER_MISMATCH` | transport caller | 1 — caller-derivable | IC caller | none | public to the affected caller | discard value |
| `DPC-204` | same mismatch | same route | same meaning | registered member Canister | 1 — caller-derivable | caller's protected active Component binding | none | visible to the affected Fleet member | discard value; protected binding retains it |
| `DPC-205` | role mismatch | same route | provisional `AUTH_ATTESTATION_ROLE_MISMATCH` | requested subject | 1 — caller-derivable | exact prepare request | none | public to the affected caller | discard value |
| `DPC-206` | same mismatch | same route | same meaning | requested role | 1 — caller-derivable | exact prepare request | none | public to the affected caller | discard value |
| `DPC-207` | same mismatch | same route | same meaning | registered role | 1 — caller-derivable | caller's protected active Component binding | none | visible to the affected Fleet member | discard value; protected binding retains it |
| `DPC-208` | Subnet mismatch | same route | existing `AUTH_ATTESTATION_SUBNET_MISMATCH` | requested subject | 1 — caller-derivable | exact prepare request | none | public to the affected caller | discard value |
| `DPC-209` | same mismatch | same route | same meaning | requested Subnet | 1 — caller-derivable | exact prepare request | none | public to the affected caller | discard value |
| `DPC-210` | same mismatch | same route | same meaning | registered placement Subnet | 1 — caller-derivable | caller's protected active Component binding | none | visible to the affected Fleet member | discard value; protected binding retains it |
| `DPC-211` | public-scope policy rejection | delegated-token preparation | existing `AUTH_PUBLIC_SCOPE_NOT_SELF_GRANTABLE` | requested role | 1 — caller-derivable | exact delegated grant | none | public to the affected caller | discard value; request retains it |
| `DPC-212` | same rejection | same route | same meaning | requested scope | 1 — caller-derivable | exact delegated grant | none | public to the affected caller | discard value; request retains it |

The slice adds fourteen caller-derivable values and no sensitive, typed or
unowned value. The caller's protected member binding and checked-in TTL policy
remain the authority; request-presented role and Subnet fields never replace
them.

Across all twenty-five classified slices, the dynamic ledger now contains 212
values: 129 caller-derivable, sixteen sensitive operator-only, 42
authoritatively typed and 25 caller-required but unowned. The 25 unowned values
and their proposed narrow retrieval owners remain unchanged.

## Classified Slice 26: Runtime Auth Prepare Replay

Authentication prepare replay formats request metadata, typed replay state and
typed response-codec causes. Command labels and quotas are fixed by the exact
endpoint or maintained replay contract.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-213` | replay expiry addition overflow | both authentication prepare routes | provisional `AUTH_PREPARE_REPLAY_EXPIRY_OVERFLOW` | helper-call-site-supplied static overflow message | 1 — caller-derivable | exact prepare command/call site | none | public contract | delete message parameter; exact registered code owns the failure |
| `DPC-214` | role-attestation replay TTL bound | role-attestation preparation | provisional `AUTH_ROLE_ATTESTATION_REPLAY_TTL_EXCEEDED` | requested replay TTL | 1 — caller-derivable | exact request metadata | none | public to the affected caller | discard value; request retains it |
| `DPC-215` | same bound | same route | same meaning | maximum replay TTL | 1 — caller-derivable | maintained replay contract | none | public contract | discard value |
| `DPC-216` | delegated-token replay TTL zero | delegated-token preparation | provisional `AUTH_TOKEN_PREPARE_REPLAY_TTL_ZERO` | command label | 1 — caller-derivable | exact endpoint/command kind | none | public contract | delete label parameter from diagnostic construction |
| `DPC-217` | delegated-token replay TTL bound | same route | provisional `AUTH_TOKEN_PREPARE_REPLAY_TTL_EXCEEDED` | command label | 1 — caller-derivable | exact endpoint/command kind | none | public contract | delete label parameter from diagnostic construction |
| `DPC-218` | same bound | same route | same meaning | requested replay TTL | 1 — caller-derivable | exact request metadata | none | public to the affected caller | discard value; request retains it |
| `DPC-219` | same bound | same route | same meaning | maximum replay TTL | 1 — caller-derivable | maintained replay contract | none | public contract | discard value |
| `DPC-220` | unexpected delegated-token recovery reason | delegated-token preparation | provisional `AUTH_PREPARE_REPLAY_RECOVERY_REASON_INVALID` | typed `RecoveryReason` | 3 — authoritatively typed | retained replay receipt | operation-correlated replay status | guarded to exact operation owner | keep typed reason in receipt/status; return exact compact code without debug text |
| `DPC-221` | delegated-token actor quota | same route | provisional `REPLAY_PENDING_ACTOR_CAPACITY` | maximum pending count | 1 — caller-derivable | maintained replay quota | none | public contract | discard value |
| `DPC-222` | delegated-token command quota | same route | provisional `REPLAY_PENDING_COMMAND_CAPACITY` | maximum pending count | 1 — caller-derivable | maintained replay quota | none | public contract | discard value |
| `DPC-223` | unexpected role-attestation recovery reason | role-attestation preparation | provisional `AUTH_PREPARE_REPLAY_RECOVERY_REASON_INVALID` | typed `RecoveryReason` | 3 — authoritatively typed | retained replay receipt | operation-correlated replay status | guarded to exact operation owner | keep typed reason in receipt/status; return exact compact code without debug text |
| `DPC-224` | role-attestation actor quota | same route | provisional `REPLAY_PENDING_ACTOR_CAPACITY` | maximum pending count | 1 — caller-derivable | maintained replay quota | none | public contract | discard value |
| `DPC-225` | role-attestation command quota | same route | provisional `REPLAY_PENDING_COMMAND_CAPACITY` | maximum pending count | 1 — caller-derivable | maintained replay quota | none | public contract | discard value |
| `DPC-226` | receipt missing adapter | both authentication prepare routes | provisional `REPLAY_RECEIPT_MISSING` | command label | 1 — caller-derivable | exact receipt command kind | none | visible to operation owner | discard label; receipt identity retains command kind |
| `DPC-227` | receipt decode adapter | same routes | provisional `REPLAY_RECEIPT_DECODE_FAILED` | command label | 1 — caller-derivable | exact receipt command kind | none | visible to operation owner | discard label |
| `DPC-228` | same adapter | same routes | same meaning | typed receipt decode cause currently flattened to `String` | 3 — authoritatively typed | replay receipt decoder | operation-correlated replay status | may expose malformed retained bytes/details | replace string field with a finite typed cause and keep it guarded |
| `DPC-229` | receipt-token mismatch adapter | same routes | provisional `REPLAY_RECEIPT_TOKEN_MISMATCH` | command label | 1 — caller-derivable | exact receipt command kind | none | visible to operation owner | discard label |
| `DPC-230` | staged-response missing adapter | same routes | provisional `REPLAY_STAGED_RESPONSE_MISSING` | command label | 1 — caller-derivable | exact receipt command kind | none | visible to operation owner | discard label |
| `DPC-231` | cost-settlement identity missing adapter | same routes | provisional `REPLAY_COST_GUARD_SETTLEMENT_MISSING` | command label | 1 — caller-derivable | exact receipt command kind | none | visible to operation owner | discard label |
| `DPC-232` | delegated-token response encode | delegated-token preparation | provisional `REPLAY_RESPONSE_ENCODE_FAILED` | typed Candid encoder cause currently flattened to `String` | 3 — authoritatively typed | exact response encoder | operation-correlated replay status | guarded implementation detail | retain finite typed cause; discard text |
| `DPC-233` | delegated-token response decode | same route | provisional `REPLAY_RESPONSE_DECODE_FAILED` | typed replay decoder cause currently flattened to `String` | 3 — authoritatively typed | exact response decoder | operation-correlated replay status | guarded retained-byte detail | retain finite typed cause; discard text |
| `DPC-234` | role-attestation response encode | role-attestation preparation | provisional `REPLAY_RESPONSE_ENCODE_FAILED` | typed Candid encoder cause currently flattened to `String` | 3 — authoritatively typed | exact response encoder | operation-correlated replay status | guarded implementation detail | retain finite typed cause; discard text |
| `DPC-235` | role-attestation response decode | same route | provisional `REPLAY_RESPONSE_DECODE_FAILED` | typed replay decoder cause currently flattened to `String` | 3 — authoritatively typed | exact response decoder | operation-correlated replay status | guarded retained-byte detail | retain finite typed cause; discard text |

The slice adds sixteen caller-derivable and seven authoritatively typed values.
It adds no sensitive-only or caller-required-unowned value. Recovery and codec
causes remain bound to the exact replay operation; they never become generic
public detail.

Across all twenty-six classified slices, the dynamic ledger now contains 235
values: 145 caller-derivable, sixteen sensitive operator-only, 49
authoritatively typed and 25 caller-required but unowned. The 25 unowned values
and their proposed narrow retrieval owners remain unchanged.

## Classified Slice 27: Runtime Auth Prepare And Issuer Provisioning

Prepare orchestration formats retained-response quotas and secondary response-
commit recovery failure. Issuer provisioning formats the exact issuer together
with typed IC call causes for interactive requests, then formats the same typed
causes again for timer renewal.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-236` | retained delegated-token responses exceed actor quota | delegated-token preparation | provisional `REPLAY_RETAINED_ACTOR_CAPACITY` | maximum retained count | 1 — caller-derivable | maintained replay-retention contract | none | public contract | discard value |
| `DPC-237` | retained responses exceed command quota | same route | provisional `REPLAY_RETAINED_COMMAND_CAPACITY` | maximum retained count | 1 — caller-derivable | maintained replay-retention contract | none | public contract | discard value |
| `DPC-238` | response recovery-marker failure | both authentication prepare routes | exact primary response/staging diagnostic | prepare command label | 1 — caller-derivable | exact replay command kind | none | visible to operation owner | discard label; command kind remains in the receipt |
| `DPC-239` | same secondary failure | same routes | exact typed recovery-marker diagnostic | nested `ReplayReceiptStoreError` currently formatted through `InternalError` | 3 — authoritatively typed | replay receipt store | operation-correlated replay status | guarded state/cause detail | retain the typed secondary code against the same operation; never append prose to the primary diagnostic |
| `DPC-240` | issuer install loop completes without success or failure | root provisioning | provisional `AUTH_ISSUER_PROOF_INSTALLATION_INCOMPLETE` | issuer principal | 1 — caller-derivable | exact provisioning request and batch | none | visible to root/operator | discard principal; request retains it |
| `DPC-241` | interactive issuer request encoding failure | same route | existing `IC_CALL_REQUEST_ENCODING_FAILED` | issuer principal | 1 — caller-derivable | exact provisioning request | none | visible to root/operator | discard principal |
| `DPC-242` | same failure | same route | same nested IC diagnostic | typed request-encoding `InternalError` | 3 — authoritatively typed | IC call request adapter | transparent registered-code propagation | dependency details follow source projection | propagate nested code; delete `to_string()` context |
| `DPC-243` | interactive issuer transport failure | same route | exact nested IC transport diagnostic | issuer principal | 1 — caller-derivable | exact provisioning request | none | visible to root/operator | discard principal |
| `DPC-244` | same failure | same route | same nested IC diagnostic | typed transport `InternalError` | 3 — authoritatively typed | IC call effect adapter | transparent registered-code propagation | target/reject details follow source projection | propagate nested code; delete `to_string()` context |
| `DPC-245` | interactive issuer response decode failure | same route | existing `IC_CALL_RESPONSE_DECODING_FAILED` | issuer principal | 1 — caller-derivable | exact provisioning request | none | visible to root/operator | discard principal |
| `DPC-246` | same failure | same route | same nested IC diagnostic | typed response-decoding `InternalError` | 3 — authoritatively typed | IC call response adapter | transparent registered-code propagation | decoder details follow source projection | propagate nested code; delete `to_string()` context |
| `DPC-247` | renewal issuer request encoding failure | guarded auth-renewal status | existing `IC_CALL_REQUEST_ENCODING_FAILED` | typed request-encoding `InternalError` | 3 — authoritatively typed | IC call request adapter | operation-correlated renewal observation | guarded dependency detail | record nested exact code; delete formatted cause |
| `DPC-248` | renewal issuer transport failure | same route | exact nested IC transport diagnostic | typed transport `InternalError` | 3 — authoritatively typed | IC call effect adapter | operation-correlated renewal observation | guarded target/reject detail | record nested exact code; delete formatted cause |
| `DPC-249` | renewal issuer response decode failure | same route | existing `IC_CALL_RESPONSE_DECODING_FAILED` | typed response-decoding `InternalError` | 3 — authoritatively typed | IC call response adapter | operation-correlated renewal observation | guarded decoder detail | record nested exact code; delete formatted cause |

The slice adds seven caller-derivable and seven authoritatively typed values.
It adds no sensitive-only or caller-required-unowned value. The shared replay-
abort formatter remains outside this slice and is still open.

Across all twenty-seven classified slices, the dynamic ledger now contains 249
values: 152 caller-derivable, sixteen sensitive operator-only, 56
authoritatively typed and 25 caller-required but unowned. The 25 unowned values
and their proposed narrow retrieval owners remain unchanged.

## Classified Slice 28: Runtime Auth Root Issuer And Batch Policy

The root-issuer facade and delegation-batch sweep format already-typed
`AuthPolicyError` values. Those values may contain issuer principals, roles,
scopes, TTLs or policy bounds, but the typed family already owns their exact
meaning and approved projection.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-250` | issuer-policy upsert mapper | root issuer policy mutation | exact nested auth-policy diagnostic | typed `AuthPolicyError` flattened with `to_string()` | 3 — authoritatively typed | pure issuer-policy validator | transparent registered-code propagation | fields follow source projection | replace broad InvalidInput wrapper with exhaustive typed dispatch |
| `DPC-251` | renewal-template Fleet/grant mapper | renewal-template mutation | exact nested Fleet-binding or grant-required diagnostic | typed `AuthPolicyError` flattened with `to_string()` | 3 — authoritatively typed | pure renewal-template validator | transparent registered-code propagation | fields follow source projection | preserve exact source identity and class |
| `DPC-252` | renewal-template catch-all mapper | same route | exact nested issuer-policy diagnostic | typed `AuthPolicyError` flattened with `to_string()` | 3 — authoritatively typed | pure issuer-policy validator | transparent registered-code propagation | fields follow source projection | delete broad Forbidden catch-all and map exhaustively |
| `DPC-253` | root delegation-batch approval mapper | guarded renewal/batch status | exact nested issuer-policy diagnostic | typed `AuthPolicyError` flattened with `to_string()` | 3 — authoritatively typed | pure issuer-policy validator | operation-correlated batch observation | protected issuer/policy fields follow source projection | retain exact typed code against the batch; delete formatted cause |

The slice adds four authoritatively typed values and no caller-derivable,
sensitive-only or caller-required-unowned value.

Across all twenty-eight classified slices, the dynamic ledger now contains 253
values: 152 caller-derivable, sixteen sensitive operator-only, 60
authoritatively typed and 25 caller-required but unowned. The 25 unowned values
and their proposed narrow retrieval owners remain unchanged.

## Classified Slice 29: Root And Non-Root Runtime Lifecycle

Lifecycle orchestration formats typed memory, environment, configuration and
nested startup causes. These wrappers describe where an existing failure was
observed, not a new diagnostic authority.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-254` | root memory bootstrap | root initialization | exact nested memory diagnostic | typed memory-bootstrap error formatted as prose | 3 — authoritatively typed | `ic-memory` adapter | lifecycle numeric log | protected memory detail | propagate exact registered adapter code |
| `DPC-255` | root environment policy | same route | existing `ENV_REQUIRED_FIELDS_MISSING` | missing-field list | 3 — authoritatively typed | `EnvPolicyError::MissingEnvFields` | lifecycle numeric log | protected environment field set | discard field list; exact code owns reinstall action |
| `DPC-256` | root environment import | same route | exact nested environment diagnostic | typed environment-import error formatted as prose | 3 — authoritatively typed | environment ops | lifecycle numeric log | protected binding detail follows source projection | propagate exact source code |
| `DPC-257` | root application init-mode lookup | same route | exact nested configuration diagnostic | typed configuration error formatted as prose | 3 — authoritatively typed | configuration ops | lifecycle numeric log | configuration detail follows source projection | propagate exact source code |
| `DPC-258` | Active root service startup | root post-upgrade | exact nested runtime-service diagnostic | typed `InternalError` formatted as prose | 3 — authoritatively typed | failing runtime service workflow | lifecycle/guarded runtime observation | source sensitivity applies | propagate exact source code; delete root-startup wrapper |
| `DPC-259` | sibling Wasm Store environment initialization | Store initialization | exact nested environment diagnostic | typed environment-workflow error formatted as prose | 3 — authoritatively typed | environment workflow | lifecycle numeric log | protected Store/root binding detail | propagate exact source code |
| `DPC-260` | standalone-local environment initialization | local non-root initialization | exact nested environment diagnostic | typed environment-workflow error formatted as prose | 3 — authoritatively typed | environment workflow | lifecycle numeric log | local environment detail follows source projection | propagate exact source code |
| `DPC-261` | non-root memory bootstrap | all non-root initialization | exact nested memory diagnostic | typed memory-bootstrap error formatted as prose | 3 — authoritatively typed | `ic-memory` adapter | lifecycle numeric log | protected memory detail | propagate exact registered adapter code |
| `DPC-262` | non-root application init-mode lookup | managed/local non-root initialization | exact nested configuration diagnostic | typed configuration error formatted as prose | 3 — authoritatively typed | configuration ops | lifecycle numeric log | configuration detail follows source projection | propagate exact source code |
| `DPC-263` | post-upgrade current-role configuration lookup | non-root post-upgrade | exact nested configuration diagnostic | typed configuration error formatted as prose | 3 — authoritatively typed | configuration ops | lifecycle numeric log | protected role/config detail | propagate exact source code |

The slice adds ten authoritatively typed values and no caller-derivable,
sensitive-only or caller-required-unowned value.

Across all twenty-nine classified slices, the dynamic ledger now contains 263
values: 152 caller-derivable, sixteen sensitive operator-only, 70
authoritatively typed and 25 caller-required but unowned. The 25 unowned values
and their proposed narrow retrieval owners remain unchanged.

## Classified Slice 30: Runtime Coordination, Restore And Activation

Runtime coordination formats two typed adapter causes and one exact refill
count. Restore supplies one closed timer-state reason. Fleet activation appends
operation, target and typed observation failures while reconciling uncertain
child calls.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-264` | root-service access wrapper | root lifecycle startup | existing `ACCESS_ROOT_REQUIRED` | typed environment/access error formatted as prose | 3 — authoritatively typed | environment ops | transparent registered-code propagation | source projection applies | propagate exact access code; delete wrapper text |
| `DPC-265` | post-upgrade memory bootstrap | root/non-root lifecycle | exact nested memory diagnostic | typed memory-bootstrap error formatted as prose | 3 — authoritatively typed | `ic-memory` adapter | lifecycle numeric log | protected memory detail | propagate exact adapter code |
| `DPC-266` | root upgrade refill fence | root upgrade admission | provisional `ICP_REFILL_UPGRADE_RESUMABLE` | resumable operation count | 1 — caller-derivable | refill status/index counters | none | visible to root/operator | discard count; bounded refill status retains it |
| `DPC-267` | authority-restore timer fence | snapshot prepare/resume | provisional `AUTHORITY_RESTORE_TIMER_RUNNING` | closed static timer-state reason | 1 — caller-derivable | timer workflow state | none | public contract | replace string reason with typed decision; exact code owns action |
| `DPC-268` | uncertain child credential-generation observation fails | root child-activation orchestration | exact primary IC/RPC call diagnostic | operation label | 1 — caller-derivable | exact protocol method | none | public contract | discard label |
| `DPC-269` | same failure | same route | same primary diagnostic | target Canister principal | 1 — caller-derivable | exact activation manifest/request | none | visible to root/operator | discard principal; operation retains target |
| `DPC-270` | same failure | same route | exact nested observation diagnostic | typed status-call `InternalError` | 3 — authoritatively typed | RPC/IC call adapter | operation-correlated activation observation | target/reject details follow source projection | retain secondary typed code against the same activation operation |
| `DPC-271` | observed child credential/status fails validation | same route | exact primary IC/RPC call diagnostic | operation label | 1 — caller-derivable | exact protocol method | none | public contract | discard label |
| `DPC-272` | same failure | same route | same primary diagnostic | target Canister principal | 1 — caller-derivable | exact activation manifest/request | none | visible to root/operator | discard principal; operation retains target |
| `DPC-273` | same failure | same route | exact child-status validation diagnostic | typed validation `InternalError` | 3 — authoritatively typed | activation-status validator | operation-correlated activation observation | protected status mismatch follows source projection | retain secondary typed code; never append prose to primary diagnostic |

The slice adds six caller-derivable and four authoritatively typed values. It
adds no sensitive-only or caller-required-unowned value. Post-transition
runtime-startup traps remain lifecycle log evidence, not public `Error.message`
values.

Across all thirty classified slices, the dynamic ledger now contains 273
values: 158 caller-derivable, sixteen sensitive operator-only, 74
authoritatively typed and 25 caller-required but unowned. The 25 unowned values
and their proposed narrow retrieval owners remain unchanged.

## Classified Slice 31: Environment, Component RPC And Service Binding

Environment initialization formats the missing-field decision and protected
Component-topology cause. Fleet-service compilation then converts five typed
configuration, plan, receipt and binding results through the generic ops text
boundary. Component-RPC lifecycle contributes no interpolation: its current
messages are static and its exact identities remain in typed requests,
bindings and removal status.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-274` | environment bootstrap required-field failure | root, Store and managed-runtime initialization | existing `ENV_REQUIRED_FIELDS_MISSING` | exact missing-field set | 3 — authoritatively typed | `EnvPolicyError::MissingEnvFields` and protected init arguments | lifecycle numeric log | protected environment fields | discard field list; preserve the exact environment code |
| `DPC-275` | managed Component/child binding validation adapter | managed-runtime initialization | exact nested Component-topology diagnostic | reachable typed `ComponentTopologyError` flattened to prose | 3 — authoritatively typed | compiled Component Topology and root-issued binding | transparent registered-code propagation | protected Fleet/root/Subnet/Spec/role/principal detail follows source projection | remove formatter and propagate the exact topology code |
| `DPC-276` | initial-service configuration adapter | Coordinator Fleet-service compilation | exact compiled configuration diagnostic | typed configuration cause flattened through `Configuration(String)` and generic ops text | 3 — authoritatively typed | immutable deployment configuration and its digest | operation-correlated provisioning status | protected topology and service configuration | replace both string wrappers with the exact typed cause |
| `DPC-277` | complete initial-service compilation adapter | same route | exact nested Fleet-service binding diagnostic | typed binding/configuration/plan/receipt value flattened by generic ops conversion | 3 — authoritatively typed | immutable provisioning plan, canonical configuration and terminal root receipts | operation-correlated provisioning/publication status | protected member and receipt authority follows source projection | propagate the exact qualified code; retain values in their typed owners |
| `DPC-278` | complete Scale Out compilation adapter | Fleet-service Scale Out | exact nested Fleet-service binding diagnostic | typed binding/configuration/plan/receipt value flattened by generic ops conversion | 3 — authoritatively typed | immutable Scale Out plan, canonical configuration and terminal root receipts | operation-correlated Scale Out status | protected member and receipt authority follows source projection | propagate the exact qualified code; retain values in their typed owners |
| `DPC-279` | planned-root receipt-index adapter | Fleet-service root receipt validation | existing `FLEET_SERVICE_BINDING_ROOT_RECEIPT_INDEX_INVALID` | root index and planned root count | 3 — authoritatively typed | immutable provisioning plan and validation request | operation-correlated provisioning status | guarded operator evidence | discard interpolation; preserve both values in plan/status evidence |
| `DPC-280` | terminal root-receipt validation adapter | same route | exact nested Fleet-service binding diagnostic | typed receipt identity, state, count, time, hash or result value flattened by generic ops conversion | 3 — authoritatively typed | immutable root batch and terminal root receipt | operation-correlated provisioning status | protected receipt authority follows source projection | propagate exact qualified code; retain evidence in the receipt/status |

The slice adds seven authoritatively typed values and no caller-derivable,
sensitive-only or caller-required-unowned value. Fleet-service values do not
move into a generic detail field: the plan, configuration, receipt and status
remain their sole authorities.

Across all thirty-one classified slices, the dynamic ledger now contains 280
values: 158 caller-derivable, sixteen sensitive operator-only, 81
authoritatively typed and 25 caller-required but unowned. The 25 unowned values
and their proposed narrow retrieval owners remain unchanged.

## Classified Slice 32: Cascade, ICP Refill And Intent Storage

Topology cascade formats protected route principals and one typed activation-
storage cause. ICP-refill policy currently debug-formats finite typed
violations. Five intent index readers then flatten typed storage causes at the
generic storage boundary.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-281` | prepared topology activation-evidence adapter | topology cascade | exact nested activation-storage diagnostic | typed storage cause flattened by generic storage conversion | 3 — authoritatively typed | Fleet-activation snapshot store | transparent registered-code propagation | protected activation evidence follows source projection | propagate exact storage code |
| `DPC-282` | child cascade send context | same route | exact nested transport/remote diagnostic | next-child principal | 3 — authoritatively typed | validated topology snapshot and exact transport target | guarded cascade observation | protected topology principal | discard from public message; retain only in typed snapshot/log evidence |
| `DPC-283` | receiver-first path mismatch | same route | existing `TOPOLOGY_RECEIVER_MISMATCH` | receiving Canister principal | 1 — caller-derivable | transport destination and maintained snapshot contract | none | visible to authenticated parent/root | discard principal; exact code owns route repair |
| `DPC-284` | branch slicing cannot locate successor | same route | existing `TOPOLOGY_NEXT_HOP_MISSING` | requested next-hop principal | 3 — authoritatively typed | validated topology snapshot | guarded cascade observation | protected topology principal | discard from public message; inspect typed snapshot/status |
| `DPC-285` | maximum refill amount violation | manual ICP refill | existing `ICP_REFILL_AMOUNT_EXCEEDS_LIMIT` | requested ICP e8s | 1 — caller-derivable | exact request | none | public to authenticated requester | discard requested value |
| `DPC-286` | same violation | same route | same meaning | configured maximum ICP e8s | 3 — authoritatively typed | checked-in Component refill policy | request-scoped policy-preflight result | guarded funding configuration | expose only through approved policy/config status; delete debug text |
| `DPC-287` | trusted rate unavailable | same route | existing `ICP_REFILL_RATE_UNAVAILABLE` | configured minimum XDR-permyriad rate | 3 — authoritatively typed | checked-in Component refill policy | request-scoped policy-preflight result | guarded funding configuration | retain in typed preflight/config owner, not error prose |
| `DPC-288` | trusted rate below gate | same route | existing `ICP_REFILL_RATE_GATE_DENIED` | observed XDR-permyriad rate | 4 — caller-required but unowned | none after the rejected preflight returns | exact ICP-refill policy-preflight result for the operation/request | guarded funding/rate evidence | add narrow typed owner before removing the value; never use generic detail |
| `DPC-289` | same violation | same route | same meaning | configured minimum XDR-permyriad rate | 3 — authoritatively typed | checked-in Component refill policy | request-scoped policy-preflight result | guarded funding configuration | retain in typed preflight/config owner, not error prose |
| `DPC-290` | cleanup-deadline adapter | local intent cleanup | exact nested intent-storage diagnostic | typed intent/index ID or deadline value flattened by storage conversion | 3 — authoritatively typed | primary intent and finite-expiry index | operation-correlated intent status | guarded intent authority | propagate exact code; retain value in typed record/status |
| `DPC-291` | bounded due-expiry page adapter | timer cleanup | exact nested intent-storage diagnostic | typed expiry-index key/value contradiction flattened by storage conversion | 3 — authoritatively typed | finite-expiry index and primary intent | guarded cleanup observation | guarded intent authority | propagate exact code; retain typed index evidence |
| `DPC-292` | earliest due-expiry adapter | timer scheduling | exact nested intent-storage diagnostic | typed expiry-index key/value contradiction flattened by storage conversion | 3 — authoritatively typed | finite-expiry index and primary intent | guarded cleanup observation | guarded intent authority | propagate exact code; retain typed index evidence |
| `DPC-293` | placement-acknowledgement presence adapter | placement acknowledgement driver | exact nested intent-storage diagnostic | typed operation/index/primary-record contradiction flattened by storage conversion | 3 — authoritatively typed | placement acknowledgement index and receipt-backed intent | operation-correlated placement status | guarded placement authority | propagate exact code; retain typed status evidence |
| `DPC-294` | placement-acknowledgement page adapter | same route | exact nested intent-storage diagnostic | typed operation/index/primary-record contradiction flattened by storage conversion | 3 — authoritatively typed | placement acknowledgement index and receipt-backed intent | operation-correlated placement status | guarded placement authority | propagate exact code; retain typed status evidence |

The slice adds two caller-derivable, eleven authoritatively typed and one
caller-required-but-unowned value. It adds no sensitive-only value. The
observed rate must gain its request-scoped preflight owner before B2; a compact
gate-denied code alone does not preserve the operator's current observation.

Across all thirty-two classified slices, the dynamic ledger now contains 294
values: 160 caller-derivable, sixteen sensitive operator-only, 92
authoritatively typed and 26 caller-required but unowned.

## Classified Slice 33: Authority Restore And Placement Allocation

Authority-restore fence messages are static; their complete operation,
Canister, history and timestamp evidence already lives in the typed status.
Receipt-backed placement allocation formats operation IDs, settlement state,
revisions and bounded capacity values at fifteen result branches.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-295` | recovery has no durable intent | placement child recovery | provisional `PLACEMENT_ALLOCATION_RECOVERY_INTENT_MISSING` | operation ID | 1 — caller-derivable | exact recovery request | none | visible to authenticated caller | discard ID; request retains it |
| `DPC-296` | settlement reaches unexpected terminal state | placement settlement | provisional `PLACEMENT_ALLOCATION_SETTLEMENT_STATE_MISMATCH` | operation ID | 1 — caller-derivable | exact allocation permit/request | none | visible to operation owner | discard ID |
| `DPC-297` | same branch | same route | same meaning | retained terminal intent state | 3 — authoritatively typed | receipt-backed intent status | operation-correlated placement status | guarded operation state | retain typed state; compact error carries only exact identity |
| `DPC-298` | settlement intent disappeared | same route | provisional `PLACEMENT_ALLOCATION_SETTLEMENT_INTENT_MISSING` | operation ID | 1 — caller-derivable | exact allocation permit/request | none | visible to operation owner | discard ID |
| `DPC-299` | settlement revision conflict | same route | provisional `PLACEMENT_ALLOCATION_SETTLEMENT_REVISION_CONFLICT` | operation ID | 1 — caller-derivable | exact allocation permit/request | none | visible to operation owner | discard ID |
| `DPC-300` | same branch | same route | same meaning | expected permit revision | 3 — authoritatively typed | durable allocation permit | operation-correlated placement status | guarded operation state | retain in permit/status |
| `DPC-301` | same branch | same route | same meaning | actual intent revision | 3 — authoritatively typed | receipt-backed intent status | operation-correlated placement status | guarded operation state | retain in status |
| `DPC-302` | settlement payload-binding conflict | same route | provisional `PLACEMENT_ALLOCATION_SETTLEMENT_BINDING_CONFLICT` | operation ID | 1 — caller-derivable | exact allocation permit/request | none | visible to operation owner | discard ID |
| `DPC-303` | pre-cleanup payload-binding conflict | terminal receipt cleanup | provisional `PLACEMENT_ALLOCATION_CLEANUP_BINDING_CONFLICT` | operation ID | 1 — caller-derivable | exact allocation permit/request | none | visible to operation owner | discard ID |
| `DPC-304` | cleanup finds pending intent | same route | provisional `PLACEMENT_ALLOCATION_CLEANUP_NOT_TERMINAL` | operation ID | 1 — caller-derivable | exact allocation permit/request | none | visible to operation owner | discard ID |
| `DPC-305` | cleanup revision conflict | same route | provisional `PLACEMENT_ALLOCATION_CLEANUP_REVISION_CONFLICT` | operation ID | 1 — caller-derivable | exact allocation permit/request | none | visible to operation owner | discard ID |
| `DPC-306` | same branch | same route | same meaning | expected terminal revision | 3 — authoritatively typed | retained terminal intent supplied to cleanup | operation-correlated placement status | guarded operation state | retain in status |
| `DPC-307` | same branch | same route | same meaning | actual intent revision | 3 — authoritatively typed | receipt-backed intent status | operation-correlated placement status | guarded operation state | retain in status |
| `DPC-308` | cleanup result payload-binding conflict | same route | provisional `PLACEMENT_ALLOCATION_CLEANUP_BINDING_CONFLICT` | operation ID | 1 — caller-derivable | exact allocation permit/request | none | visible to operation owner | discard ID |
| `DPC-309` | committed intent lacks domain membership | placement admission/recovery | provisional `PLACEMENT_ALLOCATION_DOMAIN_MEMBERSHIP_MISSING` | operation ID | 1 — caller-derivable | exact allocation request | none | visible to operation owner | discard ID |
| `DPC-310` | operation already rolled back | same route | provisional `PLACEMENT_ALLOCATION_ROLLED_BACK` | operation ID | 1 — caller-derivable | exact allocation request | none | visible to operation owner | discard ID |
| `DPC-311` | begin input conflicts with retained binding | same route | provisional `PLACEMENT_ALLOCATION_BEGIN_BINDING_CONFLICT` | operation ID | 1 — caller-derivable | exact allocation request | none | visible to operation owner | discard ID |
| `DPC-312` | placement resource capacity exceeded | same route | provisional `PLACEMENT_ALLOCATION_CAPACITY_EXCEEDED` | current reserved-plus-committed quantity | 3 — authoritatively typed | bounded intent resource totals/status | operation-correlated placement status | guarded capacity state | retain in status/counters |
| `DPC-313` | same branch | same route | same meaning | requested quantity | 1 — caller-derivable | exact allocation request and maintained quantity-one contract | none | public contract | discard value |
| `DPC-314` | same branch | same route | same meaning | reservation limit | 3 — authoritatively typed | allocation input derived from placement capacity | operation-correlated placement status | guarded capacity policy | retain in request/status authority |
| `DPC-315` | receipt-backed intent capacity reached | same route | provisional `PLACEMENT_ALLOCATION_INTENT_CAPACITY_REACHED` | current record count | 3 — authoritatively typed | bounded receipt-capacity view/counter | guarded receipt-capacity status | guarded capacity state | retain in bounded status |
| `DPC-316` | same branch | same route | same meaning | receipt record limit | 3 — authoritatively typed | maintained receipt-backed intent bound | guarded receipt-capacity status | guarded capacity contract | retain in bounded status/contract |

The slice adds thirteen caller-derivable and nine authoritatively typed values.
It adds no sensitive-only or caller-required-unowned value. Placement status
and capacity views remain the typed evidence owners; compact errors do not
reconstruct settlement authority.

Across all thirty-three classified slices, the dynamic ledger now contains
316 values: 173 caller-derivable, sixteen sensitive operator-only, 101
authoritatively typed and 26 caller-required but unowned.

## Classified Slice 34: ICP-Refill Replay And Effect Adapters

ICP-refill replay formats one recovery reason, two quota ceilings, three codec
or receipt-store causes and five secondary recovery/cost-settlement failures.
The ICP Ledger/CMC ops facade then flattens one typed infrastructure family.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-317` | unsupported refill recovery branch | ICP refill replay | provisional `ICP_REFILL_REPLAY_RECOVERY_REASON_INVALID` | typed `RecoveryReason` | 3 — authoritatively typed | retained replay receipt | operation-correlated refill replay status | guarded operation state | retain typed reason in receipt/status; remove debug text |
| `DPC-318` | pending actor quota exceeded | same route | existing `REPLAY_PENDING_ACTOR_CAPACITY` | maximum pending count | 1 — caller-derivable | maintained replay quota | none | public contract | discard value |
| `DPC-319` | pending refill-command quota exceeded | same route | existing `REPLAY_PENDING_COMMAND_CAPACITY` | maximum pending count | 1 — caller-derivable | maintained replay quota | none | public contract | discard value |
| `DPC-320` | refill response encode adapter | terminal replay commit | existing `REPLAY_RESPONSE_ENCODE_FAILED` | typed response-encoder cause flattened to `String` | 3 — authoritatively typed | refill response encoder and schema | operation-correlated replay status | guarded implementation detail | preserve finite typed cause; discard text |
| `DPC-321` | refill response decode adapter | replay/recovery response | existing `REPLAY_RESPONSE_DECODE_FAILED` | typed replay decoder cause flattened to `String` | 3 — authoritatively typed | terminal replay receipt and refill response decoder | operation-correlated replay status | guarded retained-byte detail | preserve finite typed cause; discard text |
| `DPC-322` | replay receipt decode adapter | every refill replay transition | existing `REPLAY_RECEIPT_DECODE_FAILED` | typed receipt decode cause flattened to `String` | 3 — authoritatively typed | replay receipt decoder | operation-correlated replay status | guarded retained-byte detail | preserve finite typed cause; discard text |
| `DPC-323` | cost-settlement failure cannot mark recovery | replay finalization | exact primary settlement diagnostic plus exact secondary marker diagnostic | typed recovery-marker failure appended to primary text | 3 — authoritatively typed | replay receipt store and exact operation | operation-correlated secondary numeric observation | guarded recovery state | return primary code unchanged; record secondary code against operation |
| `DPC-324` | response-commit failure cannot mark recovery | same route | exact primary commit diagnostic plus exact secondary marker diagnostic | typed recovery-marker failure appended to primary text | 3 — authoritatively typed | replay receipt store and exact operation | operation-correlated secondary numeric observation | guarded recovery state | return primary code unchanged; record secondary code against operation |
| `DPC-325` | response preparation also fails cost settlement | same route | exact primary response diagnostic plus exact secondary settlement diagnostic | typed cost-settlement failure appended to primary text | 3 — authoritatively typed | cost-guard settlement and replay operation | operation-correlated secondary numeric observation | guarded cost state | return primary code unchanged; record secondary code against operation |
| `DPC-326` | response failure cannot mark recovery | same route | exact primary response diagnostic plus exact secondary marker diagnostic | typed recovery-marker failure appended to primary text | 3 — authoritatively typed | replay receipt store and exact operation | operation-correlated secondary numeric observation | guarded recovery state | return primary code unchanged; record secondary code against operation |
| `DPC-327` | recovered cost settlement cannot advance response-commit marker | refill replay recovery | exact primary commit diagnostic plus exact secondary marker diagnostic | typed recovery-marker failure appended to primary text | 3 — authoritatively typed | replay receipt store and exact operation | operation-correlated secondary numeric observation | guarded recovery state | return primary code unchanged; record secondary code against operation |
| `DPC-328` | ICP Ledger/CMC ops adapter | ICP refill preflight and effects | exact nested IC-infrastructure diagnostic | typed `IcInfraError` flattened through `OpsError` | 3 — authoritatively typed | ICP Ledger/CMC infrastructure adapter | transparent registered-code propagation | target/reject/codec detail follows source projection | propagate exact source code; delete ops wrapper text |

The slice adds two caller-derivable and ten authoritatively typed values. It
adds no sensitive-only or caller-required-unowned value. Secondary failures
remain independently observable against the exact operation and never replace
or decorate the primary diagnostic.

Across all thirty-four classified slices, the dynamic ledger now contains 328
values: 175 caller-derivable, sixteen sensitive operator-only, 111
authoritatively typed and 26 caller-required but unowned.

## Classified Slice 35: Component Runtime Canonical Hashing

Component runtime validation uses two canonical Candid encoders before hashing
Directory authority and direct-child membership. Their formatter text is not
authority and must not become durable or public detail.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-329` | Component Directory authority encode adapter | runtime prepare/synchronize | provisional `COMPONENT_RUNTIME_DIRECTORY_AUTHORITY_ENCODE_FAILED` | typed Candid encoder cause flattened to `String` | 3 — authoritatively typed | canonical Component Directory authority DTO and encoder | operation-correlated Component runtime status | guarded implementation detail | preserve a finite typed codec cause; discard formatter text |
| `DPC-330` | direct-child membership encode adapter | same route | provisional `COMPONENT_RUNTIME_DIRECT_CHILDREN_ENCODE_FAILED` | typed Candid encoder cause flattened to `String` | 3 — authoritatively typed | canonical direct-child DTO and encoder | operation-correlated Component runtime status | guarded implementation detail | preserve a finite typed codec cause; discard formatter text |

The slice adds two authoritatively typed values. It adds no caller-derivable,
sensitive-only or caller-required-unowned value. Activation-service startup
failure traps after the durable transition and remains lifecycle log evidence,
not public `Error.message` context.

Across all thirty-five classified slices, the dynamic ledger now contains 330
values: 175 caller-derivable, sixteen sensitive operator-only, 113
authoritatively typed and 26 caller-required but unowned.

## Classified Slice 36: Placement Scaling Policy Reason

The scaling workflow publishes one preformatted `ScalingPlan.reason` when a
typed plan denies worker creation. Only `AtMaxWorkers` and `WithinBounds` can
reach that constructor; the admitted `BelowMinWorkers` branch cannot.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-331` | `AtMaxWorkers` reason | scaling worker creation | provisional `SCALING_MAX_WORKERS_REACHED` | requested pool | 1 — caller-derivable | exact scaling request | none | public to caller | discard pool prose |
| `DPC-332` | same reason | same route | same meaning | current worker count | 3 — authoritatively typed | scaling Registry count | guarded scaling status | guarded runtime state | retain typed count in status; discard prose |
| `DPC-333` | same reason | same route | same meaning | configured maximum worker count | 3 — authoritatively typed | checked-in scaling policy | guarded scaling configuration/status | guarded configuration | retain typed limit; discard prose |
| `DPC-334` | `WithinBounds` reason | scaling worker creation | provisional `SCALING_WITHIN_POLICY_BOUNDS` | requested pool | 1 — caller-derivable | exact scaling request | none | public to caller | discard pool prose |
| `DPC-335` | same reason | same route | same meaning | current worker count | 3 — authoritatively typed | scaling Registry count | guarded scaling status | guarded runtime state | retain typed count in status; discard prose |
| `DPC-336` | same reason | same route | same meaning | configured minimum worker count | 3 — authoritatively typed | checked-in scaling policy | guarded scaling configuration/status | guarded configuration | retain typed limit; discard prose |
| `DPC-337` | same reason | same route | same meaning | configured maximum worker count | 3 — authoritatively typed | checked-in scaling policy | guarded scaling configuration/status | guarded configuration | retain typed limit; discard prose |

The slice adds two caller-derivable and five authoritatively typed values. It
adds no sensitive-only or caller-required-unowned value. B4 selects the exact
registered identity from `ScalingPlanReason` and removes the free-form reason.

Across all thirty-six classified slices, the dynamic ledger now contains 337
values: 177 caller-derivable, sixteen sensitive operator-only, 118
authoritatively typed and 26 caller-required but unowned.

## Classified Slice 37: Core Receipt, Configuration And RPC Adapters

Five small core ops owners format receipt labels, typed codec/configuration
causes and one request-variant label. Static Wasm Store target errors add no
dynamic value.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-338` | provisioning receipt encode adapter | root/Coordinator receipt hashing | existing `COMPONENT_PROVISIONING_RECEIPT_ENCODE_FAILED` | static caller-supplied receipt-kind label | 1 — caller-derivable | exact hasher method and frozen domain | none | guarded operation kind | discard label prose |
| `DPC-339` | same adapter | same route | same meaning | typed Candid encoder cause | 3 — authoritatively typed | canonical receipt DTO and encoder | operation-correlated provisioning status | guarded implementation detail | preserve finite typed codec cause; discard text |
| `DPC-340` | receipt byte-count adapter | same route | existing `COMPONENT_PROVISIONING_RECEIPT_BYTE_COUNT_EXCEEDED` | static receipt-kind label | 1 — caller-derivable | exact hasher method and frozen domain | none | guarded operation kind | discard label prose |
| `DPC-341` | protected deployment validation adapter | managed deployment validation | exact qualified deployment-context diagnostic | typed topology/deployment validation cause | 3 — authoritatively typed | compiled Component Topology and protected deployment | transparent registered-code propagation | protected deployment fields follow source projection | remove wrapper prefix and propagate exact source code |
| `DPC-342` | unsupported non-structural capability path | internal root RPC | provisional `RPC_NON_STRUCTURAL_CAPABILITY_UNSUPPORTED` | typed request-variant label | 1 — caller-derivable | exact internal request | none | visible to local operation owner | discard label; exact code plus request identifies the route |
| `DPC-343` | generic child extra-argument encode adapter | child creation RPC | provisional `RPC_CREATE_EXTRA_ARG_ENCODE_FAILED` | typed Candid encoder cause | 3 — authoritatively typed | admitted application extra DTO and encoder | operation-correlated child-creation status | guarded payload/codec detail | preserve finite typed cause; discard text |
| `DPC-344` | placement child extra-argument encode adapter | placement allocation RPC | provisional `RPC_PLACEMENT_EXTRA_ARG_ENCODE_FAILED` | typed Candid encoder cause | 3 — authoritatively typed | admitted placement extra DTO and encoder | operation-correlated placement status | guarded payload/codec detail | preserve finite typed cause; discard text |

The slice adds three caller-derivable and four authoritatively typed values. It
adds no sensitive-only or caller-required-unowned value. Method/request kind
remains in typed operation authority; dependency formatter prose is deleted.

Across all thirty-seven classified slices, the dynamic ledger now contains 344
values: 180 caller-derivable, sixteen sensitive operator-only, 122
authoritatively typed and 26 caller-required but unowned.

## Classified Slice 38: Cost-Guard Public Mapper

The cost-guard mapper currently flattens one typed reserve error into either a
broad invalid-input or resource-exhausted public message.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-345` | broad invalid-input cost mapper | every costed command | exact protected cost-configuration/accounting leaf | typed `CostGuardReserveError` flattened to `String` | 3 — authoritatively typed | cost manifest, quota/reservation intent and exact reserve variant | transparent exact-code mapping plus operation-correlated cost status | protected policy/accounting details follow source projection | delete broad kind and propagate exact identity |
| `DPC-346` | broad resource-exhausted cost mapper | same routes | exact quota-pressure or payer-reserve leaf | typed `CostGuardReserveError` flattened to `String` | 3 — authoritatively typed | quota/reservation intent and exact reserve variant | transparent exact-code mapping plus operation-correlated cost status | guarded quota/balance detail | delete broad kind and propagate exact identity |

The slice adds two authoritatively typed values and no caller-derivable,
sensitive-only or caller-required-unowned value. The existing typed variant,
not formatter prose, owns public classification and retry policy.

Across all thirty-eight classified slices, the dynamic ledger now contains 346
values: 180 caller-derivable, sixteen sensitive operator-only, 124
authoritatively typed and 26 caller-required but unowned.

## Classified Slice 39: Final Small-Adapter Context

The final direct-constructor slice contains four closed formatter
discriminators, four protected or typed values and four request-cycles
secondary failures. Static exact messages add no row.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-347` | `current_wasm_store` role rejection | Store configuration facade | provisional `WASM_STORE_RUNTIME_ROLE_INVALID` | protected current Canister role | 3 — authoritatively typed | initialized runtime environment | guarded runtime status | protected runtime role | remove role prose; exact code plus environment status owns repair |
| `DPC-348` | root-state cascade invalid-child helper | root Fleet-state cascade | one of five provisional root-state cascade identities | closed `RootChildAuthority` label | 4 — caller-required but unowned | internal two-variant enum only | exact registered diagnostic selected before formatting | guarded root inventory | delete label formatter; source authority selects the exact code |
| `DPC-349` | same helper as `DPC-348` | same route | same five-way family | one of three static invalid-child reasons | 4 — caller-required but unowned | static helper argument only | exact registered diagnostic selected at each predicate | guarded root inventory | delete reason formatter; predicate selects the exact code |
| `DPC-350` | Fleet canonical strict-order helper | Fleet-activation evidence hashing | one of four provisional canonical-order identities | one of four static authority labels | 4 — caller-required but unowned | static helper argument only | closed typed canonical-order context selecting an exact code | protected activation evidence | replace label with typed context; never publish label prose |
| `DPC-351` | root-draining reservation encode adapter | reservation content hashing | provisional `ROOT_DRAINING_RESERVATION_ENCODE_FAILED` | typed Candid encoder cause | 3 — authoritatively typed | canonical reservation DTO and encoder | operation-correlated root-draining status | guarded implementation detail | preserve finite typed cause; emit no codec text |
| `DPC-352` | Sharding bootstrap exhaustion helper | Sharding assignment/bootstrap | existing `SHARDING_POOL_AT_CAPACITY` or `SHARDING_NO_FREE_SLOTS` | selected pool | 3 — authoritatively typed | exact request or checked-in Sharding configuration | request/config status | visible only to the authorized caller/operator | remove pool prose; typed exhaustion reason selects the code |
| `DPC-353` | assignment call to the exhaustion helper | Sharding assignment | same two existing identities | requested partition key | 1 — caller-derivable | exact assignment request | none | public to requesting caller | discard key prose |
| `DPC-354` | initial-bootstrap call to the exhaustion helper | Sharding bootstrap | existing `SHARDING_NO_FREE_SLOTS` | static `__bootstrap__` sentinel | 4 — caller-required but unowned | free-form internal sentinel only | typed bootstrap exhaustion branch | internal bootstrap context | delete sentinel and select the exact no-slot code |
| `DPC-355` | terminal cycles response commit cannot mark recovery | root cycles capability | exact response-commit diagnostic plus exact secondary marker diagnostic | typed recovery-marker failure appended to primary text | 3 — authoritatively typed | replay receipt and exact cycles operation | operation-correlated secondary numeric observation | guarded recovery state | return primary code unchanged; record secondary code against the operation |
| `DPC-356` | cycles response staging also fails cost settlement | same route | exact primary response diagnostic plus exact secondary settlement diagnostic | typed cost-settlement failure appended to primary text | 3 — authoritatively typed | cost-guard settlement and replay operation | operation-correlated secondary numeric observation | guarded cost state | return primary code unchanged; record secondary code against the operation |
| `DPC-357` | cycles response failure cannot mark recovery | same route | exact primary response diagnostic plus exact secondary marker diagnostic | typed recovery-marker failure appended to primary text | 3 — authoritatively typed | replay receipt and exact cycles operation | operation-correlated secondary numeric observation | guarded recovery state | return primary code unchanged; record secondary code against the operation |
| `DPC-358` | cycles cost settlement cannot mark recovery | same route | exact primary settlement diagnostic plus exact secondary marker diagnostic | typed recovery-marker failure appended to primary text | 3 — authoritatively typed | replay receipt and exact cycles operation | operation-correlated secondary numeric observation | guarded recovery state | return primary code unchanged; record secondary code against the operation |

The slice adds one caller-derivable value, seven authoritatively typed values
and four caller-required-but-unowned closed discriminators. It adds no
sensitive-only value. The four closed discriminators need exact registered
codes or typed contexts, not new detail/status fields. Secondary failures use
the exact cycles operation's existing replay/cost authority and never decorate
the primary public diagnostic.

Across all thirty-nine classified slices, the dynamic ledger now contains 358
values: 181 caller-derivable, sixteen sensitive operator-only, 131
authoritatively typed and 30 caller-required but unowned.

## Classified Slice 40: Role Attestation And Root-Signature Proofs

This first transitive-auth slice follows role-attestation preparation,
retrieval and verification through the root Canister-signature helper. Static
missing/overflow/binding messages already have exact semantic identities and
add no dynamic row.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-359` | role-attestation canonical encode adapter | attestation prepare/verify | existing `AUTH_CANONICAL_ENCODING_FAILED` | static `role attestation` context label | 4 — caller-required but unowned | free-form codec context only | exact registered code selected by the role-attestation hasher | guarded implementation context | remove label; the owning hasher selects the code |
| `DPC-360` | same adapter as `DPC-359` | same route | same meaning | typed Candid encoder cause | 3 — authoritatively typed | canonical `RoleAttestation` DTO and encoder | guarded auth recent-failure observation | implementation detail | preserve the exact numeric code and discard codec text |
| `DPC-361` | attestation verifier-config wrapper | attestation verification | exact nested verifier-configuration diagnostic | typed `InternalError` flattened to text and rewrapped | 3 — authoritatively typed | protected verifier configuration | transparent registered-code propagation | protected configuration follows source projection | delete duplicate wrapper and propagate the exact source code |
| `DPC-362` | root-signature verification wrapper | same route | attestation proof invalid, proof unavailable or exact nested verifier cause | typed `InternalError` flattened to attestation-invalid text | 3 — authoritatively typed | root-signature verifier and exact submitted proof | exhaustive typed attestation mapping plus guarded auth observation | proof detail follows source projection | preserve unavailable/configuration causes; map only actual invalid proof to `AUTH_ATTESTATION_PROOF_INVALID` |
| `DPC-363` | attestation proof-size rejection | local attestation verification | existing `AUTH_ATTESTATION_FIELD_TOO_LARGE` | one of two static proof-field labels | 1 — caller-derivable | submitted proof shape | none | public to submitting caller | discard field label; caller retains the proof |
| `DPC-364` | same rejection as `DPC-363` | same route | same meaning | submitted field byte count | 1 — caller-derivable | submitted proof | none | public to submitting caller | discard count |
| `DPC-365` | same rejection as `DPC-363` | same route | same meaning | fixed field byte ceiling | 1 — caller-derivable | maintained public proof quota | none | public contract | discard numeric prose; documentation/protocol owns the ceiling |
| `DPC-366` | expired root-proof retrieval window | prepared proof retrieval | existing `AUTH_ROOT_PROOF_RETRIEVAL_EXPIRED` | preparation operation ID | 1 — caller-derivable | exact preparation request and pending proof record | operation-correlated auth preparation status | visible to operation owner | remove debug formatting; request/status retains the ID |
| `DPC-367` | Canister-signature map reports no signature | root-proof retrieval | exact invalid-proof identity | typed dependency `NoSignature` variant flattened to text | 3 — authoritatively typed | Canister-signature map result | guarded auth recent-failure observation | implementation detail | select the exact code from the typed variant and discard display text |
| `DPC-368` | Canister-signature public-key DER parser | root-proof verification | existing `AUTH_DELEGATION_PROOF_INVALID` or attestation projection at its caller | dependency-owned parser text | 2 — sensitive operator-only | no safe public detail owner; exact invalid-proof code and aggregate auth metric remain | guarded numeric auth observation only | potentially reveals malformed key structure | discard parser text before the public boundary |
| `DPC-369` | IC Canister-signature verifier | same route | same invalid-proof family as `DPC-368` | dependency-owned verifier text | 2 — sensitive operator-only | no safe public detail owner; exact invalid-proof code and aggregate auth metric remain | guarded numeric auth observation only | cryptographic verification detail | discard verifier text before the public boundary |

The slice adds four caller-derivable values, two sensitive operator-only
values, four authoritatively typed values and one caller-required-but-unowned
closed context label. The fixed root-proof retrieval TTL's saturating addition
does not add a diagnostic: near `u64::MAX` it shortens the remaining
representable window, and retrieval still expires when `now >= expiry`.

Across all forty classified slices, the dynamic ledger now contains 369
values: 185 caller-derivable, eighteen sensitive operator-only, 135
authoritatively typed and 31 caller-required but unowned.

## Classified Slice 41: Delegated Token And Chain-Key Verification

This slice follows delegated-token preparation/retrieval/verification and the
chain-key root-proof mapper. Static typed variants already own their semantic
identity. Rows below cover the typed values currently flattened by the broad
fallbacks.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-370` | delegated-token preparation fallback | token preparation | exact one of the qualified preparation/audience/canonical identities | typed `PrepareDelegatedTokenError` flattened to `Auth(String)` | 3 — authoritatively typed | pure preparation result and active proof | exhaustive registered-code mapping | follows exact source projection | delete fallback formatting; preserve nested cause identities |
| `DPC-371` | preparation token-TTL rejection | same route | existing `AUTH_TOKEN_TTL_EXCEEDED` | requested token TTL | 1 — caller-derivable | exact preparation request | none | public to requester | discard value; request retains it |
| `DPC-372` | same rejection as `DPC-371` | same route | same meaning | active certificate token-TTL ceiling | 3 — authoritatively typed | protected active delegation proof | guarded auth proof/status | protected certificate policy | remove value from public diagnostic; proof/status retains it |
| `DPC-373` | delegated-token verification fallback | token verification | exact one of the qualified token/certificate/audience/canonical identities | typed `VerifyDelegatedTokenError` flattened to broad invalid-input text | 3 — authoritatively typed | pure verification result | exhaustive registered-code mapping | follows exact source projection | remove broad fallback and propagate nested proof causes |
| `DPC-374` | verified token-TTL rejection | same route | existing `AUTH_TOKEN_TTL_EXCEEDED` | derived submitted token TTL | 1 — caller-derivable | submitted token claims | none | public to submitting caller | discard value |
| `DPC-375` | same rejection as `DPC-374` | same route | same meaning | submitted certificate token-TTL ceiling | 1 — caller-derivable | submitted token proof certificate | none | public to submitting caller | discard value |
| `DPC-376` | required scope absent from local grant | same route | existing `AUTH_SCOPE_REJECTED` | rejected required scope | 3 — authoritatively typed | exact application access requirement | access-policy definition/status | application policy | remove scope prose; the guard owns the requested scope |
| `DPC-377` | retained-token lookup wrapper | token retrieval | existing `AUTH_TOKEN_RETRIEVAL_MISSING` or `AUTH_TOKEN_RETRIEVAL_EXPIRED` | typed retained-token lookup error flattened to text | 3 — authoritatively typed | bounded retained-token store | exact typed mapping plus preparation status | operation state | preserve the missing/expired variant, not its display text |
| `DPC-378` | root-authority verification mismatch | embedded root-proof verification | existing `AUTH_ROOT_AUTHORITY_INVALID` | protected expected root principal | 3 — authoritatively typed | verifier configuration | guarded auth configuration/status | protected authority | remove principal from diagnostic |
| `DPC-379` | same mismatch as `DPC-378` | same route | same meaning | submitted certificate root principal | 1 — caller-derivable | submitted token proof | none | public to submitting caller | discard principal |
| `DPC-380` | chain-key proof schema mismatch | root-proof verification | existing `AUTH_CHAIN_KEY_PROOF_SCHEMA_MISMATCH` | required schema version | 1 — caller-derivable | maintained wire contract | none | public contract | discard numeric value |
| `DPC-381` | same mismatch as `DPC-380` | same route | same meaning | submitted schema version | 1 — caller-derivable | submitted root proof | none | public to submitting caller | discard numeric value |
| `DPC-382` | chain-key root mismatch | same route | existing `AUTH_CHAIN_KEY_ROOT_MISMATCH` | protected expected root principal | 3 — authoritatively typed | verifier policy | guarded auth configuration/status | protected authority | remove principal |
| `DPC-383` | same mismatch as `DPC-382` | same route | same meaning | submitted root principal | 1 — caller-derivable | submitted proof/certificate | none | public to submitting caller | discard principal |
| `DPC-384` | chain-key issuer mismatch | same route | existing `AUTH_CHAIN_KEY_ISSUER_MISMATCH` | certificate issuer principal | 1 — caller-derivable | submitted certificate | none | public to submitting caller | discard principal |
| `DPC-385` | same mismatch as `DPC-384` | same route | same meaning | batch-leaf issuer principal | 1 — caller-derivable | submitted root proof | none | public to submitting caller | discard principal |
| `DPC-386` | header/delegation-cert binding mismatch | same route | existing `AUTH_CHAIN_KEY_HEADER_CERT_MISMATCH` | one of the compared header/leaf field labels | 1 — caller-derivable | submitted root-proof structures | none | public to submitting caller | discard label; proof retains both values |
| `DPC-387` | header/signature binding mismatch | same route | existing `AUTH_CHAIN_KEY_HEADER_SIGNATURE_MISMATCH` | one of the compared header/signature field labels | 1 — caller-derivable | submitted root-proof structures | none | public to submitting caller | discard label |
| `DPC-388` | active-cert/batch-leaf mismatch | same route | existing `AUTH_CHAIN_KEY_CERT_MISMATCH` | one of the compared certificate/leaf field labels | 1 — caller-derivable | submitted token/root proof | none | public to submitting caller | discard label |
| `DPC-389` | verifier-policy binding mismatch | same route | existing `AUTH_CHAIN_KEY_POLICY_MISMATCH` | compared protected-policy field label | 3 — authoritatively typed | verifier policy plus submitted proof | guarded auth configuration/status | protected policy shape | discard label; exact code and status own repair |
| `DPC-390` | proof-epoch floor rejection | same route | existing `AUTH_CHAIN_KEY_PROOF_EPOCH_STALE` | protected minimum proof epoch | 3 — authoritatively typed | verifier policy | guarded auth configuration/status | protected epoch floor | remove numeric value |
| `DPC-391` | same rejection as `DPC-390` | same route | same meaning | submitted proof epoch | 1 — caller-derivable | submitted proof | none | public to submitting caller | discard numeric value |
| `DPC-392` | key-version floor rejection | same route | existing `AUTH_CHAIN_KEY_VERSION_STALE` | protected minimum key version | 3 — authoritatively typed | verifier policy | guarded auth configuration/status | protected key floor | remove numeric value |
| `DPC-393` | same rejection as `DPC-392` | same route | same meaning | submitted key version | 1 — caller-derivable | submitted proof | none | public to submitting caller | discard numeric value |
| `DPC-394` | Registry-epoch floor rejection | same route | existing `AUTH_CHAIN_KEY_REGISTRY_EPOCH_STALE` | protected minimum Registry epoch | 3 — authoritatively typed | verifier policy | guarded auth configuration/status | protected Registry floor | remove numeric value |
| `DPC-395` | same rejection as `DPC-394` | same route | same meaning | submitted Registry epoch | 1 — caller-derivable | submitted proof | none | public to submitting caller | discard numeric value |
| `DPC-396` | chain-key window helper | same route | exact policy- or proof-window identity | one of three static window target labels | 4 — caller-required but unowned | free-form helper target only | closed typed policy/batch/certificate target selecting the registered code | protected when policy-owned | remove target string; typed target selects the exact identity |
| `DPC-397` | root-proof TTL rejection | same route | existing `AUTH_CHAIN_KEY_PROOF_TTL_EXCEEDED` | submitted proof TTL | 1 — caller-derivable | submitted root proof | none | public to submitting caller | discard value |
| `DPC-398` | same rejection as `DPC-397` | same route | same meaning | protected maximum revocation latency | 3 — authoritatively typed | verifier policy | guarded auth configuration/status | protected policy | remove numeric value |
| `DPC-399` | chain-key signature length rejection | same route | existing `AUTH_CHAIN_KEY_SIGNATURE_LENGTH_INVALID` | submitted signature length | 1 — caller-derivable | submitted root proof | none | public to submitting caller | discard value |
| `DPC-400` | zero signature component | same route | existing `AUTH_CHAIN_KEY_SIGNATURE_COMPONENT_ZERO` | static `r` or `s` component label | 1 — caller-derivable | submitted signature | none | public to submitting caller | discard label; caller retains signature bytes |
| `DPC-401` | chain-key signature verifier failure | same route | existing `AUTH_CHAIN_KEY_SIGNATURE_INVALID` | dependency-owned crypto error text | 2 — sensitive operator-only | no safe public text owner; exact proof diagnostic and aggregate auth metric remain | guarded numeric auth observation only | cryptographic implementation detail | discard library text |
| `DPC-402` | chain-key canonicalization wrapper | same route | exact nested canonical-auth diagnostic | typed `CanonicalAuthError` flattened by the broad root-proof mapper | 3 — authoritatively typed | canonical proof/header encoder | transparent registered-code propagation | follows canonical source projection | propagate nested code without wrapper text |

The slice adds eighteen caller-derivable values, one sensitive operator-only
value, thirteen authoritatively typed values and one caller-required-but-
unowned closed target discriminator. The generic `AuthProofCause for String`
has no production cause producer on either maintained call path: cached
verification cannot construct proof-invalid variants, while embedded proof
callbacks return typed `InternalError`. B4 should remove that shape rather than
preserve an untyped proof-error lane.

Across all forty-one classified slices, the dynamic ledger now contains 402
values: 203 caller-derivable, nineteen sensitive operator-only, 148
authoritatively typed and 32 caller-required but unowned.

## Classified Slice 42: Trust Anchors And Chain-Key Signing

This slice follows the delegated-token verifier configuration and chain-key
signing-policy builders. The two builders duplicate several prose helpers, but
the configuration authority and repair action are identical. Repeated helper
sites therefore share one context row rather than manufacturing signer- and
verifier-specific detail identities.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-403` | configured root Canister principal parser | delegated-token verifier construction | existing `AUTH_ROOT_CANISTER_PRINCIPAL_INVALID` | dependency-owned principal parser cause | 2 — sensitive operator-only | no safe public detail owner; checked-in configuration retains the submitted value | guarded configuration validation count | parser detail may reveal input shape | discard parser text |
| `DPC-404` | required chain-key configuration helpers | verifier and signer construction | existing `AUTH_CHAIN_KEY_CONFIG_REQUIRED`, `AUTH_CHAIN_KEY_CONFIG_HEX_INVALID` or `AUTH_CHAIN_KEY_CONFIG_FIXED_LENGTH_INVALID` | free-form field label shared by missing, empty, hex and fixed-length branches | 4 — caller-required but unowned | current `&'static str` helper argument only | closed chain-key configuration-field discriminator used by validation status | guarded configuration | replace the string parameter with a bounded enum; never publish arbitrary field prose |
| `DPC-405` | configured chain-key or IC-root hex decoder | same routes | existing `AUTH_CHAIN_KEY_CONFIG_HEX_INVALID` or `AUTH_IC_ROOT_KEY_HEX_INVALID` | dependency-owned hex decoder cause | 2 — sensitive operator-only | no safe public detail owner; configuration retains the exact submitted hex | guarded configuration validation count | may expose encoded input structure | discard decoder text; typed source selects the code |
| `DPC-406` | fixed 32-byte chain-key value has the wrong size | verifier and signer construction | existing `AUTH_CHAIN_KEY_CONFIG_FIXED_LENGTH_INVALID` | decoded byte length | 1 — caller-derivable | submitted checked-in configuration | none | visible to the configuration owner | discard length; the source field and contract retain it |
| `DPC-407` | configured secp256k1 public-key validator | verifier and signer construction | existing `AUTH_CHAIN_KEY_PUBLIC_KEY_INVALID` | dependency-owned public-key parser cause | 2 — sensitive operator-only | no safe public detail owner; submitted key remains in protected configuration | guarded configuration validation count | cryptographic key-shape detail | discard parser text |
| `DPC-408` | required IC root key is absent | verifier construction | existing `AUTH_IC_ROOT_KEY_REQUIRED` | selected build network | 1 — caller-derivable | checked-in build configuration | none | visible to the build operator | discard interpolation; the build target is already known |
| `DPC-409` | configured raw IC root key has the wrong size | same route | existing `AUTH_IC_ROOT_KEY_LENGTH_INVALID` | maintained raw-key byte-length constant | 1 — caller-derivable | public IC root-key contract | none | public contract | discard numeric prose; contract documentation owns the size |
| `DPC-410` | signer header/policy binding mismatch | chain-key batch signing | existing `AUTH_CHAIN_KEY_SIGNER_HEADER_POLICY_MISMATCH` | compared protected-policy field label | 3 — authoritatively typed | protected batch header and signing policy | durable batch diagnostic plus a closed mismatch-field discriminator if guarded repair needs it | protected policy shape | replace `&'static str`; publish only the exact diagnostic |
| `DPC-411` | enabled chain-key signature parsing, shape or verification failure | same route | existing `AUTH_CHAIN_KEY_SIGNER_SIGNATURE_INVALID` | dependency-owned signature parser/verifier text carried in `ChainKeySignerError::SignatureVerification(String)` | 2 — sensitive operator-only | no safe public text owner; exact batch and aggregate auth metric remain | durable numeric batch diagnostic and guarded numeric observation | cryptographic implementation detail | split the typed signer cause and discard text before persistence or response |
| `DPC-412` | chain-key signing support is not compiled | same route | existing `AUTH_CHAIN_KEY_CRYPTO_UNAVAILABLE` | compiled feature state currently encoded as the same signature-verification string | 3 — authoritatively typed | qualified role build and feature inventory | durable batch diagnostic plus guarded build-capability status | build capability | introduce a distinct typed cause; never infer capability from text |
| `DPC-413` | chain-key derivation-path component is malformed | signer-policy construction | existing `AUTH_CHAIN_KEY_CONFIG_HEX_INVALID` | submitted path-component index | 1 — caller-derivable | checked-in derivation-path configuration | none | visible to the configuration owner | discard index; configuration identifies the component |
| `DPC-414` | same rejection as `DPC-413` | same route | same meaning | dependency-owned hex decoder cause | 2 — sensitive operator-only | no safe public detail owner; configuration retains the submitted component | guarded configuration validation count | may reveal encoded input structure | discard decoder text |

The slice adds four caller-derivable values, five sensitive operator-only
values, two authoritatively typed values and one caller-required-but-unowned
closed field discriminator. It also confirms that verifier and signer config
helpers require one shared typed field model: duplicating the current prose
would create artificial diagnostic identities.

Across all forty-two classified slices, the dynamic ledger now contains 414
values: 207 caller-derivable, 24 sensitive operator-only, 150 authoritatively
typed and 33 caller-required but unowned.

## Classified Slice 43: Durable Chain-Key Batch Failures

This slice follows Registry canonicalization, batch-capacity admission, signing
and issuer-install failure into the durable chain-key batch. The current
`batch.failure: Option<String>` is not one semantic owner: it stores signer
causes, a Registry-staleness label and issuer-install outcomes with different
retry contracts.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-415` | delegated-auth Registry hashing fails | root batch preparation | existing `AUTH_CHAIN_KEY_REGISTRY_CANONICALIZATION_FAILED` or exact nested canonical identity | typed `CanonicalAuthError` flattened into invariant text | 3 — authoritatively typed | protected delegated-auth Registry snapshot and canonical encoder | exact typed propagation plus guarded Registry/batch status | protected Registry shape follows source projection | remove formatted wrapper and preserve the nested code |
| `DPC-416` | pending batch quota is exhausted | root batch preparation | existing `AUTH_CHAIN_KEY_BATCH_CAPACITY_EXCEEDED` | current nonexpired pending-batch count | 3 — authoritatively typed | root-local durable batch index | bounded guarded auth batch status | protected workload state | retain the count only in guarded status; discard public interpolation |
| `DPC-417` | same denial as `DPC-416` | same route | same meaning | maintained maximum pending-batch count | 1 — caller-derivable | maintained bounded auth contract | none | public contract | discard numeric prose; documentation owns the ceiling |
| `DPC-418` | batch signing fails | signing response and durable batch failure | exact `ChainKeySignerError` identity or nested management diagnostic | typed signer cause flattened twice: into `batch.failure` and broad ops text | 3 — authoritatively typed | exact signing policy, batch and typed signer result | durable numeric batch failure with typed terminal/retryable disposition | source-specific projection; management detail follows its nested cause | remove both string sinks and propagate the exact diagnostic once |
| `DPC-419` | issuer proof installation fails | durable per-issuer and aggregate batch failure | exact one of four typed install-failure meanings | `ChainKeyRootDelegationInstallFailure` debug rendering | 3 — authoritatively typed | workflow-classified issuer install outcome | durable numeric per-issuer failure plus derived aggregate batch status | protected issuer/install state | persist the typed diagnostic, not `Debug`; aggregate state must not erase per-issuer meaning |
| `DPC-420` | prepared delegation certificate hashing fails | batch leaf construction | existing `AUTH_CHAIN_KEY_CERT_CANONICALIZATION_FAILED` or exact nested canonical identity | typed `CanonicalAuthError` flattened into invariant text | 3 — authoritatively typed | prepared certificate and canonical encoder | exact typed propagation plus durable batch preparation status if retained | protected certificate shape follows source projection | remove formatted wrapper and preserve the nested code |

The slice adds one caller-derivable value and five authoritatively typed values.
It also closes the durable ownership decision: B5 must replace the shared
string field with a discriminated numeric failure record. Registry staleness,
signing failure and per-issuer installation failure remain distinct states;
none may be recovered later by parsing stored prose.

Across all forty-three classified slices, the dynamic ledger now contains 420
values: 208 caller-derivable, 24 sensitive operator-only, 155 authoritatively
typed and 33 caller-required but unowned.

## Classified Slice 44: Typed Auth Scope And Attestation Time

This slice closes the dynamic fields hidden by the terminal
`AuthOpsError::to_string()` conversion for producer-reachable scope and
attestation-time errors. Four unused delegated-token `AuthExpiryError` variants
add no rows: the maintained delegated-token enums own those meanings and B4
removes the dead variants.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-421` | terminal `AuthOpsError` conversion | every base typed auth rejection | exact nested validation, signature, scope or expiry diagnostic | typed aggregate flattened to broad ops text | 3 — authoritatively typed | exact source enum and producer | exhaustive registered-code dispatch | follows source projection | delete aggregate formatting and preserve the nested identity |
| `DPC-422` | attestation window is not ordered | role-attestation verification | existing `AUTH_ATTESTATION_WINDOW_INVALID` | submitted issue time | 1 — caller-derivable | signed attestation payload | none | public to submitting caller | discard timestamp; caller retains the proof |
| `DPC-423` | same rejection as `DPC-422` | same route | same meaning | submitted expiry time | 1 — caller-derivable | signed attestation payload | none | public to submitting caller | discard timestamp |
| `DPC-424` | active certificate issuer mismatch | delegated-token preparation | existing `AUTH_ISSUER_PRINCIPAL_MISMATCH` | protected expected local issuer principal | 3 — authoritatively typed | initialized Canister identity | guarded auth/runtime status | protected local authority | remove principal from the diagnostic |
| `DPC-425` | same mismatch as `DPC-424` | same route | same meaning | submitted active-certificate issuer principal | 1 — caller-derivable | active proof selected for the caller's request | none | visible to the affected caller | discard principal |
| `DPC-426` | attestation subject mismatch | role-attestation verification | existing `AUTH_ATTESTATION_SUBJECT_MISMATCH` | transport caller expected by the verifier | 1 — caller-derivable | IC caller | none | public to caller | discard principal |
| `DPC-427` | same mismatch as `DPC-426` | same route | same meaning | submitted attestation subject | 1 — caller-derivable | signed attestation payload | none | public to submitting caller | discard principal |
| `DPC-428` | attestation audience mismatch | same route | existing `AUTH_ATTESTATION_AUDIENCE_MISMATCH` | receiver Canister principal | 1 — caller-derivable | IC call target | none | public to caller | discard principal |
| `DPC-429` | same mismatch as `DPC-428` | same route | same meaning | submitted attestation audience | 1 — caller-derivable | signed attestation payload | none | public to submitting caller | discard principal |
| `DPC-430` | attestation Subnet mismatch | local-Subnet attestation verification | existing `AUTH_ATTESTATION_SUBNET_MISMATCH` | IC-native live receiver Subnet | 3 — authoritatively typed | initialized live receiver-Subnet evidence | guarded local-auth status | protected placement authority | remove principal from the diagnostic |
| `DPC-431` | same mismatch as `DPC-430` | same route | same meaning | submitted attestation Subnet | 1 — caller-derivable | signed attestation payload | none | public to submitting caller | discard principal |
| `DPC-432` | attestation is not yet valid | role-attestation verification | existing `AUTH_ATTESTATION_NOT_YET_VALID` | submitted issue time | 1 — caller-derivable | signed attestation payload | none | public to submitting caller | discard timestamp |
| `DPC-433` | same rejection as `DPC-432` | same route | same meaning | trusted verification time | 3 — authoritatively typed | IC time adapter | guarded auth recent-failure observation | runtime evidence | discard timestamp |
| `DPC-434` | attestation is expired | same route | existing `AUTH_ATTESTATION_EXPIRED` | submitted expiry time | 1 — caller-derivable | signed attestation payload | none | public to submitting caller | discard timestamp |
| `DPC-435` | same rejection as `DPC-434` | same route | same meaning | trusted verification time | 3 — authoritatively typed | IC time adapter | guarded auth recent-failure observation | runtime evidence | discard timestamp |
| `DPC-436` | attestation role epoch is stale | same route | existing `AUTH_ATTESTATION_EPOCH_REJECTED` | submitted attestation epoch | 1 — caller-derivable | signed attestation payload | none | public to submitting caller | discard numeric value |
| `DPC-437` | same rejection as `DPC-436` | same route | same meaning | protected minimum accepted role epoch | 3 — authoritatively typed | local role-attestation authority generation | guarded auth/runtime status | protected revocation floor | remove numeric value from the diagnostic |

The slice adds eleven caller-derivable and six authoritatively typed values.
The transport caller and receiver principal remain public call facts; live
Subnet, local issuer, time and epoch-floor evidence remain protected authority.

Across all forty-four classified slices, the dynamic ledger now contains 437
values: 219 caller-derivable, 24 sensitive operator-only, 161 authoritatively
typed and 33 caller-required but unowned.

## Classified Slice 45: Audience, Canonical And Certificate Fields

The same nested auth types validate both caller-submitted token material and
protected root certificate/Registry state. A value with both origins is
classified as authoritatively typed: the enclosing preparation, verification
or batch variant must select its exposure. Caller-input use at one site never
authorizes protected-policy disclosure at another.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-438` | delegated role-grant count exceeds its bound | token/certificate validation | existing `AUTH_AUDIENCE_GRANT_COUNT_EXCEEDED` | submitted or protected grant count | 3 — authoritatively typed | exact request/proof or protected issuer policy selected by the enclosing variant | source-specific request rejection or guarded issuer-policy status | path-dependent | discard count from public text |
| `DPC-439` | same rejection as `DPC-438` | same routes | same meaning | maintained maximum grant count | 1 — caller-derivable | public delegated-auth contract | none | public contract | discard numeric prose |
| `DPC-440` | a role grant has no scopes | same routes | existing `AUTH_AUDIENCE_GRANT_SCOPES_EMPTY` | submitted or protected role label | 3 — authoritatively typed | exact grant owner selected by the enclosing variant | source-specific request rejection or guarded issuer-policy status | path-dependent | discard role text |
| `DPC-441` | a role grant exceeds its scope bound | same routes | existing `AUTH_AUDIENCE_GRANT_SCOPE_COUNT_EXCEEDED` | submitted or protected role label | 3 — authoritatively typed | exact grant owner selected by the enclosing variant | source-specific request rejection or guarded issuer-policy status | path-dependent | discard role text |
| `DPC-442` | same rejection as `DPC-441` | same routes | same meaning | submitted or protected scope count | 3 — authoritatively typed | exact grant owner selected by the enclosing variant | source-specific request rejection or guarded issuer-policy status | path-dependent | discard count |
| `DPC-443` | same rejection as `DPC-441` | same routes | same meaning | maintained maximum scopes per grant | 1 — caller-derivable | public delegated-auth contract | none | public contract | discard numeric prose |
| `DPC-444` | grant scope label is invalid | same routes | existing `AUTH_CANONICAL_SCOPE_INVALID` | submitted or protected scope label | 3 — authoritatively typed | exact request/proof or issuer policy selected by the enclosing variant | source-specific rejection/status | path-dependent | discard scope text |
| `DPC-445` | canonical role label is invalid | token, certificate, Registry or batch canonicalization | existing `AUTH_CANONICAL_ROLE_INVALID` | submitted or protected role label | 3 — authoritatively typed | exact canonical input selected by the typed wrapper | source-specific request rejection or guarded canonical-state status | path-dependent | discard role text |
| `DPC-446` | canonical scope label is invalid | same routes | existing `AUTH_CANONICAL_SCOPE_INVALID` | submitted or protected scope label | 3 — authoritatively typed | exact canonical input selected by the typed wrapper | source-specific request rejection or guarded canonical-state status | path-dependent | discard scope text |
| `DPC-447` | token extension exceeds its bound | token preparation/verification | existing `AUTH_CANONICAL_TOKEN_EXTENSION_TOO_LARGE` | submitted extension byte length | 1 — caller-derivable | submitted token request or claims | none | public to submitting caller | discard length |
| `DPC-448` | same rejection as `DPC-447` | same route | same meaning | maintained extension byte ceiling | 1 — caller-derivable | public delegated-auth contract | none | public contract | discard numeric prose |
| `DPC-449` | certificate root authority mismatch | certificate verification | existing `AUTH_ROOT_AUTHORITY_INVALID` | protected expected root principal | 3 — authoritatively typed | local verifier policy | guarded auth configuration/status | protected root authority | remove principal |
| `DPC-450` | same mismatch as `DPC-449` | same route | same meaning | submitted certificate root principal | 1 — caller-derivable | submitted certificate | none | public to submitting caller | discard principal |
| `DPC-451` | certificate TTL exceeds its bound | certificate preparation/verification | existing `AUTH_CERT_TTL_EXCEEDED` | submitted or protected certificate TTL | 3 — authoritatively typed | exact certificate or protected issuance plan selected by the wrapper | source-specific request rejection or guarded issuer/batch status | path-dependent | discard numeric value |
| `DPC-452` | same rejection as `DPC-451` | same routes | same meaning | protected maximum certificate TTL | 3 — authoritatively typed | verifier or issuer policy | guarded auth configuration/status | protected policy | remove numeric value |
| `DPC-453` | certificate token ceiling exceeds verifier policy | same routes | existing `AUTH_CERT_MAX_TOKEN_TTL_EXCEEDED` | submitted or protected certificate token ceiling | 3 — authoritatively typed | exact certificate or issuance plan selected by the wrapper | source-specific rejection/status | path-dependent | discard numeric value |
| `DPC-454` | same rejection as `DPC-453` | same routes | same meaning | protected maximum token TTL | 3 — authoritatively typed | verifier or issuer policy | guarded auth configuration/status | protected policy | remove numeric value |
| `DPC-455` | certificate token ceiling outlives the certificate | same routes | existing `AUTH_CERT_MAX_TOKEN_TTL_OUTLIVES_CERT` | submitted or protected token ceiling | 3 — authoritatively typed | exact certificate or issuance plan selected by the wrapper | source-specific rejection/status | path-dependent | discard numeric value |
| `DPC-456` | same rejection as `DPC-455` | same routes | same meaning | derived submitted or protected certificate TTL | 3 — authoritatively typed | exact certificate or issuance plan selected by the wrapper | source-specific rejection/status | path-dependent | discard numeric value |

The slice adds five caller-derivable and fourteen authoritatively typed values.
The bounded constants remain public contract; every mixed-origin role, scope,
count or TTL follows the enclosing typed source rather than the formatter.

Across all forty-five classified slices, the dynamic ledger now contains 456
values: 224 caller-derivable, 24 sensitive operator-only, 175 authoritatively
typed and 33 caller-required but unowned.

## Classified Slice 46: Issuer Proof And Signature Buckets

Issuer Canister-signature handling duplicates the root proof's dependency-text
path, then both paths enter the generic signature string buckets. This slice
records the issuer-specific sources and the final information-loss boundary.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-457` | issuer signature map reports no signature | prepared issuer-proof retrieval | exact issuer-proof missing/invalid identity | typed dependency `NoSignature` variant flattened to text | 3 — authoritatively typed | issuer Canister-signature map result | exact typed mapping plus guarded auth preparation observation | implementation detail | select the code from the typed variant and discard display text |
| `DPC-458` | issuer Canister-signature public-key DER parser | delegated-token issuer-proof verification | existing `AUTH_DELEGATION_PROOF_INVALID` projection | dependency-owned parser text | 2 — sensitive operator-only | no safe public detail owner; exact invalid-proof code and aggregate metric remain | guarded numeric auth observation only | malformed key structure | discard parser text |
| `DPC-459` | IC issuer Canister-signature verifier | same route | same invalid-proof family as `DPC-458` | dependency-owned verifier text | 2 — sensitive operator-only | no safe public detail owner; exact invalid-proof code and aggregate metric remain | guarded numeric auth observation only | cryptographic verification detail | discard verifier text |
| `DPC-460` | terminal delegation-proof signature conversion | root or issuer proof response | exact proof-unavailable/invalid identity and safe `AUTH_PROOF_INVALID` projection | `AuthSignatureError::ProofInvalid(String)` payload formatted again into public error text | 2 — sensitive operator-only | exact typed root/issuer proof producer before the string bucket | typed proof-cause enum plus guarded numeric auth observation | inherits cryptographic/parser sensitivity | delete the String payload and never persist or reformat its content |
| `DPC-461` | terminal attestation-proof signature conversion | role-attestation verification | exact attestation-invalid identity and safe `AUTH_PROOF_INVALID` projection | `AuthSignatureError::AttestationProofInvalid(String)` payload formatted again | 2 — sensitive operator-only | exact attestation hasher/verifier cause before the string bucket | typed attestation-proof cause plus guarded numeric observation | inherits proof/configuration sensitivity | delete the String payload and preserve only the typed cause/code |
| `DPC-462` | proof-unavailable signature conversion | root or issuer proof operation | existing `AUTH_PROOF_UNAVAILABLE` | typed `AuthSignatureError::ProofUnavailable` flattened to text | 3 — authoritatively typed | exact compiled capability/preparation state | direct registered-code dispatch | public safe meaning | remove formatting; emit the exact code directly |
| `DPC-463` | retained delegated-token retrieval window expired | prepared-token retrieval | existing `AUTH_TOKEN_RETRIEVAL_EXPIRED` | durable retrieval-expiry timestamp flattened through the lookup error | 3 — authoritatively typed | retained token record for the exact preparation | operation-correlated preparation status | operation state | retain timestamp only in bounded guarded status; publish the exact code |

The slice adds four sensitive operator-only and three authoritatively typed
values. Root and issuer proof implementations may share the safe projection,
but their preparation/status owners remain distinct. Retained-token expiry
stays bound to its exact preparation rather than becoming public prose.

Across all forty-six classified slices, the dynamic ledger now contains 463
values: 224 caller-derivable, 28 sensitive operator-only, 178 authoritatively
typed and 33 caller-required but unowned.

## Classified Slice 47: Component Registry Workflow Adapters

The workflow's direct formatted denials were already classified in slice 5.
This slice covers its remaining nine production `format!` sites: one Candid
cursor encoder, one shared Store-artifact role and six typed topology-binding
wrappers. The duplicate-artifact branch contains no dynamic value.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-464` | Component Directory next-cursor encoding | Directory paging | existing `COMPONENT_DIRECTORY_CURSOR_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical cursor payload and encoder | guarded Registry recent-failure observation | implementation/encoded-state detail | discard encoder text; emit no fabricated cursor |
| `DPC-465` | verified Store catalog lacks the reserved role | top-level or child creation planning | corrected `COMPONENT_STORE_ARTIFACT_UNAVAILABLE` | admitted Component or child role | 1 — caller-derivable | exact allocation request and protected release-set catalog | none | visible to the exact provisioning caller | discard role; request and catalog retain it |
| `DPC-466` | stored peer requester binding validation | peer Component provisioning recovery | exact nested `ComponentTopologyError` identity | typed topology error flattened into a storage invariant | 3 — authoritatively typed | protected topology plus stored provisioning origin | transparent registered-code propagation | protected binding fields follow nested projection | remove wrapper text and preserve the exact topology diagnostic |
| `DPC-467` | subtree-removal target binding validation | bounded post-order removal | exact nested `ComponentTopologyError` identity | typed topology error flattened into a storage invariant | 3 — authoritatively typed | protected topology, target row and removal fence | transparent registered-code propagation plus removal status | protected target authority | remove wrapper text and preserve the exact topology diagnostic |
| `DPC-468` | registered top-level parent binding validation | descendant allocation/recovery | exact nested `ComponentTopologyError` identity | typed topology error flattened into a storage invariant | 3 — authoritatively typed | protected topology and registered parent binding | transparent registered-code propagation | protected parent authority | remove wrapper text and preserve the exact topology diagnostic |
| `DPC-469` | registered child parent binding validation | same routes as `DPC-468` | exact nested `ComponentTopologyError` identity | typed topology error flattened into a storage invariant | 3 — authoritatively typed | protected topology and registered immediate-parent binding | transparent registered-code propagation | protected parent authority | remove wrapper text and preserve the exact topology diagnostic |
| `DPC-470` | committed Component partition binding validation | Directory/runtime/draining validation | exact nested `ComponentTopologyError` identity | typed topology error flattened into a storage invariant | 3 — authoritatively typed | protected topology and committed partition | transparent registered-code propagation plus Registry recent-failure owner | protected Component authority | remove wrapper text and preserve the exact topology diagnostic |
| `DPC-471` | Component Directory caller child binding validation | Directory paging | exact nested `ComponentTopologyError` identity | typed topology error flattened into a storage invariant | 3 — authoritatively typed | protected topology, partition and registered member binding | transparent registered-code propagation plus Directory observation | protected caller membership | remove wrapper text and preserve the exact topology diagnostic |

The slice adds one caller-derivable, one sensitive operator-only and six
authoritatively typed values. It also corrects the shared Store-artifact labels:
top-level and child creation use the same helper, so child-only diagnostic names
or projections would contradict one maintained call path.

Across all forty-seven classified slices, the dynamic ledger now contains 471
values: 225 caller-derivable, 29 sensitive operator-only, 184 authoritatively
typed and 33 caller-required but unowned.

## Classified Slice 48: Component Registry Canonical Encoders

Fourteen Component Registry ops sites append dependency-owned Candid encoder
text to distinct protected authority failures. Each semantic identity is
already exact; the library prose is neither allocation authority nor a safe
public detail.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-472` | initial Component inventory encoding | root activation sealing | existing `ROOT_INITIAL_INVENTORY_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical initial-inventory input and exact encoder site | guarded root activation recent-failure observation | protected initial inventory | discard library text; do not seal |
| `DPC-473` | terminal Component history encoding | root final-inventory preparation | existing `TERMINAL_COMPONENT_HISTORY_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical terminal-history input and exact encoder site | guarded final-inventory recent-failure observation | protected terminal history | discard library text; do not advance |
| `DPC-474` | terminal root Store catalog encoding | same route | existing `ROOT_FINAL_INVENTORY_STORE_CATALOG_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical retained Store catalog and exact encoder site | guarded final-inventory recent-failure observation | protected Store catalog | discard library text |
| `DPC-475` | Fleet Subnet Root final-inventory encoding | same route | existing `ROOT_FINAL_INVENTORY_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical final-inventory authority and exact encoder site | guarded final-inventory recent-failure observation | protected root inventory | discard library text |
| `DPC-476` | Store reclamation receipt encoding | root Store reclamation | existing `ROOT_STORE_RECLAMATION_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical reclamation receipt and exact encoder site | operation-correlated reclamation status | protected Store lifecycle evidence | discard library text; do not advance |
| `DPC-477` | Store binding-finalization receipt encoding | root Store binding finalization | existing `ROOT_STORE_BINDING_FINALIZATION_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical finalization receipt and exact encoder site | operation-correlated binding-finalization status | protected Store authority | discard library text; do not advance |
| `DPC-478` | Store deletion receipt encoding | root Store deletion | existing `ROOT_STORE_DELETION_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical deletion receipt and exact encoder site | operation-correlated Store-deletion status | destructive-effect evidence | discard library text; do not conclude deletion |
| `DPC-479` | Component membership-removal hash encoding | top-level Component removal | existing `COMPONENT_MEMBERSHIP_REMOVAL_HASH_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical removal authority and exact encoder site | operation-correlated Component removal status | protected terminal membership | discard library text; do not settle removal |
| `DPC-480` | completed subtree-leaf receipt encoding | bounded subtree removal | existing `COMPONENT_SUBTREE_COMPLETED_LEAF_RECEIPT_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical synchronized leaf receipt and exact encoder site | operation-correlated subtree-removal status | protected descendant evidence | discard library text; do not finalize the leaf |
| `DPC-481` | Component Registry partition-head encoding | Registry partition mutation | existing `COMPONENT_REGISTRY_PARTITION_HASH_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical partition authority and exact encoder site | guarded partition recent-failure observation | protected Registry state | discard library text; do not commit |
| `DPC-482` | Component final-inventory encoding | Component draining/finalization | existing `COMPONENT_FINAL_INVENTORY_HASH_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical Component final inventory and exact encoder site | operation-correlated draining status | protected final inventory | discard library text; do not finalize |
| `DPC-483` | descendant commitment-digest encoding | child Registry commitment | existing `COMPONENT_DESCENDANT_COMMIT_DIGEST_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical descendant commitment and exact encoder site | operation-correlated child allocation status | protected child authority | discard library text; do not commit |
| `DPC-484` | descendant activation-digest encoding | child membership activation | existing `COMPONENT_DESCENDANT_ACTIVATION_DIGEST_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical descendant activation and exact encoder site | operation-correlated child activation status | protected child authority | discard library text; do not activate |
| `DPC-485` | descendant removal-digest encoding | child membership removal | existing `COMPONENT_DESCENDANT_REMOVAL_DIGEST_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical descendant removal and exact encoder site | operation-correlated subtree-removal status | protected child authority | discard library text; do not remove membership |

The slice adds fourteen sensitive operator-only values. The exact encoder site
already selects the diagnostic; B4 must never parse `candid::Error` text to
recover a more specific identity.

Across all forty-eight classified slices, the dynamic ledger now contains 485
values: 225 caller-derivable, 43 sensitive operator-only, 184 authoritatively
typed and 33 caller-required but unowned.

## Classified Slice 49: Component Registry Capacity And Accounting

This slice closes the remaining twenty-six production `format!` sites in
Component Registry ops after the canonical encoders. Twenty-five sites report
one projected byte count and one ceiling; the shared byte-replacement helper
adds one closed, static accounting label. Each interpolation is classified
separately because a protected ceiling can already have a typed owner while
the rejected canonical projection remains unavailable.

The normal capacity rows expose a real B1 gap. Current Component Registry and
partition status report committed `encoded_bytes`, but no current endpoint can
reconstruct the exact rejected projection when a canonical record replaces or
extends protected state. B4 must therefore add an operation-specific preflight
or status projection calculated by the same ops helper. It must not add a
root-global last-error slot or generic diagnostic detail.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-486` | `reserve_allocation`; root allocation reservation | top-level Component reservation | existing `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | rejected projected root Registry bytes | 4 — caller-required but unowned | none; `RootComponentRegistryStatusResponse.encoded_bytes` is the current committed count, not the rejected projection | request-scoped top-level allocation capacity projection keyed by the submitted operation ID | guarded operator | add the exact projection owner before removing the number |
| `DPC-487` | same rejection as `DPC-486` | same route | same meaning | immutable root Registry byte ceiling | 3 — authoritatively typed | matching `FleetRegistry.fleet_subnet_roots[].limits.maximum_registry_bytes` from the Coordinator Registry | none | guarded operator | discard the interpolation and retrieve the frozen limit from the Registry |
| `DPC-488` | `begin_subtree_removal`; fence-state construction | Component subtree-removal start | existing `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | rejected projected Component partition bytes | 4 — caller-required but unowned | none; `ComponentRegistryPartitionResponse.encoded_bytes` remains the pre-fence committed count | subtree-removal capacity projection bound to operation, Component, target and expected head | guarded operator | add the exact projection owner before removing the number |
| `DPC-489` | same rejection as `DPC-488` | same route | same meaning | effective Component Registry byte ceiling | 1 — caller-derivable | checked-in Component Spec limit plus the protected deployment-member reduction, if any | none | guarded operator | discard the interpolation; the admitted deployment contract determines the ceiling |
| `DPC-490` | same subtree-fence commit | same route | same meaning | rejected projected root Registry bytes | 4 — caller-required but unowned | none; root status exposes only the committed count | the same subtree-removal projection with `projected_root_registry_bytes` | guarded operator | add the exact projection owner before removing the number |
| `DPC-491` | same rejection as `DPC-490` | same route | same meaning | immutable root Registry byte ceiling | 3 — authoritatively typed | matching Fleet Registry root entry | none | guarded operator | discard and use Registry authority |
| `DPC-492` | `reserve_child_allocation`; child reservation commit | direct-child reservation | existing `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | rejected projected Component partition bytes | 4 — caller-required but unowned | none; no child operation is retained when reservation capacity rejects | request-scoped child-reservation capacity projection bound to operation, parent, role pair and expected Component head | exact registered parent and operator | add the exact projection owner before removing the number |
| `DPC-493` | same rejection as `DPC-492` | same route | same meaning | effective Component Registry byte ceiling | 1 — caller-derivable | admitted Component Spec/deployment limit retained by the exact parent runtime | none | exact registered parent and operator | discard the interpolation |
| `DPC-494` | same child-reservation commit | same route | same meaning | rejected projected root Registry bytes | 4 — caller-required but unowned | none; root status exposes only the committed count | the same child-reservation projection with `projected_root_registry_bytes` | guarded operator | add the exact projection owner before removal |
| `DPC-495` | same rejection as `DPC-494` | same route | same meaning | immutable root Registry byte ceiling | 3 — authoritatively typed | matching Fleet Registry root entry | none | guarded operator | discard and use Registry authority |
| `DPC-496` | `commit_verified`; terminal prepared-partition check after install precharge | top-level Component commitment | corrected `COMPONENT_COMMITMENT_COMPONENT_BYTE_RESERVATION_INVALID` | reconstructed prepared partition bytes | 2 — sensitive operator-only | protected allocation, partition candidate and frozen pre-install charge | guarded Component Registry recent-failure code/counter only | protected accounting evidence | discard the numeric operand; this branch is a reservation contradiction, not retryable capacity exhaustion |
| `DPC-497` | same contradiction as `DPC-496` | same route | same corrected meaning | protected Component byte ceiling | 2 — sensitive operator-only | compiled topology/deployment authority used by the frozen install plan | guarded recent-failure code/counter only | protected deployment authority | discard the numeric operand |
| `DPC-498` | `activate_membership`; terminal active-partition check after install precharge | top-level Component membership activation | corrected `COMPONENT_MEMBERSHIP_COMPONENT_BYTE_RESERVATION_INVALID` | reconstructed active partition bytes | 2 — sensitive operator-only | protected committed allocation, active candidate and frozen pre-install charge | guarded Component Registry recent-failure code/counter only | protected accounting evidence | discard the number; do not advise freeing capacity or unchanged retry |
| `DPC-499` | same contradiction as `DPC-498` | same route | same corrected meaning | protected Component byte ceiling | 2 — sensitive operator-only | compiled topology/deployment authority used by the frozen install plan | guarded recent-failure code/counter only | protected deployment authority | discard the numeric operand |
| `DPC-500` | `replace_encoded_bytes`; two Directory-refresh callers | Fleet-service Component Directory refresh | existing split `COMPONENT_REGISTRY_BYTE_ACCOUNTING_UNDERFLOW` / `COMPONENT_REGISTRY_BYTE_COUNT_OVERFLOW` | closed static helper label selecting Component-partition or root-Registry accounting | 2 — sensitive operator-only | exact call site and checked arithmetic edge | exact registered accounting code plus guarded Registry recent-failure observation | internal operation label | remove the string parameter after splitting subtract-underflow from add-overflow; never parse the label |
| `DPC-501` | `subtree_removal_progress_state` | bounded subtree-removal advancement | existing `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | rejected projected Component partition bytes | 4 — caller-required but unowned | none; operation status retains progress and the current partition, not the rejected next projection | subtree-removal status/preflight projection for the exact operation and next bounded step | guarded operator | add the projection owner before removal |
| `DPC-502` | same rejection as `DPC-501` | same route | same meaning | effective Component Registry byte ceiling | 1 — caller-derivable | admitted Component Spec/deployment contract | none | guarded operator | discard the interpolation |
| `DPC-503` | same subtree-progress step | same route | same meaning | rejected projected root Registry bytes | 4 — caller-required but unowned | none; root status retains only current committed bytes | the same subtree-removal projection with root scope | guarded operator | add the projection owner before removal |
| `DPC-504` | same rejection as `DPC-503` | same route | same meaning | immutable root Registry byte ceiling | 3 — authoritatively typed | matching Fleet Registry root entry | none | guarded operator | discard and use Registry authority |
| `DPC-505` | `component_draining_state` | Component draining fence | existing `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | rejected projected Component partition bytes | 4 — caller-required but unowned | none; draining status cannot expose a fence that failed before commit | operation-scoped Component-draining capacity projection bound to Component and expected head | guarded operator | add the projection owner before removal |
| `DPC-506` | same rejection as `DPC-505` | same route | same meaning | effective Component Registry byte ceiling | 1 — caller-derivable | admitted Component Spec/deployment contract | none | guarded operator | discard the interpolation |
| `DPC-507` | same draining-state construction | same route | same meaning | rejected projected root Registry bytes | 4 — caller-required but unowned | none; current root status is not the rejected projection | the same draining projection with root scope | guarded operator | add the projection owner before removal |
| `DPC-508` | same rejection as `DPC-507` | same route | same meaning | immutable root Registry byte ceiling | 3 — authoritatively typed | matching Fleet Registry root entry | none | guarded operator | discard and use Registry authority |
| `DPC-509` | `component_quiescence_intent_state` | qualified top-level Component quiescence | existing `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | rejected projected Component partition bytes | 4 — caller-required but unowned | none; quiescence status retains the current journal, not a rejected intent projection | operation-scoped quiescence capacity projection for the exact stop intent | guarded operator | add the projection owner before removal |
| `DPC-510` | same rejection as `DPC-509` | same route | same meaning | effective Component Registry byte ceiling | 1 — caller-derivable | admitted Component Spec/deployment contract | none | guarded operator | discard the interpolation |
| `DPC-511` | same quiescence-intent construction | same route | same meaning | rejected projected root Registry bytes | 4 — caller-required but unowned | none | the same quiescence projection with root scope | guarded operator | add the projection owner before removal |
| `DPC-512` | same rejection as `DPC-511` | same route | same meaning | immutable root Registry byte ceiling | 3 — authoritatively typed | matching Fleet Registry root entry | none | guarded operator | discard and use Registry authority |
| `DPC-513` | `subtree_removal_leaf_finalization_state` | bounded subtree leaf finalization | existing `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | rejected projected Component partition bytes | 4 — caller-required but unowned | none; retained removal progress precedes the rejected completed-leaf projection | exact subtree-operation/leaf capacity projection for the next finalization step | guarded operator | add the projection owner before removal |
| `DPC-514` | same rejection as `DPC-513` | same route | same meaning | effective Component Registry byte ceiling | 1 — caller-derivable | admitted Component Spec/deployment contract | none | guarded operator | discard the interpolation |
| `DPC-515` | same leaf-finalization state | same route | same meaning | rejected projected root Registry bytes | 4 — caller-required but unowned | none | the same leaf-finalization projection with root scope | guarded operator | add the projection owner before removal |
| `DPC-516` | same rejection as `DPC-515` | same route | same meaning | immutable root Registry byte ceiling | 3 — authoritatively typed | matching Fleet Registry root entry | none | guarded operator | discard and use Registry authority |
| `DPC-517` | `converge_subtree_membership_removal_bytes` | subtree leaf membership removal | existing `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | rejected projected Component partition bytes | 4 — caller-required but unowned | none; current removal receipt has not committed this projection | exact subtree-operation/leaf membership-removal capacity projection | guarded operator | add the projection owner before removal |
| `DPC-518` | same rejection as `DPC-517` | same route | same meaning | effective Component Registry byte ceiling | 1 — caller-derivable | admitted Component Spec/deployment contract | none | guarded operator | discard the interpolation |
| `DPC-519` | same membership-removal convergence | same route | same meaning | rejected projected root Registry bytes | 4 — caller-required but unowned | none | the same membership-removal projection with root scope | guarded operator | add the projection owner before removal |
| `DPC-520` | same rejection as `DPC-519` | same route | same meaning | immutable root Registry byte ceiling | 3 — authoritatively typed | matching Fleet Registry root entry | none | guarded operator | discard and use Registry authority |
| `DPC-521` | `child_creation_capacity` | direct-child creation intent | existing `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | rejected projected Component partition bytes | 4 — caller-required but unowned | none; child allocation status exposes the reservation but not this pre-effect projection | child-allocation capacity projection for the exact operation and creation edge | exact registered parent and operator | add the projection owner before removing the number |
| `DPC-522` | same rejection as `DPC-521` | same route | same meaning | frozen Component Registry byte ceiling | 3 — authoritatively typed | `RootComponentChildAllocationResponse.maximum_registry_bytes` from the exact child allocation status | none | exact registered parent and operator | discard and use operation status |
| `DPC-523` | same child-creation precharge | same route | same meaning | rejected projected root Registry bytes | 4 — caller-required but unowned | none | the same child-allocation creation projection with root scope | guarded operator | add the projection owner before removal |
| `DPC-524` | same rejection as `DPC-523` | same route | same meaning | immutable root Registry byte ceiling | 3 — authoritatively typed | matching Fleet Registry root entry | none | guarded operator | discard and use Registry authority |
| `DPC-525` | `child_install_capacity` | direct-child installation intent | existing `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | rejected projected Component partition bytes | 4 — caller-required but unowned | none; allocation status retains the created child but not this terminal precharge projection | child-allocation capacity projection for the exact operation and install edge | exact registered parent and operator | add the projection owner before removing the number |
| `DPC-526` | same rejection as `DPC-525` | same route | same meaning | frozen Component Registry byte ceiling | 3 — authoritatively typed | `RootComponentChildAllocationResponse.maximum_registry_bytes` from the exact child allocation status | none | exact registered parent and operator | discard and use operation status |
| `DPC-527` | same child-install precharge | same route | same meaning | rejected projected root Registry bytes | 4 — caller-required but unowned | none | the same child-allocation install projection with root scope | guarded operator | add the projection owner before removal |
| `DPC-528` | same rejection as `DPC-527` | same route | same meaning | immutable root Registry byte ceiling | 3 — authoritatively typed | matching Fleet Registry root entry | none | guarded operator | discard and use Registry authority |
| `DPC-529` | `install_charged_entry_bytes`; conservative terminal top-level footprint | top-level Component installation intent | existing `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | rejected maximum terminal Component footprint | 4 — caller-required but unowned | none; allocation status omits the conservative charge and effective limit | operation-scoped top-level install-capacity projection calculated before the install effect | guarded operator | add the exact preflight/status owner before removing the number |
| `DPC-530` | same rejection as `DPC-529` | same route | same meaning | effective Component Registry byte ceiling | 1 — caller-derivable | admitted Component Spec/deployment contract selected by the protected install plan | none | guarded operator | discard the interpolation |
| `DPC-531` | `persist_child_membership_activation`; terminal active-child partition check after install precharge | child membership activation | corrected `COMPONENT_CHILD_MEMBERSHIP_COMPONENT_BYTE_RESERVATION_INVALID` | reconstructed active Component partition bytes | 2 — sensitive operator-only | protected child allocation, active candidate and frozen pre-install charge | guarded Component Registry recent-failure code/counter only | protected accounting evidence | discard the number; this is a reservation contradiction, not normal exhaustion |
| `DPC-532` | same contradiction as `DPC-531` | same route | same corrected meaning | frozen Component Registry byte ceiling | 2 — sensitive operator-only | exact child allocation record | guarded recent-failure code/counter only | protected allocation authority | discard the numeric operand |
| `DPC-533` | `validate_install_capacity`; top-level installation precharge | top-level Component installation intent | existing `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | rejected projected root Registry bytes | 4 — caller-required but unowned | none; root status exposes only the committed pre-intent count | operation-scoped top-level install-capacity projection with root scope | guarded operator | add the projection owner before removal |
| `DPC-534` | same rejection as `DPC-533` | same route | same meaning | immutable root Registry byte ceiling | 3 — authoritatively typed | matching Fleet Registry root entry | none | guarded operator | discard and use Registry authority |
| `DPC-535` | `validate_creation_capacity`; top-level creation precharge | top-level Component creation intent | existing `COMPONENT_REGISTRY_BYTE_CAPACITY_EXHAUSTED` | rejected projected root Registry bytes | 4 — caller-required but unowned | none; allocation status retains the reservation but not the rejected creation projection | operation-scoped top-level creation-capacity projection | guarded operator | add the projection owner before removal |
| `DPC-536` | same rejection as `DPC-535` | same route | same meaning | immutable root Registry byte ceiling | 3 — authoritatively typed | matching Fleet Registry root entry | none | guarded operator | discard and use Registry authority |

The slice adds eight caller-derivable, seven sensitive operator-only, fourteen
authoritatively typed and twenty-two caller-required but unowned values. The
twenty-two unowned values are eleven rejected transitions viewed at Component
and/or root scope; they may share DTO vocabulary, but each projection remains
bound to its exact operation and transition.

The terminal checks at `commit_verified`, top-level membership activation and
child membership activation run after a conservative pre-effect charge has
already passed the same immutable ceiling. Their three formatted branches and
the adjacent static top-level/child root-ceiling branches are therefore
accounting contradictions. The direct-constructor ledger now assigns five
distinct protected reservation-invalid leaves instead of the ordinary
capacity-exhausted identity. That semantic correction adds five exact
candidates without changing the source-site or projection count.

Across all forty-nine classified slices, the dynamic ledger now contains 536
values: 233 caller-derivable, 50 sensitive operator-only, 198 authoritatively
typed and 55 caller-required but unowned.

## Classified Slice 50: Component Group Provisioning Formatters

The Component Group provisioning ops and workflow files contain fourteen
production `format!` sites and twenty-one dynamic values. This slice closes all
of them. The phase/cursor strings are closed helper discriminators, not public
detail: their call sites already have phase-specific exact candidates in the
direct-constructor ledgers.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-537` | `RootComponentProvisioningOps::accept`; atomic placement reservation | root Component provisioning acceptance | existing `GROUP_PROVISIONING_PLACEMENT_CAPACITY_EXHAUSTED` and `COMPONENT_PROVISIONING_ROOT_GROUP_PLACEMENT_CAPACITY_EXCEEDED` projection | rejected next total group placements | 4 — caller-required but unowned | none; rejected acceptance has no durable operation/status row and current tracked placements are not exposed | operation-scoped provisioning capacity preflight with `required_group_placements` | protected Coordinator and guarded operator | add the exact preflight owner before removing the number |
| `DPC-538` | same acceptance rejection as `DPC-537` | same route | same meaning | protected maximum group placements | 1 — caller-derivable | accepted root binding in the submitted canonical batch | none | exact Coordinator caller | discard; caller retains the frozen root limit |
| `DPC-539` | placement index already owns the requested key | same route | existing `GROUP_PROVISIONING_PLACEMENT_ALREADY_RESERVED` | submitted `ComponentGroupPlacementId` | 1 — caller-derivable | exact submitted batch placement | none | exact Coordinator caller | discard the debug rendering; request retains the ID |
| `DPC-540` | shared persisted-cursor completed-count validator | provisioning status/recovery | one of the four existing `GROUP_PROVISIONING_*_CURSOR_COUNT_INVALID` exact identities | one of four static cursor-phase labels | 4 — caller-required but unowned | current free-form `&str` helper argument only | exact registered diagnostic selected by reservation, claim, install or Registry caller | guarded operator | replace the label parameter with a closed phase discriminator/exact code |
| `DPC-541` | shared terminal-cursor canonicality validator | same route | one of the four existing `GROUP_PROVISIONING_*_TERMINAL_CURSOR_INVALID` exact identities | one of four static cursor-phase labels | 4 — caller-required but unowned | current free-form `&str` helper argument only | exact registered diagnostic selected by the four call sites | guarded operator | select the phase-specific code; add no text field |
| `DPC-542` | shared member-cursor bounds validator | same route | one of the four existing `GROUP_PROVISIONING_*_MEMBER_CURSOR_OUT_OF_RANGE` exact identities | one of four static cursor-phase labels | 4 — caller-required but unowned | current free-form `&str` helper argument only | exact registered diagnostic selected by the four call sites | guarded operator | select the phase-specific code; add no text field |
| `DPC-543` | canonical provisioning-authority Candid encoder | acceptance/receipt and restoration validation | existing `GROUP_PROVISIONING_AUTHORITY_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical authority input and exact encoder site | guarded Component provisioning recent-failure observation | protected plan/authority shape | discard dependency prose; do not commit a hash |
| `DPC-544` | `required_member_allocation` shared helper | grouped install or Registry commitment | existing `GROUP_INSTALL_RESERVATION_MISSING` or `GROUP_REGISTRY_COMMIT_RESERVATION_MISSING` | one of two static progress-phase labels | 4 — caller-required but unowned | current free-form helper label only | exact registered diagnostic selected by install/Registry call site | guarded operator | replace the string label with exact typed phase selection |
| `DPC-545` | protected Coordinator authorization | root provisioning acceptance/advance | existing `GROUP_PROVISIONING_COORDINATOR_REQUIRED` | transport caller principal | 1 — caller-derivable | IC caller | none | public to the caller | discard principal text; exact denial retains remediation |
| `DPC-546` | total root Component-instance capacity | acceptance preflight | existing `GROUP_CAPACITY_ROOT_LIMIT_EXCEEDED` | rejected occupied total after the batch | 3 — authoritatively typed | `RootComponentRegistryStatusResponse.reserved_component_instances` and `.committed_component_instances` plus submitted batch count | controller-readable Component Registry status plus exact submitted plan | guarded operator | remove interpolation; recompute from typed status/request |
| `DPC-547` | same rejection as `DPC-546` | same route | same meaning | immutable root Component-instance ceiling | 3 — authoritatively typed | matching Fleet Registry root limits | Fleet Registry/root binding inspection | guarded operator | discard and use root authority |
| `DPC-548` | planned Spec has no root admission | same route | existing `GROUP_CAPACITY_SPEC_ADMISSION_MISSING` | submitted Component Spec ID | 1 — caller-derivable | exact provisioning plan | none | exact Coordinator caller | discard; plan retains the Spec |
| `DPC-549` | per-Spec root instance-capacity rejection | same route | existing `GROUP_CAPACITY_SPEC_LIMIT_EXCEEDED` | rejected occupied count for the exact Spec | 4 — caller-required but unowned | none; no maintained status exposes per-Spec reserved plus committed counts for the rejected operation | operation-scoped provisioning capacity preflight with exact Spec and projected occupied count | protected Coordinator and guarded operator | add the typed projection before removing the number |
| `DPC-550` | same rejection as `DPC-549` | same route | same meaning | submitted Component Spec ID | 1 — caller-derivable | exact provisioning plan | none | exact Coordinator caller | discard; plan retains the Spec |
| `DPC-551` | same rejection as `DPC-549` | same route | same meaning | immutable per-root admission maximum for the Spec | 3 — authoritatively typed | matching Fleet Registry Component admission | Fleet Registry/root binding inspection | guarded operator | discard and use admission authority |
| `DPC-552` | workflow group-placement capacity preflight | same acceptance route as `DPC-537` | existing `GROUP_CAPACITY_PLACEMENT_LIMIT_EXCEEDED` | rejected required group placements | 4 — caller-required but unowned | none; root tracked-placement state has no maintained public projection before acceptance | same operation-scoped provisioning capacity preflight as `DPC-537` | protected Coordinator and guarded operator | add one shared typed owner calculated by the authoritative helper |
| `DPC-553` | same rejection as `DPC-552` | same route | same meaning | immutable root group-placement ceiling | 3 — authoritatively typed | protected Fleet Subnet Root binding limits | Fleet Registry/root binding inspection | guarded operator | discard and use root authority |
| `DPC-554` | Ready prepaid-pool capacity preflight | same route | existing `GROUP_CAPACITY_READY_POOL_INSUFFICIENT` | submitted batch Component count | 1 — caller-derivable | exact provisioning plan | none | exact Coordinator caller | discard; plan retains the count |
| `DPC-555` | same rejection as `DPC-554` | same route | same meaning | observed Ready pool count | 3 — authoritatively typed | `CanisterPoolResponse.ready` | controller-only `canic_pool_list` status | guarded operator | discard and use bounded pool status |
| `DPC-556` | exact Store-artifact closure check | same route | existing `GROUP_STORE_ARTIFACT_MISSING` or `GROUP_STORE_ARTIFACT_DUPLICATE` | observed matching Store catalog entry count | 3 — authoritatively typed | root Store bootstrap catalog | guarded root Store bootstrap status | protected Store inventory | discard count; exact missing/duplicate code selects the action |
| `DPC-557` | same rejection as `DPC-556` | same route | same meaning | planned Component role | 1 — caller-derivable | exact provisioning plan role union | none | exact Coordinator caller | discard; plan retains the role |

The slice adds seven caller-derivable, one sensitive operator-only, six
authoritatively typed and seven caller-required but unowned values. Four of the
seven unowned rows are closed progress discriminators and require exact code
selection, not new DTO data. The other three rows represent two capacity data
gaps: the rejected group-placement total appears at preflight and atomic commit,
while the per-Spec occupied count appears once. Both belong in the exact
operation-scoped provisioning preflight/status surface.

Across all fifty classified slices, the dynamic ledger now contains 557
values: 240 caller-derivable, 51 sensitive operator-only, 204 authoritatively
typed and 62 caller-required but unowned.

## Zero-Row Closure: Native Configuration Authoring

The largest remaining mechanical `format!` cluster is not a Canister public-
error frontier. The complete cluster contains 67 native/test-only formatter
sites:

| Native/test-only owner | `format!` sites |
| --- | ---: |
| `config/validation/component_spec.rs` | 42 |
| `config/validation/auth.rs` | 12 |
| `config/validation/mod.rs` | 9 |
| `config/validation/app.rs` | 3 |
| `config/schema/log.rs` | 1 |
| **Total** | **67** |

The first four files are reachable only through `config::validation`, whose
module declaration is gated by
`cfg(any(not(target_arch = "wasm32"), test))`. The `LogConfig::validate`
implementation containing the final site has the same gate. The nested App
guidance formatter is included in the 67-site count and ultimately feeds the
same native `ConfigSchemaError::ValidationError(String)` authoring path.

These sites therefore add no numbered DPC row and do not alter the 557-value
totals. They remain rich native authoring errors owned by `ConfigSchemaError`,
not runtime diagnostics. This is an explicit exclusion, not permission to ship
the prose in release Wasm: B4 must still split or relocate native validation
ownership as required by [configuration-leaves.md](configuration-leaves.md),
without inventing compact runtime codes for host authoring failures.

The mechanical closure is reproducible with:

```text
rg -n 'format!\(' crates/canic-core/src/config/validation/*.rs
rg -n 'ConfigSchemaError::ValidationError\(format!' \
  crates/canic-core/src/config/schema/log.rs
```

## Classified Slice 51: Fleet Subnet Root Inventory Counts

The Fleet Subnet Root workflow contains exactly two production `format!`
sites. Both interpolate protected inventory counts already reconstructible from
bounded controller projections; neither needs a new detail field.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-558` | `finalize_inventory`; retained non-Store asset fence | `canic_fleet_subnet_root_final_inventory` | existing `ROOT_FINAL_INVENTORY_RETAINED_ASSETS` | current pool, allocation and workload Canister count | 3 — authoritatively typed | controller-only `CanisterPoolResponse`; its mutually exclusive non-Store state counters reconstruct the exact value | none | guarded operator inventory | remove the count from the diagnostic; inspect bounded pool status and complete recycling or handoff |
| `DPC-559` | `canister_summary`; active sibling-Store cardinality invariant | `canic_fleet_subnet_root_canister_summary` | existing `ROOT_SUMMARY_STORE_CARDINALITY_INVALID` projected as `COMPONENT_REGISTRY_STATE_INVALID` | current root-owned runtime Store count | 3 — authoritatively typed | controller-only `WasmStoreOverviewResponse.stores` | none | protected root/Store inventory | discard the count; overview retains the exact inventory and the public projection stays fail-closed |

The slice adds two authoritatively typed values and no caller-derivable,
sensitive or caller-required-unowned value. The other Fleet Subnet Root
workflow constructors are static or preserve typed nested causes and were
already closed by the direct-constructor ledger.

Across all fifty-one classified slices, the dynamic ledger now contains 559
values: 240 caller-derivable, 51 sensitive operator-only, 206 authoritatively
typed and 62 caller-required but unowned.

## Classified Slice 52: Coordinator Hashing And Provisioning-Time Evidence

The earlier root-deletion helper row classified its formatted record-family
label but not the nested Candid cause in the same message. This slice corrects
that omission and closes every other production `format!` value in the two
Coordinator ops files: one retired scale-out hash cause and three operands from
one protected current-root time-ordering contradiction.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-560` | root-deletion `response_hash`; same message as `DPC-119` | Coordinator root-deletion intent, readiness, execution, completion and status | one of existing `ROOT_DELETION_*_ENCODING_FAILED` identities selected by the typed record family | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical retained root-deletion record and exact typed hash caller | operation-correlated root-deletion failure identity | protected destructive-lifecycle evidence | discard dependency prose; preserve the record and exact family-specific code before commit |
| `DPC-561` | `component_scale_out_receipt_content_hash` | Coordinator Component scale-out retirement, replay and status validation | existing `FLEET_COMPONENT_SCALE_OUT_RECEIPT_ENCODING_FAILED` | dependency-owned Candid encoder cause | 2 — sensitive operator-only | canonical retired scale-out receipt and exact encoder site | guarded Coordinator recent-failure identity | protected retirement evidence | discard dependency prose; preserve the receipt and stop retirement |
| `DPC-562` | `validate_current_root_provision_record`; time-ordering contradiction | Coordinator Component provisioning restoration/status/advance | existing `FLEET_ROOT_PROVISION_CURRENT_START_TIME_REGRESSED` or `FLEET_ROOT_PROVISION_CURRENT_OBSERVATION_TIME_REGRESSED` | previous durable observation time | 2 — sensitive operator-only | protected provisioning acceptance/receipt sequence | guarded Coordinator recent-failure identity | protected persistence evidence | discard numeric operand; exact failed predicate selects the diagnostic and retained state remains authoritative |
| `DPC-563` | same contradiction as `DPC-562` | same route | same two existing meanings | current root provisioning start time | 2 — sensitive operator-only | durable `FleetComponentProvisioningRootProvisionRecord.started_at_ns` | guarded Coordinator recent-failure identity | protected persistence evidence | discard numeric operand; preserve the current record and select the exact time-edge code |
| `DPC-564` | same contradiction as `DPC-562` | same route | same two existing meanings | current root provisioning observation time | 2 — sensitive operator-only | durable `FleetComponentProvisioningRootProvisionRecord.recorded_at_ns` | guarded Coordinator recent-failure identity | protected persistence evidence | discard numeric operand; preserve the current record and select the exact time-edge code |

The slice adds five sensitive operator-only values and no caller-derivable,
authoritatively typed or caller-required-unowned value. None of the raw codec
or timestamp operands is required on the public boundary: the exact diagnostic
selects the failed repair edge while the canonical records remain protected.

Across all fifty-two classified slices, the dynamic ledger now contains 564
values: 240 caller-derivable, 56 sensitive operator-only, 206 authoritatively
typed and 62 caller-required but unowned.

## Classified Slice 53: Blob Billing Public Adapter

The current blob-billing facade is still maintained in 0.102 even though a
later hard cut removes it from Canic. Its public mapper has eight dynamic
values across direct formatting and typed `to_string`/string-forwarding paths.
Only billing-reachable conversion variants are included; unrelated root-hash
conversion variants cannot be produced by this workflow.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-565` | billing `BoundaryConversion::BillingNatExceedsU128` | blob billing configuration | qualified `BLOB_BILLING_NAT_EXCEEDS_U128` | one of the static billing field labels | 1 — caller-derivable | exact submitted `BlobStorageBillingConfig` field selected by the conversion call | none | public to configuration caller | discard label; request plus exact code identifies the rejected field |
| `DPC-566` | `CashierDecodeError::InvalidCycleBalance` public projection | Cashier balance/top-up/status workflows | existing `BLOB_CASHIER_CYCLE_BALANCE_INVALID` projected as `BLOB_CASHIER_RESPONSE_INVALID` | static Cashier balance field label | 2 — sensitive operator-only | exact typed conversion site and upstream response | guarded numeric recent-failure identity | provider response detail | discard field prose; retain only the exact numeric leaf before projection |
| `DPC-567` | `CashierDecodeError::InvalidGatewayPrincipal` public projection | Cashier gateway synchronization | existing `BLOB_CASHIER_GATEWAY_PRINCIPAL_INVALID` projected as `BLOB_CASHIER_RESPONSE_INVALID` | invalid principal supplied by Cashier | 2 — sensitive operator-only | transient typed Cashier response | guarded numeric recent-failure identity | foreign provider identity; prohibited publicly | mask principal and record only the exact internal leaf |
| `DPC-568` | `CashierDecodeError::TooManyGatewayPrincipals` public projection | Cashier gateway synchronization | existing `BLOB_CASHIER_GATEWAY_LIMIT_EXCEEDED` projected as `BLOB_CASHIER_RESPONSE_INVALID` | rejected upstream principal count | 2 — sensitive operator-only | transient typed Cashier response | guarded numeric recent-failure identity | provider response detail | discard count; exact code identifies the bounded-response violation |
| `DPC-569` | same rejection as `DPC-568` | same route | same meaning | configured or request-supplied maximum principal count | 1 — caller-derivable | exact sync request or stored `BlobStorageBillingConfig.gateway_principal_limit` | none | guarded operator | discard; caller/config retains the ceiling |
| `DPC-570` | `CashierBalanceInternal` string forwarding | Cashier balance and project-funding workflows | qualified `BLOB_CASHIER_BALANCE_INTERNAL` | foreign Cashier internal-error message | 2 — sensitive operator-only | untrusted typed Cashier response only | guarded numeric recent-failure identity | foreign implementation detail; prohibited publicly | discard upstream text and expose only the exact safe diagnostic |
| `DPC-571` | top-up `NotAuthorized` projection | Cashier top-up and project funding | qualified `BLOB_CASHIER_TOP_UP_NOT_AUTHORIZED` | principal asserted by the Cashier error response | 2 — sensitive operator-only | untrusted typed Cashier response; it is not verified as the requested account | guarded numeric recent-failure identity | foreign principal; prohibited publicly | mask principal; do not treat it as caller-derived authority |
| `DPC-572` | top-up `InternalError` projection | Cashier top-up and project funding | qualified `BLOB_CASHIER_TOP_UP_INTERNAL` | foreign Cashier internal-error message | 2 — sensitive operator-only | untrusted typed Cashier response only | guarded numeric recent-failure identity | foreign implementation detail; prohibited publicly | discard upstream text and expose only the exact safe diagnostic |

The slice adds two caller-derivable and six sensitive operator-only values.
It adds no authoritatively typed or caller-required-unowned value. In
particular, the principal returned by `NotAuthorized` is not assumed to equal
the submitted account; trusting that foreign value would turn response prose
into authority. The four already-reconciled Cashier decode identities retain
their guarded numeric owner until the later blob hard cut retires them without
reuse. The complete current blob producer pass now qualifies the billing,
top-up, hash and lifecycle meanings without allocating a number.

Across all fifty-three classified slices, the dynamic ledger now contains 572
values: 242 caller-derivable, 62 sensitive operator-only, 206 authoritatively
typed and 62 caller-required but unowned.

## Classified Slice 54: Component Endpoint Access Predicates

Two control-plane access predicates format the transport caller into
`AccessError::Denied`. The endpoint macros project those denials onto public
Canic errors, so both values belong in this ledger even though their immediate
constructor type is not `Error` or `InternalError`.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-573` | `RootCapabilityCallerPredicate` denial | root `canic_response_capability_v1` | existing `ACCESS_ROOT_OR_ACTIVE_COMPONENT_REQUIRED` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard principal; the predicate-specific code retains both admitted identity classes |
| `DPC-574` | `ActiveComponentMemberPredicate` denial | active-Component-only role-attestation endpoints | existing `ACCESS_ACTIVE_COMPONENT_REQUIRED` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard principal; exact membership-required code owns the action |

The slice adds two caller-derivable values and nothing in the other three
classes. It does not duplicate the workflow membership rows: these are the
separate pre-handler access denials, and endpoint authentication must select
their exact codes before any workflow call.

Across all fifty-four classified slices, the dynamic ledger now contains 574
values: 244 caller-derivable, 62 sensitive operator-only, 206 authoritatively
typed and 62 caller-required but unowned.

## Classified Slice 55: Shared Access And Build-Network Predicates

The six shared caller/topology predicates each interpolate the rejected caller.
The environment predicate additionally formats its expected and observed build
networks. All are public pre-handler denials selected by the access expression.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-575` | `access::auth::predicates::is_controller` | any controller-guarded endpoint | existing `ACCESS_CONTROLLER_REQUIRED` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard caller text; exact controller-required code owns the denial |
| `DPC-576` | `is_whitelisted` negative predicate | any whitelist-guarded endpoint | existing `ACCESS_WHITELIST_REQUIRED` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard caller text; do not conflate a negative predicate with configuration lookup failure |
| `DPC-577` | `is_child` negative predicate | any direct-child-guarded endpoint | existing `ACCESS_DIRECT_CHILD_REQUIRED` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard caller text; exact child-required code owns the action |
| `DPC-578` | `is_parent` negative predicate | any parent-guarded endpoint | existing `ACCESS_PARENT_REQUIRED` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard caller text; protected parent lookup failures retain their separate dependency identity |
| `DPC-579` | `is_root` negative predicate | any root-guarded endpoint | existing `ACCESS_ROOT_REQUIRED` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard caller text; protected root lookup failures retain their separate dependency identity |
| `DPC-580` | `is_same_canister` negative predicate | any self-call-only endpoint | existing `ACCESS_SELF_REQUIRED` | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard caller text; exact self-required code owns the denial |
| `DPC-581` | `access::env::check_build_network` mismatch | endpoint using `build_network_ic` or `build_network_local` | existing `ACCESS_BUILD_NETWORK_MISMATCH` | endpoint-required build network | 1 — caller-derivable | static access predicate selected by the endpoint declaration | none | public contract | discard interpolation; endpoint contract retains the required network |
| `DPC-582` | same mismatch as `DPC-581` | same route | same meaning | observed artifact build network | 3 — authoritatively typed | operator-visible `CanicRuntimeStatus.build_network` | none | operator-only runtime identity | remove from public denial; retrieve through guarded runtime status |

The slice adds seven caller-derivable values and one authoritatively typed
value. It adds no sensitive or caller-required-unowned value. Missing build-
network authority is a separate static diagnostic, not another row.

Across all fifty-five classified slices, the dynamic ledger now contains 582
values: 251 caller-derivable, 62 sensitive operator-only, 207 authoritatively
typed and 62 caller-required but unowned.

## Classified Slice 56: Access Attestation And Delegated Token

The two first-argument access decoders each wrap their parser cause once before
placing it in `AccessError::Denied`. Each row below counts the underlying
dynamic datum once across that transparent inner/outer formatting chain; it
does not pretend the wrapper created a second fact. Typed verification errors
remain exact cause-preserving edges.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-583` | attestation subject/caller mismatch | endpoint using `attested_local_subnet` | existing `AUTH_ATTESTATION_SUBJECT_MISMATCH` | decoded attestation subject | 1 — caller-derivable | caller-supplied first-argument `SignedRoleAttestation.payload.subject` | none | public to affected caller | discard principal; submitted proof retains the subject |
| `DPC-584` | same mismatch as `DPC-583` | same route | same meaning | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard principal; transport owns caller identity |
| `DPC-585` | local-Subnet attestation verification denial | same route | exact nested authentication diagnostic and approved projection | typed verification `InternalError` currently flattened with `to_string()` | 3 — authoritatively typed | runtime auth verifier and exact proof-validation cause | transparent numeric diagnostic preservation | source sensitivity applies | remove the access prose wrapper and preserve the exact source code/projection |
| `DPC-586` | bounded role-attestation first-argument decode | same route | existing `ACCESS_ROLE_ATTESTATION_MALFORMED` | dependency-owned Candid decoder cause carried through the inner and outer formatter | 2 — sensitive operator-only | none; input bytes plus bounded decoder determine failure | none | parser detail prohibited publicly | discard dependency prose; retain quota checks and exact malformed-attestation code |
| `DPC-587` | delegated-token verification denial other than the two expiry projections | endpoint using delegated-token access | exact nested authentication diagnostic and approved projection | typed verification `InternalError` currently flattened with `to_string()` | 3 — authoritatively typed | delegated-token verifier and exact auth cause | transparent numeric diagnostic preservation | source sensitivity applies | preserve exact code; do not infer identity from rendered text |
| `DPC-588` | delegated-token subject/caller mismatch | same route | existing `AUTH_SUBJECT_CALLER_MISMATCH` | verified token subject | 1 — caller-derivable | caller-supplied first-argument `DelegatedToken.claims.subject` after verification | none | public to affected caller | discard principal; submitted token retains the subject |
| `DPC-589` | same mismatch as `DPC-588` | same route | same meaning | transport caller principal | 1 — caller-derivable | IC ingress caller | none | public to affected caller | discard principal; transport owns caller identity |
| `DPC-590` | delegated token lacks endpoint scope | same route | existing `ACCESS_REQUIRED_SCOPE_MISSING` | endpoint-required scope | 1 — caller-derivable | static access declaration and submitted token scopes | none | public contract | discard scope prose; endpoint contract plus exact code owns remediation |
| `DPC-591` | bounded delegated-token first-argument decode | same route | existing `ACCESS_DELEGATED_TOKEN_MALFORMED` | dependency-owned Candid decoder cause carried through the inner and outer formatter | 2 — sensitive operator-only | none; input bytes plus bounded decoder determine failure | none | parser detail prohibited publicly | discard dependency prose; retain quota checks and exact malformed-token code |

The slice adds five caller-derivable, two sensitive operator-only and two
authoritatively typed values. It adds no caller-required-unowned value. The
decoder causes are deliberately discarded; exposing them would turn bounded
parser internals into a public protocol. Expiry branches continue to reuse
their existing exact auth identities and contain no additional dynamic value.

Across all fifty-six classified slices, the dynamic ledger now contains 591
values: 256 caller-derivable, 64 sensitive operator-only, 209 authoritatively
typed and 62 caller-required but unowned.

## Classified Slice 57: Access Dependency And Service Guards

The remaining access adapters contain one free-form dependency helper, one
typed static Fleet-service declaration parser and one protected runtime cause.
They require different hard-cut treatment even though all currently become
`AccessError::Denied(String)`.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-592` | `access::auth::dependency_unavailable` | whitelist, parent, root and delegated-token-config predicates | exact source configuration/environment diagnostic projected as existing `ACCESS_DEPENDENCY_UNAVAILABLE` | one of four static dependency-detail labels | 4 — caller-required but unowned | current free-form `&str` helper argument only | typed source-error preservation at each call site | protected dependency state | delete helper label; preserve the exact configuration/environment cause before applying the safe access projection |
| `DPC-593` | `access::deployment::require_service_authority`; static guard parse failure | application endpoint declaring a Fleet-service Authority guard | existing `ACCESS_SERVICE_GUARD_INVALID` projected as `ACCESS_CONFIGURATION_INVALID` | typed `ComponentDeploymentIdParseError` rendered with declaration kind/bounds | 1 — caller-derivable | exact endpoint service declaration and maintained bounded `FleetServiceId` contract | none | application developer contract | discard parser prose; declaration plus exact code owns repair and reinstall action |
| `DPC-594` | same service guard after successful parsing | same route | existing `ACCESS_SERVICE_AUTHORITY_REQUIRED` or exact protected runtime dependency diagnostic | typed Component-runtime `InternalError` currently flattened with `to_string()` | 3 — authoritatively typed | protected Component deployment/Directory runtime authority | transparent numeric diagnostic preservation | protected service authority | preserve exact runtime cause and approved projection; do not replace corruption/unavailability with an ordinary negative predicate |

The slice adds one caller-derivable, one authoritatively typed and one caller-
required-unowned closed discriminator. The category-4 row requires typed cause
preservation, not a response field or generic detail DTO.

Across all fifty-seven classified slices, the dynamic ledger now contains 594
values: 257 caller-derivable, 64 sensitive operator-only, 210 authoritatively
typed and 63 caller-required but unowned.

## Classified Slice 58: Blob Root-Hash And Lifecycle Facade

The non-billing blob facade projects `BlobStorageConversionError` directly to
public invalid-input errors. Only the root-hash variants are reachable on this
path. Its lifecycle mapper has two unit variants with static prose and
therefore forms a zero-row sub-closure inside this slice.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-595` | `BlobRootHashError::InvalidLength` | blob hash canonicalization and text-hash lifecycle endpoints | qualified `BLOB_ROOT_HASH_LENGTH_INVALID` | submitted root-hash text length | 1 — caller-derivable | exact endpoint input | none | public to affected caller | discard actual length; request retains it |
| `DPC-596` | same rejection as `DPC-595` | same route | same meaning | required canonical root-hash text length | 1 — caller-derivable | maintained `BLOB_ROOT_HASH_TEXT_LENGTH` contract | none | public contract | discard constant; exact code and protocol own the bound |
| `DPC-597` | `BlobRootHashError::InvalidHexCharacter` | same text-hash routes | qualified `BLOB_ROOT_HASH_HEX_INVALID` | invalid byte index in submitted text | 1 — caller-derivable | exact endpoint input | none | public to affected caller | discard index; caller can inspect submitted value |
| `DPC-598` | same rejection as `DPC-597` | same route | same meaning | invalid byte from submitted text | 1 — caller-derivable | exact endpoint input | none | public to affected caller | discard byte value; request retains it |
| `DPC-599` | `BlobStorageConversionError::InvalidRootHashByteLength` | blob byte-hash canonicalization and gateway deletion confirmation | qualified `BLOB_ROOT_HASH_BYTE_LENGTH_INVALID` | submitted byte-vector length | 1 — caller-derivable | exact endpoint input | none | public to affected caller | discard actual length; request retains it |
| `DPC-600` | same rejection as `DPC-599` | same route | same meaning | required root-hash byte length | 1 — caller-derivable | maintained `BLOB_ROOT_HASH_BYTE_LENGTH` contract | none | public contract | discard constant; exact code and protocol own the bound |

The slice adds six caller-derivable values and nothing in the other classes.
`BlobRootHashError::{Empty, InvalidPrefix}` and
`BlobStorageLifecycleError::{BlobNotLive, BlobPendingDeletion}` add no dynamic
row; their static exact meanings are qualified by the current blob family.
`blobs_are_live` deliberately maps malformed byte entries to `false`
rather than a public error and is outside this message inventory.

Across all fifty-eight classified slices, the dynamic ledger now contains 600
values: 263 caller-derivable, 64 sensitive operator-only, 210 authoritatively
typed and 63 caller-required but unowned.

## Classified Slice 59: Auth API Terminal Mapper

`AuthApi::map_auth_error` is one shared formatter, but its fifteen production
call sites terminate distinct typed owners. The rows below close the wrapper
edge per source route. They do not authorize an aggregate auth code: every row
must preserve the exact nested identity already enumerated by the auth,
configuration and runtime semantic ledgers.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-601` | delegated-token configuration lookup in `require_delegated_token_issuer_enabled` | issuer token endpoints | exact nested configuration diagnostic | typed `ConfigOps` error flattened by the shared mapper | 3 — authoritatively typed | compiled runtime configuration ops | transparent numeric preservation | protected configuration | preserve exact configuration code/projection; delete mapper prose |
| `DPC-602` | current-Canister configuration lookup in the same gate | issuer token endpoints | exact nested configuration diagnostic | typed `ConfigOps` error flattened by the shared mapper | 3 — authoritatively typed | compiled current-role configuration ops | transparent numeric preservation | protected role configuration | preserve exact source code/projection; do not create an issuer-gate wrapper code |
| `DPC-603` | `verify_token_material` | delegated-token verification helper | exact nested delegated-token diagnostic | typed verifier `InternalError` flattened by the shared mapper | 3 — authoritatively typed | delegated-token verifier and authenticated request | transparent numeric preservation | source projection applies | preserve the exact verifier identity already classified by the auth slices |
| `DPC-604` | `prepare_component_role_attestation_root` | root role-attestation prepare | exact nested prepare/replay/signature diagnostic | typed runtime-auth workflow error | 3 — authoritatively typed | role-attestation prepare operation and replay authority | transparent numeric preservation | operation and proof sensitivity follow source | preserve exact source identity and any operation-correlated numeric secondary evidence |
| `DPC-605` | `get_role_attestation_root` | root role-attestation retrieval | exact nested receipt/proof diagnostic | typed runtime-auth workflow error | 3 — authoritatively typed | retained attestation receipt and root-signature proof owner | transparent numeric preservation | proof sensitivity follows source | preserve exact source identity; delete aggregate mapper text |
| `DPC-606` | `verify_role_attestation` | local role-attestation verification | exact nested attestation diagnostic | typed verifier workflow error | 3 — authoritatively typed | role-attestation verifier | transparent numeric preservation | source projection applies | preserve exact proof/time/audience/epoch code |
| `DPC-607` | `verify_local_subnet_role_attestation` | local-Subnet role-attestation verification | exact nested attestation and receiver-Subnet diagnostic | typed verifier workflow error | 3 — authoritatively typed | verifier plus live receiver Subnet authority | transparent numeric preservation | protected topology/proof detail | preserve exact source code; never collapse a Subnet mismatch into generic auth failure |
| `DPC-608` | `upsert_root_issuer_policy_root` | root issuer-policy administration | exact nested policy/storage diagnostic | typed root-issuer workflow error | 3 — authoritatively typed | root issuer-policy workflow and stable policy owner | transparent numeric preservation | controller/root policy | preserve exact validation or storage identity |
| `DPC-609` | `upsert_root_issuer_renewal_template_root` | root renewal-template administration | exact nested renewal-policy/storage diagnostic | typed renewal workflow error | 3 — authoritatively typed | root renewal template and timer policy owner | transparent numeric preservation | controller/root policy | preserve exact template or persistence identity |
| `DPC-610` | `get_or_create_chain_key_delegation_proof_root` | issuer-authenticated root proof creation | exact nested chain-key batch/issuer diagnostic | typed chain-key workflow error | 3 — authoritatively typed | durable chain-key batch and exact issuer request | transparent numeric preservation | cryptographic and issuer sensitivity follow source | preserve exact batch, signing, Registry or caller identity |
| `DPC-611` | `provision_chain_key_delegation_proof_for_issuer_root` | root issuer provisioning | exact nested request/transport/response/install diagnostic | typed issuer-provisioning workflow error | 3 — authoritatively typed | exact issuer provisioning operation and proof-install state | transparent numeric preservation | transport/proof sensitivity follows source | preserve primary code and operation-correlated secondary numeric evidence |
| `DPC-612` | `prepare_delegated_token` | issuer token preparation | exact nested prepare/replay/signature diagnostic | typed token-prepare workflow error | 3 — authoritatively typed | token prepare operation, replay receipt and issuer proof | transparent numeric preservation | token/proof sensitivity follows source | preserve exact source identity; never map by class/origin alone |
| `DPC-613` | `get_delegated_token` | issuer token retrieval | exact nested issuer-proof receipt diagnostic | typed auth-ops error | 3 — authoritatively typed | retained token claims/proof receipt | transparent numeric preservation | token proof state | preserve exact lookup, caller or proof identity |
| `DPC-614` | `install_active_delegation_proof` | issuer active-proof installation | exact nested proof-validation/persistence diagnostic | typed auth-ops error | 3 — authoritatively typed | validated root proof and active-proof stable owner | transparent numeric preservation | protected proof material | preserve exact validation/storage code before commitment |
| `DPC-615` | `active_delegation_proof_status` | issuer active-proof status | exact nested active-proof status diagnostic | typed auth-ops error | 3 — authoritatively typed | active-proof lifecycle state | transparent numeric preservation | guarded issuer status | preserve exact state/access code; do not format a generic internal failure |

The slice adds fifteen authoritatively typed wrapper values and nothing in the
other classes. These rows close the shared interpolation only because every
source family is already expanded in the semantic auth/configuration ledgers;
they are not a substitute for that variant-level evidence. B4 removes the
class/origin switch and carries the exact numeric source identity through the
facade.

Across all fifty-nine classified slices, the dynamic ledger now contains 615
values: 263 caller-derivable, 64 sensitive operator-only, 225 authoritatively
typed and 63 caller-required but unowned.

## Classified Slice 60: Generic Public Projection And Replay Cleanup Context

The final generic `InternalError` projection formats its source exactly once;
the borrowed and owned `From` implementations both delegate to that same
expression and therefore do not create two dynamic facts. The shared replay
cleanup helpers add two independently typed secondary failures. Static helper
context labels add no rows.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-616` | `api::error::internal_error_to_public`; class/origin fallback used by both `From<InternalError>` implementations | every endpoint returning a converted `InternalError` without an attached public error | exact source diagnostic or its approved safe projection | source `InternalError` rendered with `to_string()` | 3 — authoritatively typed | exact typed conversion owner or direct-constructor identity already enumerated by the semantic ledgers | transparent numeric preservation in the code-first `InternalError` | sensitivity follows the exact source projection | remove the class/origin guess and message rendering; carry the already-selected registered identity through both `From` edges |
| `DPC-617` | `workflow::replay::abort_reserved_receipt_after_failure`; failed pre-effect reservation abort | delegated-token and role-attestation preparation plus manual ICP refill replay cleanup | exact nested replay-receipt storage diagnostic retained as secondary evidence | typed `ReplayReceiptStoreError` appended to the primary failure | 3 — authoritatively typed | exact replay receipt and operation-bound receipt-storage validator | operation-correlated replay status or structured numeric runtime diagnostic | protected replay-state detail follows the nested source projection | preserve the primary result; retain the secondary numeric identity against the same operation and never append its prose |
| `DPC-618` | `workflow::replay::mark_recovery_required_after_failure`; failed post-effect recovery-marker commit | ICP refill, root provisioning, root recycling and non-root cycles replay recovery | exact nested replay-receipt storage diagnostic retained as secondary evidence | typed `ReplayReceiptStoreError` appended to the primary failure | 3 — authoritatively typed | exact replay receipt, recovery reason and operation-bound receipt-storage validator | operation-correlated replay status or structured numeric runtime diagnostic | protected post-effect recovery state follows the nested source projection | preserve the primary result; retain the secondary numeric identity against the same operation and never append its prose |

The slice adds three authoritatively typed values and nothing in the other
classes. `DPC-616` closes one interpolation site, not one row per endpoint and
not one row per delegating `From` implementation. `DPC-617` and `DPC-618`
close the shared replay-abort frontier left open by Slice 27; they do not
replace the command-specific recovery rows already owned by each workflow.

Across all sixty classified slices, the dynamic ledger now contains 618
values: 263 caller-derivable, 64 sensitive operator-only, 228 authoritatively
typed and 63 caller-required but unowned.

## Classified Slice 61: Local Intent Capacity And Placement Acknowledgement

The public local-intent API formats four capacity operands. The root RPC
placement-receipt acknowledgement formats its request operation ID in three
distinct rejection branches. These seven values are the public subset of the
runtime-intent/RPC execution pass.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-619` | `LocalIntentWorkflow::begin`; rejected reservation | `LocalIntentApi::begin` | qualified `INTENT_LOCAL_CAPACITY_EXCEEDED` | submitted intent resource key | 1 — caller-derivable | exact `BeginLocalIntentInput.resource_key` | none | public to affected caller | discard key; request retains it |
| `DPC-620` | same rejection as `DPC-619` | same route | same meaning | current reserved quantity for the resource | 4 — caller-required but unowned | private canonical `IntentStoreOps::totals` observation only | request-scoped `LocalIntentCapacityStatusResponse.reserved_quantity`, bound to the exact resource key | public only to the resource's application caller; guarded if exposed generically | add the narrow capacity preflight/status owner before removing the value |
| `DPC-621` | same rejection as `DPC-619` | same route | same meaning | submitted reservation quantity | 1 — caller-derivable | exact `BeginLocalIntentInput.quantity` | none | public to affected caller | discard; request retains it |
| `DPC-622` | same rejection as `DPC-619` | same route | same meaning | submitted reservation limit | 1 — caller-derivable | exact `BeginLocalIntentInput.reservation_limit` | none | public to affected caller | discard; request retains it |
| `DPC-623` | placement receipt actor mismatch | root `AcknowledgePlacementReceipt` capability | qualified `INTENT_PLACEMENT_ACK_ACTOR_MISMATCH` | submitted operation ID | 1 — caller-derivable | exact acknowledgement request | none | public to affected caller | discard operation text; request and exact diagnostic own remediation |
| `DPC-624` | placement receipt is not committed | same route | qualified `INTENT_PLACEMENT_ACK_NOT_COMMITTED` | submitted operation ID | 1 — caller-derivable | exact acknowledgement request | none | public to affected caller | discard operation text; retry the same request only after terminal commitment |
| `DPC-625` | receipt does not represent a placement effect | same route | qualified `INTENT_PLACEMENT_ACK_EFFECT_MISMATCH` | submitted operation ID | 1 — caller-derivable | exact acknowledgement request | none | public to affected caller | discard operation text; preserve the exact receipt and fail closed |

The slice adds six caller-derivable values and one caller-required-unowned
value. The missing owner is specific capacity evidence, not a generic intent
detail field or global last error.

`IntentCleanupWorkflow`'s formatted due-intent identity, abort context and
deadline occur only on lifecycle/timer paths. They feed the guarded runtime
recent-failure/lifecycle diagnostic owner, not a public `Error.message`, and
therefore add no row here. Their numeric runtime-log migration remains B4 work.

Across all sixty-one classified slices, the dynamic ledger now contains 625
values: 269 caller-derivable, 64 sensitive operator-only, 228 authoritatively
typed and 64 caller-required but unowned.

## Classified Slice 62: RPC Authority, Cycles Funding And Replay Scalars

This slice expands every live scalar field in `RpcWorkflowError` except the two
free-form replay codec strings. Those strings remain source-specific and cannot
be classified as one aggregate datum. The maintained routes contribute twenty
scalar values.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-626` | `RpcWorkflowError::ChildNotFound` | Component-child targeting and root/non-root cycles funding | provisional `RPC_CHILD_NOT_FOUND` | requested or calling child principal | 1 — caller-derivable | exact request target or transport caller | none | public to affected caller | discard principal; request/transport retains it |
| `DPC-627` | `RpcWorkflowError::NotChildOfCaller` | Component-child recycling and cycles funding | provisional `RPC_CHILD_NOT_DIRECT` | rejected child principal | 1 — caller-derivable | exact request target or transport caller | none | public to affected caller | discard child principal; exact code owns the authority failure |
| `DPC-628` | same rejection as `DPC-627` | same routes | same meaning | expected direct parent/caller principal | 1 — caller-derivable | transport receiver/caller and submitted route | none | public to affected caller | discard principal; never infer parentage from the diagnostic |
| `DPC-629` | `RpcWorkflowError::InsufficientFundingCycles` | root/non-root `RequestCycles` capability | provisional `RPC_CYCLES_FUNDING_BALANCE_INSUFFICIENT` | policy-approved transfer amount after clamping | 4 — caller-required but unowned | transient funding decision only | request-scoped `CyclesFundingPreflightResponse.approved_cycles` bound to exact caller/role/request | exact registered child and guarded operator | add the narrow preflight owner; do not expose it as generic error detail |
| `DPC-630` | same rejection as `DPC-629` | same route | same meaning | live root/parent Canister cycle balance | 2 — sensitive operator-only | controller-authenticated IC Canister status at observation time | none | funding authority balance; prohibited to arbitrary children | remove the raw balance from the child-facing diagnostic; operator inspects authoritative Canister status |
| `DPC-631` | `RpcWorkflowError::FundingRequestExceedsChildBudget` | same cycles-funding route | provisional `RPC_CYCLES_FUNDING_CHILD_BUDGET_EXHAUSTED` | submitted cycle request | 1 — caller-derivable | exact `CyclesRequest.cycles` | none | public to affected caller | discard; request retains it |
| `DPC-632` | same rejection as `DPC-631` | same route | same meaning | remaining admitted child budget | 4 — caller-required but unowned | private `CyclesFundingLedgerOps` snapshot and compiled limits only | request-scoped `CyclesFundingPreflightResponse.remaining_child_budget` | exact registered child and guarded operator | add typed preflight/status ownership before removing the value |
| `DPC-633` | same rejection as `DPC-631` | same route | same meaning | admitted maximum per-child budget | 4 — caller-required but unowned | compiled role-specific funding limits only | request-scoped `CyclesFundingPreflightResponse.max_per_child` | exact registered child and guarded operator | add typed preflight/status ownership before removing the value |
| `DPC-634` | `RpcWorkflowError::FundingCooldownActive` | same cycles-funding route | provisional `RPC_CYCLES_FUNDING_COOLDOWN_ACTIVE` | computed retry delay in seconds | 4 — caller-required but unowned | transient policy decision only | request-scoped `CyclesFundingPreflightResponse.retry_after_secs` | exact registered child and guarded operator | add typed retry timing; do not force callers to parse prose |
| `DPC-635` | `RpcWorkflowError::FundingOperationInProgress` | same cycles-funding route | provisional `RPC_CYCLES_FUNDING_OPERATION_IN_PROGRESS` | calling child principal | 1 — caller-derivable | transport caller and exact replay actor | none | public to affected caller | discard principal; retry the exact operation after reconciliation |
| `DPC-636` | `RpcWorkflowError::InvalidReplayTtl` | every replay-protected root capability | provisional split `RPC_REPLAY_TTL_ZERO` or `RPC_REPLAY_TTL_EXCEEDED` | submitted replay TTL | 1 — caller-derivable | exact request metadata | none | public to affected caller | split the current combined variant by predicate; discard the submitted value |
| `DPC-637` | same rejection as `DPC-636` | same routes | same two meanings | maintained replay TTL ceiling | 1 — caller-derivable | static root capability replay contract | none | public contract | discard the constant; exact zero/exceeded identity owns remediation |
| `DPC-638` | `RpcWorkflowError::ReplayTtlOverflow` | every replay-protected root capability | qualified `RPC_REPLAY_TIME_RANGE_UNSUPPORTED` | live receiver time in nanoseconds | 2 — sensitive operator-only | transient IC time observation before reservation | none | internal timing operand; not required publicly | discard the exact clock value; no reservation has occurred and a saturated receiver clock cannot represent any positive expiry |
| `DPC-639` | same rejection as `DPC-638` | same routes | same meaning | submitted replay TTL | 1 — caller-derivable | exact request metadata | none | public to affected caller | discard; request and exact overflow code own repair |
| `DPC-640` | `RpcWorkflowError::ReplayExpired` | replay-protected root capability | qualified `REPLAY_RECEIPT_EXPIRED` | static capability label | 1 — caller-derivable | invoked capability and command kind | none | public contract | discard label; begin a newly admitted operation |
| `DPC-641` | `RpcWorkflowError::ReplayConflict` | same routes | qualified `REPLAY_PAYLOAD_MISMATCH` | static capability label | 1 — caller-derivable | invoked capability and exact request ID/payload | none | public contract | discard label; replay only the original payload or use a new ID |
| `DPC-642` | `RpcWorkflowError::ReplayDuplicateSame` | same routes | qualified `REPLAY_OPERATION_IN_PROGRESS` | static capability label | 1 — caller-derivable | invoked capability and exact replay receipt | none | public contract | discard label; retry the same request later |
| `DPC-643` | `RpcWorkflowError::ReplayStoreCapacityReached` | fresh root replay reservation | provisional `RPC_REPLAY_GLOBAL_CAPACITY_EXHAUSTED` | maintained global root-replay entry maximum | 1 — caller-derivable | static root replay contract | none | public contract | discard maximum; exact capacity code owns bounded retry |
| `DPC-644` | `RpcWorkflowError::ReplayStoreCallerCapacityReached` | same reservation route | qualified `REPLAY_PENDING_ACTOR_CAPACITY` | transport caller principal | 1 — caller-derivable | exact replay actor | none | public to affected caller | discard caller principal |
| `DPC-645` | same rejection as `DPC-644` | same route | same meaning | maintained per-caller replay entry maximum | 1 — caller-derivable | static root replay contract | none | public contract | discard maximum; await terminal operations before retry |

The slice adds fourteen caller-derivable values, two sensitive operator-only
values and four caller-required-unowned values. The four unowned values form
one request-scoped cycles-funding preflight contract; they do not justify four
last-error fields. The raw Canister balance is deliberately not part of that
child-facing DTO.

`RpcWorkflowError::{CanisterRoleNotFound, ParentNotFound}` have no production
constructor. `MissingReplayMetadata` is matched without rendering and becomes
the static operation-ID-required error. `CyclesFundingDisabled` is static.
These branches add no dynamic row. Slice 63 expands and closes
`ReplayEncodeFailed(String)` and `ReplayDecodeFailed(String)` at their typed
sources.

Across all sixty-two classified slices, the dynamic ledger now contains 645
values: 283 caller-derivable, 66 sensitive operator-only, 228 authoritatively
typed and 68 caller-required but unowned.

## Classified Slice 63: Replay Codec And Root Recovery Context

The two free-form replay codec buckets expand to distinct source values. This
slice closes the root replay encoder/decoder, the shared committed-response
schema helper and the two root-specific secondary cleanup sites. Existing auth
and ICP-refill response codec rows are not counted again.

| ID | Source field or branch | Public route | Diagnostic meaning | Dynamic value | Class | Authoritative owner | Proposed owner | Sensitivity | Hard-cut disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DPC-646` | `encode_root_replay_response`; noncompact root response | replay-protected root capability response staging | qualified `REPLAY_RESPONSE_ENCODE_FAILED` | dependency-owned Candid encoder cause | 3 — authoritatively typed | exact typed response and operation-bound replay receipt | operation-correlated replay status | guarded implementation detail | discard dependency prose; select the exact response-encode code before staging |
| `DPC-647` | `decode_root_replay_response`; noncompact root response | cached/recovered root capability response | qualified `REPLAY_RESPONSE_DECODE_FAILED` | dependency-owned Candid decoder cause | 3 — authoritatively typed | retained terminal response bytes and exact root replay decoder | operation-correlated replay status | guarded retained-byte detail | preserve terminal bytes and exact decoder code; discard dependency prose |
| `DPC-648` | root replay guard/store `ReceiptDecodeFailed` forwarding | every replay-protected root capability | qualified `REPLAY_RECEIPT_DECODE_FAILED` | typed receipt decoder cause flattened to `String` | 3 — authoritatively typed | exact replay receipt slot and canonical receipt decoder | operation-correlated replay status | protected stable-byte detail | preserve malformed bytes and exact receipt-decode code; never forward prose |
| `DPC-649` | compact root response unknown-variant branch | cached/recovered cycles response | qualified `RPC_REPLAY_COMPACT_VARIANT_INVALID` | unknown retained variant tag | 2 — sensitive operator-only | retained terminal response bytes | operation-correlated replay status | protected retained-byte detail | discard tag value; preserve bytes and fail closed with the exact variant code |
| `DPC-650` | compact root response truncation helper | same route | qualified `RPC_REPLAY_COMPACT_CYCLES_VALUE_TRUNCATED` | static compact field label | 1 — caller-derivable | maintained compact response schema | none | public protocol contract | remove helper label; the exact cycles-value decoder site selects the code |
| `DPC-651` | `committed_response_bytes`; response schema version absent | delegated-token, role-attestation and ICP-refill response recovery | qualified `REPLAY_RESPONSE_SCHEMA_VERSION_MISSING` | static response-family label | 1 — caller-derivable | retained receipt command kind and invoked recovery route | none | visible to exact operation owner | discard label; receipt/route identifies the response family |
| `DPC-652` | same helper; unsupported response schema | same routes | qualified `REPLAY_RESPONSE_SCHEMA_VERSION_UNSUPPORTED` | static response-family label | 1 — caller-derivable | retained receipt command kind and invoked recovery route | none | visible to exact operation owner | discard label; exact schema code owns remediation |
| `DPC-653` | same rejection as `DPC-652` | same routes | same meaning | observed retained response schema version | 3 — authoritatively typed | exact terminal replay receipt | operation-correlated replay status | guarded receipt evidence | remove from public message; preserve in the retained receipt/status |
| `DPC-654` | same helper; response bytes absent | same routes | qualified `REPLAY_TERMINAL_RESPONSE_MISSING` | static response-family label | 1 — caller-derivable | retained receipt command kind and invoked recovery route | none | visible to exact operation owner | discard label; exact command and committed receipt state own the failure without collapsing it into missing staged response |
| `DPC-655` | root staged-response recovery; recovery-marker commit also fails | root replay recovery after cost settlement | exact nested replay-receipt storage diagnostic retained as secondary evidence | typed recovery-marker `InternalError` appended to the primary error | 3 — authoritatively typed | exact replay receipt, recovery reason and response-commit operation | operation-correlated replay status or structured numeric runtime diagnostic | protected post-effect recovery evidence | preserve the primary failure and record the secondary numeric identity separately |
| `DPC-656` | `abort_replay_after_failure`; reservation cleanup also fails | root capability pre-effect failure cleanup | exact nested replay-receipt storage diagnostic retained as secondary evidence | typed cleanup `InternalError` appended to the primary error | 3 — authoritatively typed | exact reserved replay receipt and cleanup operation | operation-correlated replay status or structured numeric runtime diagnostic | protected replay state | preserve the primary failure and record the secondary numeric identity separately |

The slice adds four caller-derivable values, one sensitive operator-only value
and six authoritatively typed values. It adds no unowned value. Static compact
missing-tag/trailing-byte branches contain no interpolation and remain exact
allocation work. The three auth/refill dependency decoder causes remain owned
by `DPC-233`, `DPC-235` and `DPC-321`; this slice adds only the schema helper's
separate label/version operands.

Across all sixty-three classified slices, the dynamic ledger now contains 656
values: 287 caller-derivable, 67 sensitive operator-only, 234 authoritatively
typed and 68 caller-required but unowned.
