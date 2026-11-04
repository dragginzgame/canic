# Stable-memory layout

Canic composes one `ic-storage` runtime per canister. Storage owns the physical
backing, allocation directory, exclusive leases, codecs and collections. Canic
owns its permanent store keys, namespace grants, lifecycle ordering and domain
records. The current Storage family requires clean reinstall across releases;
same-release restart, snapshots and interruption recovery remain supported.

## Host namespace grants

Components return complete checked `MemoryRequest` lists without opening stores.
The host combines those requests and selects `AllocationGrants` before bootstrap.
The framework contributes `canic-core` / `canic.core.` and
`canic-control-plane` / `canic.control_plane.`. Application namespaces must be
disjoint; a request cannot grant its own owner, choose an allocation ID or open
another runtime over the backing.

```rust
canic::memory::memory_allocation_grants!(
    provider = application_memory_grants,
    authorities = [("app", "app.")],
);

fn application_memory() -> canic::memory::MemoryBootstrap {
    let mut input = canic::memory::framework();
    input.grants = application_memory_grants().expect("host namespace grants");
    input.requests.extend(application_requests().expect("application requests"));
    input
}
```

Pass that composition to `canic::start!(memory = application_memory(), ...)`.
Canic supplies `StorageConfig::default()` to the single owner. Storage enforces
its allocation, extent, physical-page and Wire budgets. Growth uses Wasm pages
and returns typed `RuntimeGrowError`, including capacity, arithmetic and backing
refusal. An ordinary refusal preserves the prior allocation for retry.
Allocation IDs are opaque `u64` values selected by Storage; callers use checked
`StableKey` inputs and inspect `allocation_id` only for diagnostics.

Declaring a store does not allocate its payload pages. Lazy stores open through
`canic::memory::open(key)` after committed bootstrap. Their exclusive leases
retain backing custody even if the runtime handle is dropped. Store APIs own
capacity reservations; observation must not open a second lease.

## Lifecycle and composed admission

Lifecycle restores Canic's invariants synchronously, then runs the selected
synchronous participant, then schedules deferred application work. A participant
uses `canic::memory::runtime::shared_runtime()` to verify its requirements and
open its admitted keys. It does not become a second lifecycle or storage owner.

An optional `MemoryBootstrapAdmission::new(identity, prepare)` supplies one
composed pre-commit callback. Its identity is a `[u8; 32]` digest covering the
callback's semantics and configuration. Cold bootstrap invokes admission before
publishing authority; warm verification requires the same declarations, grants
and policy and does not rerun admission. Consumer errors retain their typed
source through `MemoryRegistryError::Admission`.

Native consuming applications must perform the same composition on every thread
that touches Canic storage, before database initialization or store access:

```rust
canic::memory::bootstrap(application_memory()).expect("composed host bootstrap");
```

A process-global `Once` does not initialize another thread's runtime. Canic's own
unit-test hook is not installed for applications compiling it as a dependency.
Actual database composition belongs to the consuming application's qualification.

## Records and collections

Canic-owned Serde records and checkpoints use native Wire with the `CSW\x01`
storage marker. The maintained record ceiling is 64 MiB encoded bytes, 1 GiB
conservative traversal allocation allowance, 64 Mi visited nodes and depth 128;
narrower record bounds still apply. Fixed-width keys retain their explicit
encodings. Principal map keys use Canic's `PrincipalKeyRecord` and preserve the
canonical raw principal bytes.

`Storable` encoding and decoding return typed codec errors. Storage collection
construction, access and iteration are fallible. `StableCell::load` also requires
an explicit payload ceiling and checks integrity before decoding. Metrics
provider rows use `ic-metrics-storage`; the upstream adapter owns their schema
and validation. Host diagnostics report a reservation for that upstream map.

Checkpoint and authority hash domains retain their maintained Wire contracts.
Required external IC protocols, dedicated Fleet activation commitments and Host
release-build records keep their separately specified encodings.

## Allocation reporting and dependency identity

The protected allocation report reads the committed Storage directory without
opening stores, writing, growing memory or advancing its generation. It reports
retained keys, opaque IDs and logical capacities, including unopened zero-page
allocations. Its summary separates data pages, directory metadata, unassigned
physical pages and the one protected root page. Capacity does not establish
record payload occupancy or reclaimable space.

Role validation rejects multiple reachable Storage identities with
`role_contract_multiple_storage_runtimes`. It uses the selected normal Wasm
graph, including active features and renamed dependencies. Build/dev edges,
native-only dependencies and proc-macro subtrees do not define that runtime.
The adoption guard also requires one Platform Runtime and Runtime Guard in
each selected root and private-consumer graph. Matching package identities are
necessary; lifecycle and authority composition still need behavioral proof.

## Retained store boundaries

Independent authority, funding, authentication, replay and telemetry owners keep
their separate lifetimes. Ordered expiry, pending, reverse-lookup and terminal
eligibility indexes preserve bounded reads. The Component provisioning map
retains its checked record ceiling. Receipt metadata owns application count and
replay retention; cleanup reserves its own eligibility capacity before admission.
Shard rows own activation, while routing still checks the current allocation
operation identity.

Template manifests, staged chunk sets, compact chunk references and payload
vectors remain separate. Manifest approval and chunk cleanup have different
lifetimes. GC counts resolvable slots from references and vector length without
loading chunk bodies. Current-family tests must qualify these decisions; older
allocator measurements do not describe this Storage graph.

## Maintained allocation inventory

