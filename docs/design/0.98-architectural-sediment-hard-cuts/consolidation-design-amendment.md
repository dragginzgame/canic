# 0.98.2 Consolidation Design Amendment

Date: 2026-07-22

Status: closed at immutable `v0.98.2`

## 1. Objective

This amendment records the scope added to 0.98.2 before publication. It
reconstructs the current Canic architecture from source and removes bounded
remnants of superseded designs. Readiness means that each current
responsibility has one named authority, every investigated candidate has a
disposition, and no removed path remains reachable through production,
features, macros, tests, examples, fixtures, build scripts, or operator tools.

This is a bounded extension of the existing 0.98 hard-cut line. Confirmed sediment is hard-cut rather than
deprecated, aliased, dual-written, or preserved behind a compatibility
branch.

## 2. Exact baseline and amended 0.98.2 boundary

The immutable repository baseline is:

- commit `e0dcd0cbb8f550e4c0366d9e1007ca32dceb2aa7`;
- tree `ae154b0deb862702d48fed4dd235caf76089f7a2`;
- published tag `v0.98.1` at the same tree; and
- committed `Cargo.lock` SHA-256
  `801ad42f9b2a733e925d3c4b0b66cae1922b60b3b7b2cc0166a9a52cfd2092e2`.

The maintainer explicitly approved combining the original Slice C randomness
hard cut and this consolidation work in one 0.98.2 patch. The immutable
comparison baseline for the complete patch is therefore `v0.98.1`; an
intermediate dirty-worktree fingerprint is not a release authority.

The published 0.98.0 and 0.98.1 work remains immutable and is not counted
again. The released 0.98.2 batch owns:

1. deletion of the unused per-role randomness configuration and unreachable
   `raw_rand` adapter; and
2. the 42-candidate consolidation audit recorded in the supporting ledger,
   including its one fixed P1, 11 fixed P2 findings, and 30 proved notes.

This amendment does not reinterpret the already released
`project-protocol-stub` or LocalIntent fixture deletions.

## 3. Evidence order and audit method

The audit uses evidence in this order:

1. production entry points and state mutation paths;
2. durable records, Candid/JSON contracts, generated descriptors, and config
   schemas;
3. macro expansion and feature-selected build graphs for native and wasm;
4. workspace metadata, reverse consumers, build scripts, tests, fixtures, and
   deployment tooling;
5. current architecture, status, closeout, audit, and changelog documents;
6. selective Git history explaining why a suspicious surface was introduced
   or why its consumer disappeared.

Textual absence alone is not deletion proof. For each candidate the ledger
records its old responsibility, current owner, producer/consumer boundary,
target and feature reachability, persistence or contract implications, and
the reason its disposition preserves current behavior.

Historical design and dated audit reports remain evidence, not supported
surface. A historical symbol mention is retained when the document is clearly
anchored to its old snapshot and does not instruct a current caller to use the
removed path.

## 4. Reconstructed authority map

### 4.1 Crates and dependency direction

| Responsibility | Canonical owner | Allowed consumers and direction |
| --- | --- | --- |
| Public canister facade and lifecycle/build macros | `canic` | Role packages depend on the facade; it delegates to core/control-plane/macros |
| Runtime model, DTOs, policy, ops, workflow, stable state | `canic-core` | No Canic workspace dependency; lowest shared runtime layer |
| Procedural endpoint expansion | `canic-macros` | Used by `canic`; expansion reaches only the documented hidden facade boundary |
| Root and Wasm-store control-plane behavior | `canic-control-plane` | Depends only on `canic-core`; selected by facade features |
| Built-in Wasm-store canister | `canic-wasm-store` | Depends on `canic` with the exact Wasm-store role feature |
| Build, config, artifact, deployment-truth, and ICP CLI adapters | `canic-host` | May consume runtime boundary types; CLI consumes host, never the reverse |
| Operator command parsing and rendering | `canic-cli` | Depends on host, backup, and semantic core DTOs |
| Backup/restore plan, journal, receipt, artifact, and recovery model | `canic-backup` | Independent host-side library; CLI supplies the maintained process adapter |
| Shared PocketIC harness internals | `canic-testing-internal` | Test-only consumer of product crates and current role canisters |
| Cross-canister integration suites | `canic-tests` | Uses testing-internal and current product contracts |

