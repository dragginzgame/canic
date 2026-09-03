# 0.110 B1 Destroyed-State And Reconstruction Inventory

Date: 2026-09-03
State: allocation catalog complete; hard-cut preconditions and maintainer
acceptance open
Design owner: [0.110 Fleet runtime contraction](../../../design/0.110-fleet-runtime-contraction/0.110-design.md)
Source baseline: immutable `v0.110.5` at
`50f40171d6177c3d1e490b1fdb5f6163323b2cd5`

## Verdict

Canic's current allocation catalog is complete enough to name every
Canic-owned stable-memory domain that an authorized cross-release reinstall
destroys. It does not support the broader statement that all such state is
reconstructed.

Three different outcomes exist:

1. authoritative state rebuilt from current config, release artifacts,
   protected init payloads or exact live observations;
2. operation and history state deliberately reset only after the predecessor
   has no unresolved effect; and
3. dynamic application, placement, blob and credential state that Canic cannot
   reconstruct and that a consumer must start fresh, restore under separate
   application authority or accept as destroyed outside this release line.

No positional codec cut is yet selected. A B3 cut remains blocked by unresolved
Canic-owned paid-effect or release-boundary preconditions, not by the absence of
a particular consumer's discard/reseed procedure. The pre-1.0 contract is
reinstall-only and retains no compatibility decoder or import lane.

## Release Boundary

This inventory applies only between releases. Same-release interruption,
retry, idempotency, backup and restore continue to depend on the current
stable records and may not use this document to waive recovery.

The cross-release transaction must:

- stop or terminally account for every predecessor operation before discarding
  its replay, intent, fence, funding or GC evidence;
- observe every controlled canister balance and bind every retained,
  transferred, debited or explicitly discarded cycle;
- compile one new desired-state plan from the current config and current
  release-set authority rather than reopen predecessor plan/journal bytes;
- use fresh operation identities and current protected init arguments; and
- re-prove controllers, module hashes, topology and terminal convergence from
  live observations after reinstall.

The host plan has only `Install` and `Reinstall` modes; it has no `Upgrade`
variant. Existing module-bearing canisters therefore select reinstall in the
current Ensure policy. However, `CurrentReleaseSetManifest` does not yet carry
the design's planned machine-readable reinstall-only transition mode. The
behavioral restriction exists, while the structured release-metadata and
negative no-effect qualification remain open B5 evidence.

## Authoritative Reconstruction Inputs

| Authority | What it can rebuild | What it cannot rebuild |
| --- | --- | --- |
| validated current `canic.toml` and compiled role authority | role/capability graph, Component topology, auth trust configuration, metrics profile and protected deployment shape | live identities, dynamic assignments, sessions, receipts or application data |
| current release-set manifest and artifact digests | exact Root, Coordinator, Store and application Wasms plus Candid/protocol identities | predecessor stable records or application content |
| new Fleet Ensure desired state and journal | one reviewed current operation, protected init arguments and effect ordering | predecessor in-flight operation continuation |
| exact live ICP observations | existing Principals, controllers, modules, lifecycle and cycle balances that remain observable | lost semantic history or unobservable predecessor effects |
| Coordinator/Root protected protocols | fresh Registry, Root mirror, Component bindings, admission projection and Store publication | arbitrary predecessor topology or deleted historical memberships |
| downstream/application owner input | optional application data, blob registrations and business-level assignment reseed | Canic provides no default cross-release importer |

Fresh Root authority is carried by `FleetSubnetRootInitArgs`: the exact Fleet
binding, initial release set, expected module hash, sibling Store authority,
install identity and reviewed pool imports. Fresh Store authority is carried by
`FleetSubnetWasmStoreInitArgs`. Each managed Component or child receives a
`CanisterInitPayload` containing its release-build identity, exact
Root/Component binding, protected deployment and optional admission
projection. Coordinator init receives the configured App, Registry authority,
admission policy, Component deployment configuration and Root funding policy.

Those inputs rebuild current authority. They do not migrate predecessor state.

## Complete Canic Allocation Ledger

The following table covers all 39 `StateAllocationKey` values and all active
Canic memory IDs 10 through 65. Grouping does not merge ownership: each named
key remains an independent allocation.

