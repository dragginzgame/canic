# 0.33 Refactor Addendum

Status: proposed follow-up
Date: 2026-05-09

This addendum tracks post-0.33 design refactors identified during the
complexity cleanup. These are not required for the ICP CLI hard cut, and should
not be done as drive-by rewrites. Each item should be taken only when the area
is already being changed or when a focused refactor can preserve behavior with
clear tests.

## Current Context

The 0.33.2 cleanup reduced the complexity audit residual risk by splitting
test/support bulk out of production modules for metrics, directory placement,
config schema, and intent storage. The remaining large files are mostly active
facades or workflows where mechanical splitting would create churn without
improving the design.

## Candidates

### 1. Config Validation Ownership

Recommended first.

Current pressure:

- `config/schema/mod.rs` and `config/schema/subnet.rs` still combine serde
  shapes with cross-role validation policy.
- Validation changes are likely to continue as fleet, environment, auth,
  scaling, sharding, and directory behavior evolves.

Suggested shape:

- Keep `config/schema/*` as passive serde/data shapes.
- Move cross-object validation into `config/validation/*`.
- Split validators by feature area:
  - `auth`
  - `fleet`
  - `scaling`
  - `sharding`
  - `directory`
  - `topup`

Target result:

- Schema files stay data-oriented.
- Validation failures remain typed through `ConfigSchemaError`.
- Feature validators can be tested independently without building whole config
  fixtures for every case.

### 2. IC Management Facade Split

Recommended second.

Current pressure:

- `ops/ic/mgmt.rs` and `infra/ic/mgmt.rs` are large facade modules.
- The boundary between domain-safe operations and raw IC transport should stay
  explicit as ICP CLI and management-canister behavior changes.

Suggested shape:

- `ops/ic/canister_lifecycle.rs`: create, install, reinstall, upgrade.
- `ops/ic/cycles.rs`: balance, deposit, top-up, cycle-account behavior.
- `ops/ic/snapshots.rs`: snapshot list/create/load/delete.
- `ops/ic/calls.rs`: query/update/raw call wrappers.
- `ops/ic/chain_key.rs`: ECDSA/Schnorr public-key and signature calls.
- Mirror only the necessary lower-level transport helpers in `infra/ic/*`.

Target result:

- `ops` remains the domain-safe management facade.
- `infra` remains thin transport.
- Call sites import the capability they need instead of one broad management
  module.

### 3. Provisioning Workflow Phases

Current pressure:

- `workflow/ic/provision.rs` coordinates validation, allocation, install,
  registration, propagation, and metrics.
- It is behavior-heavy, so this should be split only with focused tests around
  phase boundaries.

Suggested shape:

- `validate`: request, role, and topology checks.
- `allocate`: create or reuse a canister.
- `install`: install/reinstall code and arguments.
- `register`: write topology and registry state.
- `propagate`: cascade state and metrics.

Target result:

- Provisioning order is explicit.
- Phase-specific failure behavior is testable without reading the whole flow.
- Metrics remain at workflow boundaries, not in pure validation helpers.

### 4. Sharding Placement Decomposition

Current pressure:

- Directory placement now has clearer separation between orchestration,
  state/support, and tests.
- Sharding has similar growth potential but should not be split until sharding
  behavior changes again.

Suggested shape:

- `planning`: decide desired shard/worker placement.
- `selection`: choose existing shard/worker.
- `lifecycle`: create/retire/repair shard canisters.
- `storage`: keep deterministic state transitions in ops/storage.

Target result:

- Sharding mirrors the directory placement decomposition where useful.
- Planning and storage mutations stay separate.

### 5. Root Request Pipeline

Current pressure:

- Root request handling coordinates request family, replay, authorization,
  metadata, execution, cycles, and cached responses.
- Ordering matters for security, so this needs a deliberate design rather than
  a file split.

Suggested shape:

- `map`: convert DTO request to an internal command.
- `replay_preflight`: reject invalid/expired/conflicting replay metadata.
- `authorize`: validate capability, caller, subject, and target bindings.
- `execute`: perform the authorized operation.
- `commit_replay`: persist successful replay response or abort reservation.

Target result:

- Security ordering is visible in types and phase names.
- Bypass-prone steps become harder to skip accidentally.
- Tests can target phase ordering directly.

## Non-Goals

- No compatibility shims.
- No new public API solely to support the refactor.
- No broad behavior changes while moving modules.
- No touching `canic-cli` as part of this addendum unless a later CLI-owned
  task explicitly requires it.

## Suggested Order

1. Config validation ownership.
2. IC management facade split.
3. Provisioning workflow phases.
4. Sharding placement decomposition, only when sharding changes resume.
5. Root request pipeline, only as a dedicated security/design refactor.
