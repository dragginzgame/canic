# Canic 0.102 Direct Internal-Constructor Frontier

Date: 2026-08-13

## Status

This B1 frontier corrects the scope of the provisional semantic ledger. The
previous family passes reconcile **685 collision-free proposed identities**
within the typed and explicitly expanded surfaces they examined. They do not
yet prove that every current direct `InternalError` producer maps to one of
those identities.

No numeric allocation may be frozen until every site in this frontier is linked
to one existing exact meaning, adds a separately justified exact meaning, or is
proved to be a transparent wrapper, native/test-only path or sediment that is
deleted rather than numbered.

## Method

The initial census used immutable source baseline `v0.101.53`. The current
0.102.0 candidate reconciliation is pinned to the control-plane/core source
content at `0750c309104b111fa6f5a1b3355c04fcb38faf71`; later working-tree Rust
changes are confined to `canic-cli`, outside this frontier. A direct diff from
the baseline confirms that the intervening control-plane/core changes add or
remove no `InternalError` constructor reference. Any later change under the
scanned roots requires a fresh count before allocation.

For that exact source content, the scan:

1. enumerates Rust source under `canic-core/src` and
   `canic-control-plane/src`;
2. excludes external `tests.rs` and `tests/` files;
3. removes inline `#[cfg(test)] mod tests { ... }` tails; and
4. counts production references matching
   `InternalError::[A-Za-z_]+`.

This is a conservative constructor-reference frontier, not a proposed code
count. Several references may share one actionable meaning, and wrapper or
conversion definitions may receive no code. Conversely, formatted values in
one broad constructor may conceal several typed meanings.

## Current Result

| Owner region | Production references |
| --- | ---: |
| `canic-control-plane/src/ops` | 1,269 |
| `canic-control-plane/src/workflow` | 606 |
| `canic-control-plane` other | 1 |
| `canic-core/src/ops` | 104 |
| `canic-core/src/workflow` | 227 |
| `canic-core/src/api` | 1 |
| **Total** | **2,208 in 101 files** |

Current site-level progress:

| Reconciled slice | References classified | Remaining frontier |
| --- | ---: | ---: |
| Top-level Component allocation persistence | 55 | 2,153 |
| Direct-child reservation/install persistence | 83 | 2,070 |
| Child commitment and activation persistence | 73 | 1,997 |
| Child create/install/activation workflow | 61 | 1,936 |
| Top-level Component commitment/activation workflow | 25 | 1,911 |
| Top-level Component draining/quiescence/recycling workflow | 51 | 1,860 |
| Subtree-removal orchestration workflow | 26 | 1,834 |
| Subtree stop/recycling/protected-authority workflow | 27 | 1,807 |
| Root draining/final inventory/logical removal persistence | 73 | 1,734 |
| Store reclamation/publication-binding finalization persistence | 45 | 1,689 |
| Store deletion/root-deletion preparation persistence | 61 | 1,628 |
| Final/initial root-inventory persistence | 35 | 1,593 |
| Root activation/initial-inventory convergence workflow | 15 | 1,578 |
| Root draining/final inventory/logical removal workflow | 36 | 1,542 |
| Store reclamation/binding-finalization workflow | 7 | 1,535 |
| Store deletion/root-deletion readiness workflow | 19 | 1,516 |
| Sibling Store adoption workflow | 7 | 1,509 |
| Component Directory paging/protected-member workflow | 20 | 1,489 |
| Component Directory convergence/runtime-status workflow | 51 | 1,438 |
| Component peer/protected-allocation workflow | 23 | 1,415 |
| Component Registry preparation/allocation/create-install workflow | 55 | 1,360 |
| Component Directory paging and protected-status persistence | 15 | 1,345 |
| Top-level Component draining and removal transition persistence | 50 | 1,295 |
| Top-level Component draining and removal protected validation | 58 | 1,237 |
| Subtree fence and advancement persistence | 33 | 1,204 |
| Subtree stop and deletion persistence | 31 | 1,173 |
| Subtree membership, Directory and leaf-finalization persistence | 31 | 1,142 |
| Protected subtree authority and history validators | 33 | 1,109 |
| Top-level commitment and activation persistence | 61 | 1,048 |
| Fleet-service Directory refresh persistence | 21 | 1,027 |
| Remaining Registry, accounting and hash adapters | 42 | 985 |
| Component Group provisioning acceptance through activation persistence | 64 | 921 |
| Component Group protected request and stable phase validation | 36 | 885 |
| Component Group publication and provisioned-result integrity | 35 | 850 |
| Component Group cursor, member authority and commit mapping | 42 | 808 |
| Component Group provisioning orchestration workflow | 56 | 752 |
| Coordinator genesis, join, provisioning and lifecycle public transitions | 67 | 685 |
| Coordinator test adapters, retired receipt encoding and status lookup | 5 | 680 |
| Coordinator publication and progress classification | 18 | 662 |
| Coordinator scale-out synchronization and runtime response integrity | 16 | 646 |
| Coordinator Directory publication and root acceptance evidence | 24 | 622 |
| Coordinator root-provision response/current-progress validation | 10 | 612 |
| Coordinator Registry admission, snapshot and root-lifecycle gates | 14 | 598 |
| Coordinator root-deletion transition and durable-history validation | 21 | 577 |
| Coordinator deployment-ledger public plan boundaries | 2 | 575 |
| Coordinator workflow admission, fences and root-error propagation | 12 | 563 |
| Canister pool inventory initialization, reset and claims | 11 | 552 |
| Canister pool autonomous creation intent through rollover | 32 | 520 |
| Canister pool handoff, Store deletion, configuration and helpers | 26 | 494 |
| Canister pool maintenance, import, handoff and refill workflow | 17 | 477 |
| Root Store bootstrap manifest, artifact and live-catalog workflow | 23 | 454 |
| Root bootstrap Subnet discovery and sibling Store adoption state | 8 | 446 |

