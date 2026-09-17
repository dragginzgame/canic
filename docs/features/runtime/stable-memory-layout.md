# Stable-memory layout

Canic uses published ic-memory 0.14.1 and a single MemoryManager per canister.
The default allocation bucket is **16 Wasm pages (1 MiB)**. A bucket belongs to
one virtual memory; it cannot be shared between IDs. The manager's own metadata
page is separate. This setting reduces the minimum physical allocation of a
small populated store from 8 MiB to 1 MiB.

## Capacity and selection

A bucket is allocation granularity, not a per-store size limit. Stores grow by
acquiring more buckets. The manager has 32,768 bucket slots shared across all
virtual memories in a canister. Consequently smaller buckets also reduce its
addressable managed capacity:

| Build setting | Bucket bytes | Manager bucket-table capacity |
| --- | ---: | ---: |
| `CANIC_MEMORY_BUCKET_PAGES=1` | 64 KiB | 2 GiB |
| `CANIC_MEMORY_BUCKET_PAGES=8` | 512 KiB | 16 GiB |
| `CANIC_MEMORY_BUCKET_PAGES=16` (default) | 1 MiB | 32 GiB |
| `CANIC_MEMORY_BUCKET_PAGES=32` | 2 MiB | 64 GiB |
| `CANIC_MEMORY_BUCKET_PAGES=64` | 4 MiB | 128 GiB |
| `CANIC_MEMORY_BUCKET_PAGES=128` | 8 MiB | 256 GiB |

These are manager addressing ceilings, not a promise of available IC memory,
cycles or application capacity. The setting accepts any nonzero `u16` number
of pages. Invalid values fail the build. Smaller selections reduce allocation
slack and manager capacity together; the qualified default is 16 pages. Select a
larger value when building an
application expected to outgrow 32 GiB, including its database, indexes,
framework allocations and headroom. The environment variable must accompany
the actual artifact build, rather than only a later install command.

Cargo tracks the variable through the core build script; Canic's complete-build
reuse fingerprint also includes the build environment. The selected size is
compiled into each artifact and supplied to ic-memory before stable stores open.
`canic::memory::configured_bucket_pages()` exposes that compiled selection.
There is no runtime resize, automatic switch or per-ID bucket override. Changing
this setting belongs to a fresh installation at a release boundary. Same-release
reopen and interruption recovery retain the same manager geometry.

The 1 MiB default balances the many small framework stores against manager
capacity. It is not claimed to be globally optimal for every application's data
or workload. Application-owned stores and IcyDB share the same manager geometry;
Canic does not change their schemas, IDs or ownership.

Consumers composed into the same canister must use the same ic-memory package
identity. IcyDB is a test-only dependency of this repository; its separate
version in the workspace lockfile does not enter deployed Canic roles. Its
composed integration is qualified explicitly once dependencies align, using
`make test-pocketic-case CASE=icydb_lifecycle_composition`. Canic's production
Wasm dependency guard continues to require a single memory runtime.

The 0.14 update retains fixed-ID declarations and bucket selection. It adds
upstream limits to ledger recovery (including 16 MiB logical payloads, depth 32
and bounded histories); out-of-contract state rejects. Canic does not enable
key-only placement, automatic migration or the optional historical-admission
hook. Release transitions retain Canic's reinstall-only policy.

## Representation and consolidation

- IDs 33 (activation), 59 (restore fence), 61 (admission projection), and 64
  (Coordinator admission) each use their own optional bounded cell. Empty state
  remains explicit. The cell checks the complete encoding before writing;
  declared record limits remain enforced. Independent authority records stay
  separate.
- ID 27 retains a multi-record provisioning map. Its B-tree allocator uses small
  overflow pages, while its encoder and decoder enforce the existing 8,650,000
  byte record ceiling. The large logical record limit no longer sizes every
  B-tree node for the worst case.
- Receipt ID 45 owns its application replay deadline. ID 41 maintains the exact
  application receipt count for constant-time capacity reporting. Insertion,
  replacement and reclamation maintain that count; reconciliation validates it.
  Cleanup eligibility remains in ordered ID 48, including reserved capacity and
  exact binding/revision validation. The separate deadline allocation is removed.
- Shard ID 52 owns its activation flag. Registration records activation in the
  same row; selection still checks activation and routing authority. Assignment
  lookup remains in ID 53. The separate active-set allocation and facades are
  removed.

This is a hard cut of the maintained stable layout. Release transitions are
reinstall-only. There are no old-layout readers or migration paths. Canonical
snapshots and same-release recovery use the maintained records.