The resulting product direction is:

```text
role canisters -> canic -> {canic-core, canic-control-plane, canic-macros}
canic-control-plane -> canic-core

canic-cli -> {canic-host, canic-backup, canic-core}
canic-host -> {canic-control-plane, canic-core}
canic-backup -> external crates only

canic-tests -> canic-testing-internal -> product crates
```

No shared runtime crate depends on a canister-role package, host facade, CLI,
or backup implementation.

### 4.2 Canister roles and deployment ownership

| Area | Authority |
| --- | --- |
| Role declaration and package identity | `[roles]` plus `package.metadata.canic`, validated by core/host role contracts |
| Topology attachment | `[subnets.*.canisters]`; child roles are derived through the current scaling/sharding/directory shapes |
| Exactly one root and root placement | Config validation; root must be the singleton root role on `prime` |
| Root bootstrap and managed release activation | Thin `canic::start!` adapter -> lifecycle workflow -> control plane |
| Built-in Wasm-store generation/staging | `canic` build support and `canic-host::bootstrap_store`; the generated or canonical package uses one feature contract |
| Installed deployment identity | Host install state plus live root registry evidence |
| Build artifacts and environment selection | Host release-set/build provenance; `environment`, `artifact_environment`, and `build_network` remain distinct |

### 4.3 Public facade and macro expansion

- `canic` owns the supported public API, DTO, ID, lifecycle, and macro
  surface.
- `canic-macros` parses endpoint declarations and expands against semantic
  facade paths.
- `canic-core::cdk` remains public only as a hidden cross-crate/macro plumbing
  boundary; it is not a documented application facade.
- `canic::build!` is the sole role build-script entry and is validated by
  parsing Rust syntax, not matching source lines.
- `canic::start!` stays a thin lifecycle adapter; restoration is synchronous
  and hooks are deferred through the current timer authority.

### 4.4 Configuration authority

- `canic-core::config::schema::ConfigModel` is the sole accepted TOML schema.
- Parsing is strict (`deny_unknown_fields`) and produces typed logical-path
  evidence for invalid input.
- Host discovery parses each chosen config once and passes that model through
  passive role and state-contract validation.
- Build-time rendering produces the compiled role config; runtime reads that
  compiled value rather than reparsing project files.
- Host projection, role capability validation, deployment planning, and the
  operator guide all describe the same current fields.
- Omitted application whitelist is deny-all. ICP refill is root-only,
  controller-triggered, and manual; no timer owns it.

### 4.5 Authentication and authorization

- Endpoints authenticate and authorize; workflow/ops receive authenticated
  context.
- Role attestation uses one structural root canister-signature proof shape.
- Delegated authorization uses the chain-key root batch/certificate model and
  issuer-local canister signatures for token claims.
- Canonical hashes bind caller/subject, root, issuer, audience, role grants,
  key policy, registry epoch, proof epoch, and time bounds.
- Root and issuer canister-signature seed/domain constants are the only
  payload-family identifiers; removed singleton `*PayloadKind` enums no
  longer pretend that multiple internal modes exist.
- Runtime status, CLI auth commands, current architecture documentation, and
  recurring trust-chain audit rules project this same model.

### 4.6 Intent, execution, receipts, replay, and recovery

- `ReceiptBackedIntentOps` owns durable receipt/resource records and ordered
  indexes.
- Pure policy decides admission and replay windows; ops mutate records;
  workflows orchestrate external effects and settlement.
- Replay receipts bind operation ID, caller, command kind, payload, target,
  cost settlement, external-effect evidence, and exact revision.
- Statuses `Reserved`, `ExternalEffectInFlight`, terminal response staging,
  `Committed`, and `RecoveryRequired` are current state-machine states, not
  migration aliases.