These dispositions are recorded in the Component Registry persistence and
workflow ledgers and the Fleet Subnet Root workflow ledger linked from the
design status.
Some remaining sites may reuse already qualified meanings, but they remain open
until the concrete constructor is linked in the site manifest.

### Expanded Semantic Frontier

The Coordinator family reveals one shared `receipt_invariant` constructor with
292 production call sites: 235 in the parent source, 10 in `root_deletion` and
47 in `deployment_ledger`. Replacing that one direct site with its call-site
meanings expands the effective disposition frontier by 291:

| Frontier | Total | Classified | Open |
| --- | ---: | ---: | ---: |
| Mechanical direct constructors | 2,208 | 1,762 | 446 |
| Coordinator receipt-invariant calls replacing one adapter site | 292 | 292 | 0 |
| **Effective semantic disposition frontier** | **2,499** | **2,053** | **446** |

The adapter is explicitly a no-code funnel. Semantic progress must use the
2,499-site frontier until all 292 calls have exact dispositions; counting its
single constructor as one diagnostic would be a false closure.

The largest two files alone contain 1,154 references:

- `canic-control-plane/src/ops/component_registry/mod.rs`: 800; and
- `canic-control-plane/src/workflow/component_registry/mod.rs`: 354.

Component Registry and Component provisioning ops/workflows are now closed.
Fleet Coordinator, Canister pool, bootstrap and Wasm Store paths come next.
They contain external-effect, interruption and authority failures and cannot
inherit broad codes merely because an inner typed-family audit is complete.

## Reconciliation Order

1. Component Registry ops and workflow, split by allocation, installation,
   activation, Directory convergence, draining, subtree removal, final
   inventory and root/Store retirement.
2. Component provisioning and Fleet Coordinator ops/workflows.
3. Remaining Canister pool, bootstrap and Wasm Store lifecycle.
4. Fleet Mirror, Component Directory synchronization and Fleet-service peer
   workflows.
5. Remaining `canic-core` runtime, RPC, placement, replay and cascade sites.
6. Final rescan proving every reference has one disposition and every allocated
   meaning has a current producer.

The order follows authority and external-effect risk rather than source-file
alphabetical order. Each sub-ledger must record the concrete function and
constructor site, not merely the enclosing file.

## Per-File Frontier

