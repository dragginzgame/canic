# Capability Pipeline Conformance Audit — 2026-03-08

## Run Context

- Audit run: `capability-pipeline-conformance`
- Definition: `docs/audits/recurring/capability-pipeline-conformance.md`
- Auditor: `codex`
- Date (UTC): `2026-03-08 17:08:39Z`
- Branch: `eleven`
- Commit: `c98bb574`
- Worktree: `dirty`
- Scope:
  - `crates/canic-core/src/api/rpc/capability/*`
  - `crates/canic-core/src/workflow/rpc/request/handler/*`
  - `crates/canic-core/src/ops/rpc/mod.rs`
  - `crates/canic/src/macros/endpoints.rs`

## Checklist

### 1. Envelope Validation Is Fail-Closed

- [x] Unsupported service rejected (`service must be Root`).
- [x] Unsupported capability/proof versions rejected.
- [x] Envelope rejection occurs before proof/execute path.

Evidence:
- `api/rpc/capability/envelope.rs`
- `api/rpc/capability/mod.rs` (`validate_root_capability_envelope` before proof/dispatch)
- Unit coverage in `api/rpc/capability/tests.rs`:
  - service mismatch
  - capability version mismatch
  - proof version mismatch

### 2. Capability Hash Binding Is Canonical

- [x] Hash is domain separated (`CANIC_CAPABILITY_V1`).
- [x] Canonical payload strips request metadata before hashing.
- [x] Role-attestation and delegated-grant paths verify binding.
- [x] Hash mismatch rejects as invalid.

Evidence:
- `ops/rpc/mod.rs` (`root_capability_hash`, caller envelope construction)
- `api/rpc/capability/hash.rs`
- `api/rpc/capability/proof.rs`
- Unit tests:
  - `verify_capability_hash_binding_accepts_match` (pass)
  - `verify_capability_hash_binding_rejects_mismatch` (pass)

### 3. Proof Verification Covers All Proof Modes

- [x] Structural proof route exists and is constrained by capability family.
- [x] Role-attestation route performs hash binding plus attestation verification.
- [x] Delegated-grant route performs hash, claim, and signature checks.
- [x] Proof-mode dispatcher fails closed on mismatches.

Evidence:
- `api/rpc/capability/verifier.rs`
- `api/rpc/capability/proof.rs`
- `api/rpc/capability/grant.rs`

### 4. Replay Metadata Projection Is Safe

- [x] Zero TTL rejected.
- [x] Future issued-at beyond skew rejected.
- [x] Expired metadata rejected.
- [x] Replay request id binds `(request_id, nonce)`.

Evidence:
- `api/rpc/capability/replay.rs`
- `api/rpc/capability/mod.rs`
- Unit coverage in `api/rpc/capability/tests.rs`:
  - expired metadata rejection
  - future metadata rejection
  - nonce binding behavior

### 5. Pipeline Ordering Is Preserved

- [x] Envelope validation occurs before proof verification.
- [x] Proof verification occurs before replay metadata projection.
- [x] Capability path dispatches through `response_replay_first`.
- [x] Replay commit remains post-execution in workflow.

Evidence:
- `api/rpc/capability/mod.rs`
- `workflow/rpc/request/handler/mod.rs`

### 6. Policy and Execution Responsibilities Stay Split

- [x] Authorization functions remain in `authorize.rs`.
- [x] Side effects remain in `execute.rs`.
- [x] No direct execute path bypasses preflight.

Evidence:
- `workflow/rpc/request/handler/authorize.rs`
- `workflow/rpc/request/handler/execute.rs`
- `workflow/rpc/request/handler/mod.rs`

### 7. Test Coverage

- [x] Capability-hash unit checks executed in this run and passed.
- [x] Replay guard unit checks executed in this run and passed.
- [x] Focused integration coverage now exercises `InsufficientRootCycles` authorization branch.

Executed tests:
- `cargo test -p canic-core verify_capability_hash_binding_accepts_match`
- `cargo test -p canic-core verify_capability_hash_binding_rejects_mismatch`
- `cargo test -p canic-core check_replay_rejects_duplicate_same_payload`
- `cargo test -p canic-core check_replay_rejects_conflicting_payload_for_same_request_id`
- `cargo test -p canic-core check_replay_rejects_invalid_ttl`
- `cargo test -p canic cycles_rejects_when_requested_above_root_balance`

## Findings

### High

- None.

### Medium

- None.

### Low

- None.

## Verdict

- Pipeline conformance: **Pass**
- Fail-closed behavior: **Pass**
- `InsufficientRootCycles` branch now covered by focused PocketIC replay integration.