- Application receipts and Canic-reserved receipt namespaces share storage but
  not authority: public consumer APIs cannot read or settle `canic:` records.
- Same-ID replay is the supported exact observation path; aggregate capacity,
  typed errors, workflow records, and logs are diagnostics. A new raw receipt
  dump endpoint would expand a security-sensitive public contract and is not
  required to remove sediment.

### 4.7 Backup, restore, and operator recovery

- `canic-backup` owns version-1 plans, execution and artifact journals,
  receipts, manifests, integrity validation, locks, and reconciliation.
- `canic-cli` owns command parsing/rendering and supplies the current
  `canic-host::icp` executor.
- Snapshot create/list/download occurs through the typed ICP CLI adapter.
  Runtime canisters do not own a duplicate management-canister snapshot path.
- Private upload/download staging, pending claims, inventory reconciliation,
  checksum-bound publication, stopped-state proof, and command-tree lifetime
  locks are current safety machinery. They are not obsolete rollback or
  unknown-outcome shortcuts.
- No pending-reset command, blind replay, text snapshot-ID parser, direct
  snapshot CLI, or alternate backup runner survives.

### 4.8 Management-canister and platform effects

- `infra::ic::mgmt` contains target-specific raw CDK calls and passive
  argument/result shapes.
- `ops::ic::mgmt` owns checked single-step platform effects and typed metrics.
- Workflow owns multi-step provisioning, install, delete, funding, HTTP, and
  chain-key orchestration.
- After this line, runtime snapshot adapters are absent because all surviving
  snapshot consumers are host backup/restore paths.
- Each remaining management operation has a production, feature-selected, or
  current test seam consumer.

### 4.9 Timers and asynchronous workflows

- `TimerWorkflow` is the only built-in and application scheduling authority.
- Each subsystem provides an exact next deadline and consumes one
  generation-safe, non-overlapping timer identity.
- Lifecycle restores state synchronously and schedules asynchronous work.
- Intent cleanup, placement acknowledgement, pool reset, auth renewal, cycle
  funding, and log retention are deadline/work-driven rather than parallel
  fixed-rate schedulers.
- Root ICP-to-cycles conversion is deliberately absent from the timer key
  catalog.

### 4.10 Topology, registry, pool, template, and Wasm-store state

- Authoritative topology and registry records live in control-plane/core
  storage and are projected through ops/views.
- Placement allocation is the sole child-creation authority across scaling,
  sharding, and directory consumers.
- Pool reset owns a current generation-bound pending record and bounded cursor;
  it is not the removed restore pending-reset command.
- Root-owned publication binding plus active/detached/retired lifecycle state
  is the sole Wasm-store publication selection authority.
- Store-local GC is one-way; stores in GC cannot publish.

### 4.11 Protocols, generated clients, and type ownership

- DTOs are passive Candid/Serde boundary values.
- `model` owns authoritative runtime states; `storage::stable::*Record` owns
  persisted schemas; canonical snapshots end in `*Data`; `view` owns internal
  read-only projections.
- Checked-in Candid consists only of the canonical Wasm-store descriptor and
  two external blob-storage fixtures. Protocol tests bind those descriptors
  to current semantic DTOs and method inventories.
- Local `.icp` declarations and generated wrappers are build artifacts, not
  checked-in alternate authorities.
- Version-1 JSON names remain on current externally persisted evidence. They
  are not compatibility readers for older shapes.

### 4.12 Stable and durable state

- `canic-core::state_contract` and control-plane state contracts declare
  memory IDs, record types, bounds, applicability, export/import coverage, and
  migration policy.
- The host state manifest combines those declarations with one parsed config
  and role package evidence.
- Active CycleTracker memory is modeled as active with its canonical
  `CycleTrackerEntryRecord`/`CycleTrackerData` snapshot path.
- Reserved ranges remain deliberate allocation policy. No stable memory ID is
  renumbered in 0.98.2.

### 4.13 CLI evidence and test infrastructure