| References | Production file | Current disposition |
| ---: | --- | --- |
| 800 | `crates/canic-control-plane/src/ops/component_registry/mod.rs` | Fully classified |
| 354 | `crates/canic-control-plane/src/workflow/component_registry/mod.rs` | Fully classified |
| 177 | `crates/canic-control-plane/src/ops/component_provisioning.rs` | Fully classified |
| 56 | `crates/canic-control-plane/src/workflow/component_provisioning.rs` | Fully classified |
| 154 | `crates/canic-control-plane/src/ops/fleet_coordinator/mod.rs` | Fully classified: 154 direct sites and all 235 receipt-funnel calls |
| 69 | `crates/canic-control-plane/src/ops/canister_pool/mod.rs` | Fully classified across three consecutive range owners |
| 69 | `crates/canic-control-plane/src/workflow/fleet_subnet_root.rs` | Fully classified |
| 32 | `crates/canic-core/src/workflow/component_runtime.rs` | Site-level reconciliation open |
| 32 | `crates/canic-core/src/workflow/runtime/auth/prepare/replay.rs` | Site-level reconciliation open |
| 27 | `crates/canic-control-plane/src/workflow/fleet_registry_mirror/mod.rs` | Site-level reconciliation open |
| 23 | `crates/canic-control-plane/src/ops/component_directory_synchronization/mod.rs` | Site-level reconciliation open |
| 23 | `crates/canic-control-plane/src/workflow/bootstrap/root_store/mod.rs` | Fully classified, including 2 transparent typed topology adapters |
| 21 | `crates/canic-control-plane/src/ops/fleet_coordinator/root_deletion/mod.rs` | Fully classified: 21 direct sites and all 10 hidden receipt-funnel calls |
| 15 | `crates/canic-core/src/workflow/placement/allocation.rs` | Site-level reconciliation open |
| 14 | `crates/canic-core/src/workflow/ic/icp_refill/replay.rs` | Site-level reconciliation open |
| 14 | `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs` | Site-level reconciliation open |
| 13 | `crates/canic-control-plane/src/ops/fleet_service_peer/mod.rs` | Site-level reconciliation open |
| 13 | `crates/canic-control-plane/src/workflow/component_directory_synchronization/mod.rs` | Site-level reconciliation open |
| 13 | `crates/canic-core/src/ops/component_provisioning_plan/mod.rs` | Site-level reconciliation open |
| 13 | `crates/canic-core/src/ops/fleet_registry/mod.rs` | Site-level reconciliation open |
| 12 | `crates/canic-control-plane/src/workflow/fleet_coordinator/mod.rs` | Fully classified, including 6 transparent root-error propagation sites |
| 11 | `crates/canic-control-plane/src/workflow/canister_pool/mod.rs` | Fully classified with its 6-site refill child |
| 11 | `crates/canic-core/src/ops/auth/token/error.rs` | Site-level reconciliation open |
| 11 | `crates/canic-core/src/workflow/runtime/fleet_activation.rs` | Site-level reconciliation open |
| 10 | `crates/canic-core/src/workflow/runtime/intent.rs` | Site-level reconciliation open |
| 9 | `crates/canic-core/src/ops/storage/authority_restore/mod.rs` | Site-level reconciliation open |
| 9 | `crates/canic-core/src/workflow/rpc/request/handler/execute.rs` | Site-level reconciliation open |
| 8 | `crates/canic-control-plane/src/workflow/runtime/template/mod.rs` | Site-level reconciliation open |
| 8 | `crates/canic-core/src/workflow/runtime/auth/renewal.rs` | Site-level reconciliation open |
| 7 | `crates/canic-core/src/ops/auth/delegation/chain_key_batch/mod.rs` | Site-level reconciliation open |
| 7 | `crates/canic-core/src/ops/auth/token/verification.rs` | Site-level reconciliation open |
| 7 | `crates/canic-core/src/workflow/runtime/auth/prepare/admission.rs` | Site-level reconciliation open |
| 6 | `crates/canic-control-plane/src/workflow/canister_pool/refill.rs` | Fully classified with parent Canister pool workflow |
| 6 | `crates/canic-core/src/workflow/rpc/request/handler/authorize.rs` | Site-level reconciliation open |
| 6 | `crates/canic-core/src/workflow/runtime/auth/mod.rs` | Site-level reconciliation open |
| 6 | `crates/canic-core/src/workflow/runtime/root.rs` | Site-level reconciliation open |
| 5 | `crates/canic-control-plane/src/ops/fleet_registry_mirror/mod.rs` | Site-level reconciliation open |
| 5 | `crates/canic-control-plane/src/ops/storage/state/root_wasm_store.rs` | Fully classified with root bootstrap Subnet discovery |
| 5 | `crates/canic-control-plane/src/workflow/component_rpc/lifecycle.rs` | Site-level reconciliation open |
| 5 | `crates/canic-core/src/ops/fleet_service_binding/mod.rs` | Site-level reconciliation open |
| 5 | `crates/canic-core/src/ops/storage/intent/mod.rs` | Site-level reconciliation open |
| 5 | `crates/canic-core/src/workflow/env/mod.rs` | Site-level reconciliation open |
| 5 | `crates/canic-core/src/workflow/runtime/auth/root_issuer/mod.rs` | Site-level reconciliation open |
| 5 | `crates/canic-core/src/workflow/runtime/nonroot.rs` | Site-level reconciliation open |
| 4 | `crates/canic-control-plane/src/workflow/runtime/template/client/mod.rs` | Site-level reconciliation open |
| 4 | `crates/canic-control-plane/src/workflow/runtime/template/publication/lifecycle/gc.rs` | Site-level reconciliation open |
| 4 | `crates/canic-core/src/ops/auth/delegation/errors.rs` | Site-level reconciliation open |
| 4 | `crates/canic-core/src/workflow/cascade/topology.rs` | Site-level reconciliation open |
| 4 | `crates/canic-core/src/workflow/ic/icp_refill/mod.rs` | Site-level reconciliation open |
| 3 | `crates/canic-control-plane/src/workflow/bootstrap/root.rs` | Fully classified with sibling Store state facade |
| 3 | `crates/canic-control-plane/src/workflow/runtime/fleet_activation/mod.rs` | Site-level reconciliation open |
| 3 | `crates/canic-control-plane/src/workflow/wasm_store/mod.rs` | Site-level reconciliation open |
| 3 | `crates/canic-core/src/workflow/placement/scaling/mod.rs` | Site-level reconciliation open |
| 3 | `crates/canic-core/src/workflow/runtime/auth/prepare/mod.rs` | Site-level reconciliation open |
| 3 | `crates/canic-core/src/workflow/runtime/authority_restore.rs` | Site-level reconciliation open |
| 3 | `crates/canic-core/src/workflow/runtime/mod.rs` | Site-level reconciliation open |
| 2 | `crates/canic-control-plane/src/ops/fleet_coordinator/deployment_ledger/mod.rs` | Fully classified: 2 direct sites and all 47 hidden receipt-funnel calls, including 2 test-only dispositions |
| 2 | `crates/canic-control-plane/src/workflow/runtime/template/publication/lifecycle/inventory.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/ops/auth/delegation/active.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/ops/auth/delegation/chain_key_batch/merkle.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/ops/auth/token/retention/mod.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/ops/component_provisioning_receipt/mod.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/ops/component_runtime.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/ops/config/mod.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/ops/rpc/mod.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/ops/rpc/request/dispatch.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/ops/runtime/init_payload.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/workflow/cascade/snapshot/mod.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/workflow/cost_guard/mod.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/workflow/rpc/capability/mod.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/workflow/runtime/cycles/mod.rs` | Site-level reconciliation open |
| 2 | `crates/canic-core/src/workflow/runtime/log.rs` | Site-level reconciliation open |
| 1 | `crates/canic-control-plane/src/config.rs` | Site-level reconciliation open |
| 1 | `crates/canic-control-plane/src/workflow/root_authority/mod.rs` | Site-level reconciliation open |
| 1 | `crates/canic-control-plane/src/workflow/runtime/template/publication/lifecycle/creation.rs` | Site-level reconciliation open |
| 1 | `crates/canic-control-plane/src/workflow/state/mod.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/api/component_deployment.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/ops/auth/delegation/chain_key_batch/install.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/ops/auth/delegation/chain_key_batch/selection.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/ops/auth/delegation/chain_key_batch/signing.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/ops/auth/delegation/chain_key_registry.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/ops/fleet_activation.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/ops/ic/cycles_ledger.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/ops/ic/icp_refill.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/ops/ic/mod.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/ops/root_draining_reservation/mod.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/ops/runtime/env/mod.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/ops/runtime/install_source.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/ops/runtime/log.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/workflow/ic/icp_refill/cost_guard.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/workflow/placement/index/cleanup.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/workflow/placement/index/config.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/workflow/placement/index/create.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/workflow/placement/index/mod.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/workflow/placement/sharding/bootstrap.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/workflow/placement/sharding/mod.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/workflow/rpc/authority.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/workflow/rpc/request/handler/nonroot_cycles.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/workflow/rpc/request/mod.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/workflow/runtime/auth/root_delegation_batch/mod.rs` | Site-level reconciliation open |
| 1 | `crates/canic-core/src/workflow/topology/guard.rs` | Site-level reconciliation open |

## Completion Rule

B1 site coverage is complete only when a mechanical manifest accounts for all
2,208 baseline references and fails on an unclassified added site. Line numbers
are evidence locations, not stable identity; the manifest key must include the
owning function and typed variant or constructor ordinal where a function has
several distinct decisions.

The final proposed allocation is produced only after this manifest is
reconciled with the 685-name qualified subset. The result may be larger, or may
be smaller if the full owner/action/retry review proves qualified names
identical. Neither outcome permits a generic fallback code.