| Allocation keys | IDs | Selected owners | Cross-release disposition | Required reconstruction or precondition |
| --- | --- | --- | --- | --- |
| `TemplateManifests`, `TemplateChunkSets`, `TemplateChunkRefs`, `TemplateChunkPayloads` | 10-13 | Root, Store | rebuild | Republish the exact current release artifacts and manifests from the immutable release set; no predecessor Store catalog is imported. |
| `WasmStoreGcState` | 14 | Store | reset | Predecessor GC must be terminal/Normal or its deletion intent must be explicitly abandoned after catalog/content accounting; start new GC only after current catalog publication. |
| `FleetCoordinatorRegistry` | 15 | Coordinator | rebuild | Initialize a fresh Coordinator Registry from current Fleet authority and rejoin/activate current Roots through the new operation. Historical Registry versions and removed entries are discarded. |
| `RootWasmStoreState` | 16 | Root | rebuild | Recreate the sibling Store authority and publication state from protected init, adoption/bootstrap and current artifact publication. |
| `RootFleetRegistryMirror` | 17 | Root | rebuild | Synchronize the current Coordinator-authored Registry after fresh Root initialization; no predecessor mirror is trusted. |
| `RootComponentRegistry` | 18-23 | Root | rebuild current topology; discard history | Re-provision or re-observe the current desired Component tree. Creation, install, drain, removal and directory history is not reconstructed. Reusing any live Principal requires exact controller/module/balance observation in the new plan. |
| `RootCanisterPool` | 24-26 | Root | rebuild from desired and live state | Re-import only explicitly reviewed controlled empty canisters. Lost creation/handoff receipts cannot justify reuse; controllers, lifecycle and cycles must be proved again. |
| `RootComponentProvisioning` | 27-29 | Root | reset and re-run | No predecessor provisioning cursor or intent continues across the release. The old operation must be terminal or abandoned without repeating an unresolved effect; a new plan uses new operation IDs. |
| `CoreRuntimeChildren`, `CoreRuntimeBindings`, `CoreFleetState`, `CoreFleetActivation` | 30-33 | every Runtime role | rebuild | Protected init and current Root/Coordinator protocols recreate identity, binding, activation and child projection. Dynamic predecessor child history is discarded. |
| `CoreAuthState` | 34 | auth-capable roles and Root | partial rebuild; credentials reset | Rebuild only configured trust anchors and protected authority. Sessions, replay entries, active delegation proofs, issuers, renewal cursors and chain-key batches are not reconstructed; clients/Roots must establish fresh current credentials. |
| `CoreReplayReceipts` | 35 | every Runtime role | reset | Every predecessor paid or externally visible effect must be terminally reconciled before reset. Old operation IDs are never retried in the new release. |
| `CoreCycles` | 36-38 | every Runtime role | reset history; conserve live value | Cycle tracker/top-up/funding history is discarded, but live balances are not. The new plan must observe and conserve each controlled balance and must prevent a reset funding window from authorizing unaccounted duplicate spend. |
| `CoreCyclesIcpRefillRecords` | 39 | Root | reset | All ledger/CMC refill intents and responses must be terminally reconciled. No predecessor ICP debit is repeated from a fresh empty journal. |
| `CoreRuntimeLog` | 40 | every Runtime role | discard | Runtime logs are diagnostic history, not reconstruction authority. Retain externally before reinstall only if operationally required. |
| `CoreIntent` | 41-46 | every Runtime role | reset | Pending, receipt-backed and expiry-indexed intents may disappear only after their effects are reconciled or explicitly abandoned. Fresh work receives fresh intent identities. |
| `CoreApplicationReceipts` | 47-48 | every Runtime role | reset | Application replay/eligibility history is not portable. The application owner must prevent predecessor requests from being resubmitted as new-release work. |
| `CorePlacementAcknowledgement` | 49 | every Runtime role | reset | Old placement acknowledgements do not authorize current allocations; current Root/parent authority must issue fresh work. |
| `PlacementScalingRegistry` | 50 | scaling roles | discard/reseed | Config rebuilds scaling policy, not dynamic worker assignments or business placement. Existing assignments require owner reseed or explicit data loss acceptance. |
| `PlacementIndexRegistry` | 51 | index roles | discard/reseed | Config rebuilds index policy, not key-to-child assignments. The application owner must rebuild the index or accept its loss. |
| `ShardingRegistry`, `ShardingAssignments`, `ShardingActiveSet` | 52-54 | sharding roles | discard/reseed | Config rebuilds shard-pool policy, not entity assignments or the active dynamic set. The application owner must reseed them. |
| `BlobStorageRoots`, `BlobStoragePendingDeletions`, `BlobStorageGatewayPrincipals`, `BlobStorageBilling` | 55-58 | blob-storage feature consumers | discard/reseed | Canic config does not recreate live blob registrations, pending deletion authority, gateway membership or billing state. A fresh release starts empty; application backup or reseed is independently consumer-owned. Paid or cycle-bearing work must still be reconciled before destruction. |
| `CoreAuthorityRestoreFence` | 59 | Root, Coordinator | reset only from terminal open state | A sealed or restoring predecessor cannot cross the cut. Finish or safely abandon restoration and prove timers/authority are open before reinstall. |
| `CoreAsyncJobRecovery` | 60 | every Runtime role | reset | Reconcile each durable async job and its possible external effect first; new-release timers/jobs do not resume old recovery payloads. |
| `CoreFleetAdmissionProjection` | 61 | Fleet-admission targets | rebuild | The new Coordinator/Root admission protocol prepares, activates and opens a fresh projection from current policy. Predecessor transition receipts are discarded. |
| `FleetCoordinatorFunding` | 62 | Coordinator | rebuild policy; reset ledger | Init restores current policy, not grants, reservations or the active spend window. Reinstall must disable funding, wait out/bind the predecessor window, or explicitly account for prior grants so reset cannot double-spend. |
| `RootFunding` | 63 | Root | rebuild policy; reset journal | Re-establish current Coordinator-authored policy only after predecessor requests and rotations are terminal. Historical spend is not reconstructed. |
| `FleetCoordinatorAdmission` | 64 | Coordinator | rebuild | Initialize the current admission policy and distribute a new transition. Prior participant receipts and transition history are discarded. |
| `RootAdmission` | 65 | Root | rebuild | Recreate Root-local policy/participants through the current Coordinator transition. Prior reservations and retained results are not imported. |