## Template decisions

Keep template manifests and staged chunk-set metadata separate. Approval can be
removed while staged chunks remain. Combining them would make manifest listing
and approval updates read/write the chunk hash list as well. With one 40-chunk
release, the standalone manifest encodes to 191 bytes; the candidate aggregate
encodes to 2,946 bytes. A saved small bucket does not justify coupling these
paths.

Keep the compact chunk-reference map and payload vector separate. An isolated
measurement with the pinned ic-stable-structures 0.7.2 compares eight chunks and
counts calls to the backing `Memory` interface:

| Bytes per chunk | Split virtual pages | Direct-map virtual pages | Split reads | Direct-map reads | Split writes | Direct-map writes |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 4,096 | 114 | 1 | 201 | 465 | 220 | 527 |
| 1,048,576 | 130 | 132 | 201 | 74,706 | 220 | 87,140 |

The direct map saves space for tiny chunks but uses more space and approximately
372 times as many backing reads for full chunks. Counts are storage-call
measurements, not IC instruction or wall-time ratios. The vector retains its
1,048,580-byte slot stride; short nonfinal slots and cleared slots can leave
unused space. That remaining tradeoff is explicit. A packed bulk-data allocator
would be a separate design, rather than forcing bulk payloads through B-tree
overflow chains.

GC chunk counting uses the reference map and the payload vector's length. It
does not load chunk bodies merely to count resolvable slots. This avoids up to
1 MiB of transient payload buffering per visited chunk and payload-volume reads
during accounting; it does not reduce allocated stable pages. Empty payloads
remain valid slots, and out-of-range references remain excluded. Reopen and
bounded metadata-read checks cover this path.

Further physical reductions would need a separate measured change: packing
short bulk chunks, avoiding unnecessary store initialization where lifecycle
invariants permit it, or qualifying smaller buckets for known-capacity roles.
None is assumed safe from empty-store measurements alone. The default remains
1 MiB so this follow-up does not reduce the shared manager capacity again.

## Inventory and retained boundaries

The following inventory covers every maintained Canic-owned allocation. Role
and feature selection determine which entries a canister actually opens. Bucket
selection applies to the whole manager; individual record bounds do not reserve
that amount in cells. ic-memory owns its ledger at ID 0. The two removed
allocations are 47 (deadline metadata now in 45) and 54 (activation now in 52).
Their IDs are not renumbered into unrelated owners.