- CLI status/inspect/medic/state/auth/cycles/deploy/backup/restore commands
  render current host/runtime evidence and preserve typed underlying causes.
- `canic-testing-internal` owns shared PocketIC build/install/query helpers;
  `canic-tests` owns behavior suites.
- Audit probes measure supported role/feature floors. Test canisters each map
  to a current PocketIC contract. The blank sandbox remains a documented
  maintainer compile-drift workspace, not production architecture.

## 5. Hard-cut rules

For every implemented finding:

- delete old code and tests together;
- preserve only the canonical owner;
- do not add deprecated aliases, compatibility readers, feature fallbacks, or
  dual writes;
- add or update tests only around the surviving contract;
- reject retired config through the current strict parser rather than a
  special legacy scanner;
- retain historical reports only as snapshot evidence; and
- make Candid, stable, JSON, CLI, dependency, and public API impact explicit.

## 6. Implementation slices

### Slice A — Baseline and authority reconstruction

Freeze the immutable `v0.98.1` baseline and the complete 0.98.2 scope,
enumerate all 37 workspace
packages, derive dependency direction, inventory features/build scripts/
generated descriptors, and build this authority map.

### Slice B — Build and configuration truth

Replace lexical `canic::build!` detection with a Syn syntax visitor, pass one
parsed config through passive validation, correct the current configuration
guide, add a guide-parsing test, and remove a retired testing annotation
breadcrumb.

### Slice C — Runtime and state hard cuts

Delete the dead host duration parser, narrow control-plane internals, model
CycleTracker as active state, collapse the obsolete capability verifier/router,
and delete the unreachable runtime snapshot adapter plus dead metrics.

### Slice D — Authentication shape consolidation

Remove singleton root/issuer payload-kind taxonomies while preserving exact
seed/domain bytes and all wire, Candid, stable, and cryptographic behavior.

### Slice E — Host/operator surface and dependency evidence

Delete orphan ICP CLI wrappers left by the old snapshot/operator surface,
narrow helper visibility to actual cross-crate consumers, and annotate the
three manifest-only role fixture dependencies for Cargo Machete.

### Slice F — Repository closure

Repeat exact and conceptual stale-path searches, resolve every ledger row,
run the completion validation matrix, update changelog/status, and write the
closeout document without changing package versions or external state.

## 7. Contract impact policy

The complete 0.98.2 patch includes the original breaking TOML/public Rust
randomness hard cut and these additional unconsumed Rust surfaces:

- dead `canic-core` runtime snapshot operations and their impossible metric
  labels;
- dead `canic-host::icp` convenience/display methods;
- host ICP helpers that were public despite having only same-crate consumers;
  and
- control-plane modules that were public despite having only crate-local
  consumers.

Explicit per-role randomness input is rejected by the surviving strict schema.
There is no stable-memory ID or record-byte change, no Candid method/type
change, no maintained JSON key change, no CLI command/flag/output change, and
no Cargo package version change. Host-only `serde_path_to_error 0.1.20`
provides typed config-path evidence. The direct `canic-host` Syn dependency
uses the already compiled Syn 3.0.3 graph and adds no dependency version.
The token trust-chain recurring audit advances from v1 to fingerprinted v2 so
its search inventory names the surviving seed/domain authorities; the v1
fingerprint remains in the superseded manifest.

## 8. Closure criteria

The amended 0.98.2 batch is ready to publish only when:

1. all 37 packages and every authority-map area have coverage;
2. all candidate ledger rows have a non-`UNRESOLVED` disposition;
3. all confirmed P0/P1 and bounded P2 findings are remediated;
4. current config fields, features, adapters, packages, and generated
   contracts have proven consumers;
5. stable/Candid/JSON/CLI/public/dependency effects are recorded;
6. targeted and completion validation pass, with any unavailable external
   journey stated as a limitation rather than fabricated evidence;
7. stale-symbol/conceptual searches leave only strict rejection fixtures or
   clearly historical evidence; and
8. changelog, status, tracker, ledger, and closeout agree on counts and verdict.