## Role-Level Destruction Sets

The exact catalog projection for the canonical roles is:

- every managed role and Store with `Runtime` selects the common Runtime set:
  `CoreRuntimeChildren`, `CoreRuntimeBindings`, `CoreFleetState`,
  `CoreFleetActivation`, `CoreReplayReceipts`, `CoreCycles`, `CoreRuntimeLog`,
  `CoreIntent`, `CoreApplicationReceipts`, `CoreAsyncJobRecovery` and
  `CorePlacementAcknowledgement`;
- `index_hub` additionally selects `PlacementIndexRegistry`;
- `user_hub` additionally selects the three Sharding allocations;
- `scale_hub` additionally selects `PlacementScalingRegistry`;
- `test` additionally selects `CoreFleetAdmissionProjection`;
- `user_shard` additionally selects `CoreAuthState`;
- Root adds `CoreAuthState`, `CoreCyclesIcpRefillRecords`,
  `CoreAuthorityRestoreFence`, the four Template allocations and every
  Root-prefixed allocation;
- Store adds the four Template allocations and `WasmStoreGcState`; and
- Coordinator selects only `FleetCoordinatorRegistry`,
  `FleetCoordinatorFunding`, `FleetCoordinatorAdmission` and
  `CoreAuthorityRestoreFence`.

`app`, `index_child` and `scale_replica` add no allocation beyond the common
Runtime set in the canonical fixture. Feature-selected blob storage is absent
from the canonical eleven-role roster but present in the repository-owned
`blob_storage_probe`, so its Canic-owned hard-cut and same-release recovery
boundaries remain part of qualification.

## State Outside The Canic Allocation Catalog

Application-owned stable memory is not described by `StateAllocationKey` and
is still destroyed by reinstall. Canic neither exports nor imports it across a
release. The downstream owner must explicitly choose fresh initialization,
restore from an independently governed application backup, or accept loss.
Canic backup/restore guarantees remain same-release behavior and are not a
cross-release migration lane.

The predecessor host Fleet Ensure plan, journal and `state.json` are likewise
not imported into a successor release. They remain historical/operator
evidence only. The current release-set manifest, current desired input and a
new operation journal become authority. Reusing a controlled Principal or
balance discovered live does not convert the predecessor journal into a
migration contract.

## Blocking Findings And Acceptance Items

The catalog does not yet justify an unconditional codec hard cut:

- dynamic index, shard and scale assignments are not reconstructable from
  Canic config alone and must be explicitly classified as destroyed by the
  release transaction;
- reset Coordinator/Root funding ledgers need an exact anti-double-spend
  boundary across their active windows;
- all in-flight paid/replay-protected operations need a terminal-or-abandoned
  precondition before their journals are destroyed;
- the release-set manifest still lacks the planned machine-readable
  reinstall-only transition field; and
- representative instruction/table allowances and the negative ordinary-
  upgrade no-effect proof remain unaccepted.

These are Canic release-boundary evidence gaps, not arguments for compatibility
decoders. The pre-1.0 response is to stop the affected cut until the fresh
reinstall transaction proves its owned preconditions—not to add a predecessor
import path or wait for a particular consumer's migration policy.

## B1 Disposition

The allocation inventory is complete and all non-reconstructable domains are
named. Consumer application reseed is explicitly outside the Canic gate. B1
still requires accepted instruction/table allowances and maintainer acceptance
of the hard-cut preconditions. B2 and B3 remain blocked.