| ID | Stable key | Representation |
| ---: | --- | --- |
| 10 | `canic.control_plane.template.manifests.v1` | B-tree |
| 11 | `canic.control_plane.template.chunk_sets.v1` | B-tree |
| 12 | `canic.control_plane.template.chunk_refs.v1` | B-tree |
| 13 | `canic.control_plane.template.chunk_payloads.v1` | StableVec |
| 14 | `canic.control_plane.wasm_store.gc_state.v1` | Cell |
| 15 | `canic.control_plane.fleet_coordinator.registry.v1` | Cell |
| 16 | `canic.control_plane.root.wasm_store.state.v1` | Cell |
| 17 | `canic.control_plane.root.fleet_registry_mirror.v1` | Cell |
| 18 | `canic.control_plane.root.component.registry_state.v1` | Cell |
| 19 | `canic.control_plane.root.component.allocations.v1` | B-tree |
| 20 | `canic.control_plane.root.component.registry_entries.v1` | B-tree |
| 21 | `canic.control_plane.root.component.principal_index.v1` | B-tree |
| 22 | `canic.control_plane.root.component.subtree_removal_history.v1` | B-tree |
| 23 | `canic.control_plane.root.component.draining.v1` | B-tree |
| 24 | `canic.control_plane.root.canister_inventory.assets.v1` | B-tree |
| 25 | `canic.control_plane.root.canister_pool.state.v1` | Cell |
| 26 | `canic.control_plane.root.canister_pool.handoff_receipts.v1` | B-tree |
| 27 | `canic.control_plane.root.component_provisioning.operations.v1` | B-tree, checked record encoding / small overflow pages |
| 28 | `canic.control_plane.root.component_provisioning.placements.v1` | B-tree |
| 29 | `canic.control_plane.root.component_provisioning.state.v1` | Cell |
| 30 | `canic.core.runtime.canister_children.v1` | B-tree |
| 31 | `canic.core.runtime.bindings.v1` | Cell |
| 32 | `canic.core.fleet.state.v1` | Cell |
| 33 | `canic.core.fleet.activation.v1` | Bounded optional cell |
| 34 | `canic.core.auth.local_application_authorization.state.v1` | Cell |
| 35 | `canic.core.replay.receipts.v1` | B-tree |
| 36 | `canic.core.cycles.tracker.v1` | B-tree |
| 37 | `canic.core.cycles.topup_events.v1` | B-tree |
| 38 | `canic.core.cycles.funding_ledger.v1` | B-tree |
| 39 | `canic.core.cycles.icp_refill_records.v1` | B-tree |
| 40 | `canic.core.log.entries.v1` | B-tree |
| 41 | `canic.core.intent.meta.v1` | Cell, including application receipt count |
| 42 | `canic.core.intent.records.v1` | B-tree |
| 43 | `canic.core.intent.totals.v1` | B-tree |
| 44 | `canic.core.intent.pending.v1` | B-tree |
| 45 | `canic.core.intent.receipt_backed_records.v1` | B-tree, including application retention |
| 46 | `canic.core.intent.expiry_index.v1` | B-tree |
| 48 | `canic.core.application_receipt.eligibility.v1` | B-tree plus reservation |
| 49 | `canic.core.placement.acknowledgement_index.v1` | B-tree |
| 50 | `canic.core.placement.scaling_registry.v1` | B-tree |
| 51 | `canic.core.placement.index_registry.v1` | B-tree |
| 52 | `canic.core.sharding.registry.v1` | B-tree, including activation |
| 53 | `canic.core.sharding.assignments.v1` | B-tree |
| 55 | `canic.core.blob_storage.roots.v1` | B-tree |
| 56 | `canic.core.blob_storage.pending_deletions.v1` | B-tree |
| 57 | `canic.core.blob_storage.gateway_principals.v1` | B-tree |
| 58 | `canic.core.blob_storage.billing.v1` | Cell |
| 59 | `canic.core.authority_restore.fence.v1` | Bounded optional cell |
| 60 | `canic.core.async_job_recovery.v1` | Cell |
| 61 | `canic.core.fleet_admission.projection.v1` | Bounded optional cell |
| 62 | `canic.control_plane.fleet_coordinator.funding.v1` | Cell |
| 63 | `canic.control_plane.root.funding.v1` | Cell |
| 64 | `canic.control_plane.fleet_admission.v1` | Bounded optional cell |
| 65 | `canic.control_plane.root.admission.v1` | Cell |
| 66 | `canic.core.auth.delegated_token_issuer.state.v1` | Cell |
| 67 | `canic.core.auth.root_delegation.state.v1` | Cell |
| 68 | `canic.control_plane.fixture_store.v1` | B-tree |

Retain the independent funding, registry, authority, authentication, replay and
telemetry owners. Live assets and completed handoff receipts have different
lifetimes. Ordered expiry, pending membership, reverse lookups and terminal
eligibility avoid unbounded scans; they remain separate indexes. Fixture Store
ID 68 is already a related typed namespace and remains separate from executable
template authority. A smaller bucket reduces the minimum footprint of these
small owners without coupling their mutation or retention rules.

## Qualification

The maximum Coordinator admission fixture encodes to 2,055,610 bytes. Its
bounded cell uses 2,097,152 virtual bytes and 2,162,688 physical bytes including
the isolated manager metadata: 2 MiB plus one 64 KiB page. The four empty
singleton types each use one virtual page. Their declared maxima remain record
ceilings rather than preallocated B-tree node capacity.

The targeted qualification covers bounded-cell reopen and rejection before
writes, full-width count encoding, receipt retry/reconciliation/reclamation,
reserved cleanup space at the 1,000-record admission limit, shard activation and
registry reopen, manifests, memory ownership and descriptor inventories.
Template storage-call measurements above are native isolated-memory results.
The disposable Fleet Root reports 47,251,456 bytes (45.0625 MiB) of allocated
stable memory, including 2,949,120 bytes of virtual extent. Virtual extent is
not a payload-occupancy measurement. Protected observation preserves stable
bytes and controller authority. Receipt conformance survives same-release
restart, exact retries, settlement and reclamation. All seven Canic/IcyDB
lifecycle/import cases pass, including held replies and consumer reinstall.

The three governed runtime targets took 359s, 51s and 191s including builds and
runner setup. All 1,643 source/lock inputs stayed unchanged. These results
qualify the fresh-layout default; they do not measure the current Toko Miner
estate or establish worst-case instruction cost for every maximum-sized record.
Large provisioning records still traverse overflow pages when loaded.