Role and feature selection determine the declared stores and those opened by
lifecycle. The upstream directory is outside application allocations and has no
application-selected ID.

| Stable key | Representation |
| --- | --- |
| `canic.control_plane.template.manifests.v1` | B-tree |
| `canic.control_plane.template.chunk_sets.v1` | B-tree |
| `canic.control_plane.template.chunk_refs.v1` | B-tree |
| `canic.control_plane.template.chunk_payloads.v1` | StableVec |
| `canic.control_plane.wasm_store.gc_state.v1` | Cell |
| `canic.control_plane.fleet_coordinator.registry.v1` | Cell |
| `canic.control_plane.root.wasm_store.state.v1` | Cell |
| `canic.control_plane.root.fleet_registry_mirror.v1` | Cell |
| `canic.control_plane.root.component.registry_state.v1` | Cell |
| `canic.control_plane.root.component.allocations.v1` | B-tree |
| `canic.control_plane.root.component.registry_entries.v1` | B-tree |
| `canic.control_plane.root.component.principal_index.v1` | B-tree |
| `canic.control_plane.root.component.subtree_removal_history.v1` | B-tree |
| `canic.control_plane.root.component.draining.v1` | B-tree |
| `canic.control_plane.root.canister_inventory.assets.v1` | B-tree |
| `canic.control_plane.root.canister_pool.state.v1` | Cell |
| `canic.control_plane.root.canister_pool.handoff_receipts.v1` | B-tree |
| `canic.control_plane.root.component_provisioning.operations.v1` | B-tree, checked record encoding / small overflow pages |
| `canic.control_plane.root.component_provisioning.placements.v1` | B-tree |
| `canic.control_plane.root.component_provisioning.state.v1` | Cell |
| `canic.core.runtime.canister_children.v1` | B-tree |
| `canic.core.runtime.bindings.v1` | Cell |
| `canic.core.fleet.state.v1` | Cell |
| `canic.core.fleet.activation.v1` | Bounded optional cell |
| `canic.core.auth.local_application_authorization.state.v1` | Cell (generation and protected binding) |
| `canic.core.auth.local_application_authorization.sessions.v1` | Bounded vector (4 KiB rows; canonical IC Auth session payload only) |
| `canic.core.auth.local_application_authorization.replays.v1` | Bounded vector (512-byte consumed-proof tombstones) |
| `canic.core.replay.receipts.v1` | B-tree |
| `canic.core.cycles.tracker.v1` | B-tree |
| `canic.core.cycles.topup_events.v1` | B-tree |
| `canic.core.cycles.funding_ledger.v1` | B-tree |
| `canic.core.cycles.icp_refill_records.v1` | B-tree |
| `canic.core.intent.meta.v1` | Cell, including application receipt count |
| `canic.core.intent.records.v1` | B-tree |
| `canic.core.intent.totals.v1` | B-tree, including explicit quota-window retention |
| `canic.core.intent.pending.v1` | B-tree |
| `canic.core.intent.receipt_backed_records.v1` | B-tree, including application retention |
| `canic.core.intent.expiry_index.v1` | B-tree |
| `canic.core.caller_authority.header.v1` | Bounded optional cell |
| `canic.core.application_receipt.eligibility.v1` | B-tree plus reservation |
| `canic.core.placement.acknowledgement_index.v1` | B-tree |
| `canic.core.placement.scaling_registry.v1` | B-tree |
| `canic.core.placement.index_registry.v1` | B-tree |
| `canic.core.sharding.registry.v1` | B-tree, including activation |
| `canic.core.sharding.assignments.v1` | B-tree |
| `canic.core.caller_authority.rows.v1` | B-tree: sources, Component fences and original receipts |
| `canic.core.authority_restore.fence.v1` | Bounded optional cell |
| `canic.core.async_job_recovery.v1` | Cell |
| `canic.core.fleet_admission.projection.v1` | Bounded optional cell |
| `canic.control_plane.fleet_coordinator.funding.v1` | Cell |
| `canic.control_plane.root.funding.v1` | Cell |
| `canic.control_plane.fleet_admission.v1` | Bounded optional cell |
| `canic.control_plane.root.admission.v1` | Cell |
| `canic.core.auth.delegated_token_issuer.state.v1` | Cell |
| `canic.core.auth.root_delegation.state.v1` | Cell |
| `canic.control_plane.fixture_store.v1` | B-tree |

## Qualification

Focused tests cover explicit namespace admission, uncommitted-read refusal, warm
identity checks, cold reopen, exclusive custody, growth refusal, typed codec
rejection, metrics failure retention and receipt capacity. Installed Canic
lifecycle and dedicated/embedded Blob evidence must identify the exact selected
graph. A native substrate result does not replace installed or downstream
application acceptance.

## Storage 0.65.1 template measurement

The native comparison writes and reads eight chunks through each layout. Counts
are calls to the backing interface, not IC instructions or wall time. Split pages
include both independent backing directories; direct pages include its directory.

| Bytes per chunk | Split physical pages | Direct physical pages | Split reads | Direct reads | Split writes | Direct writes |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 4,096 | 136 | 5 | 32 | 32 | 71 | 34 |
| 1,048,576 | 136 | 260 | 32 | 32 | 71 | 48 |

The previous overflow-read penalty no longer applies. The current split layout
retains lower physical capacity for full chunks; a direct map packs tiny chunks
more efficiently. A different packing policy needs its own workload evidence.


## Continue From Here

- [Read protected and public observations](public-observability.md)
- [Browse runtime features](README.md)
- [Back to the main README](../../../README.md)
