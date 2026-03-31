# Replay Integrity and TTL Audit — 2026-03-08

## Run Context

- Audit run: `replay-integrity-and-ttl`
- Definition: `docs/audits/recurring/replay-integrity-and-ttl.md`
- Auditor: `codex`
- Date (UTC): `2026-03-08 17:08:39Z`
- Branch: `eleven`
- Commit: `c98bb574`
- Worktree: `dirty`
- Scope:
  - `crates/canic-core/src/ops/replay/*`
  - `crates/canic-core/src/ops/storage/replay.rs`
  - `crates/canic-core/src/storage/stable/replay.rs`
  - `crates/canic-core/src/workflow/rpc/request/handler/replay.rs`
  - `crates/canic/tests/root_replay.rs`

## Checklist

### 1. Replay Slot Identity Is Stable and Scoped

- [x] Canonical slot key derives from caller/target/service/request identity.
- [x] Legacy slot key compatibility path is present and explicit.
- [x] Slot key derivation remains deterministic.

Evidence:
- `ops/storage/replay.rs` (`REPLAY_SLOT_KEY_DOMAIN`, `slot_key`)
- `ops/replay/key.rs` (`root_slot_key`, `legacy_root_slot_key`)
- `ops/replay/guard.rs` (canonical + legacy lookup)

### 2. TTL Validation Is Strict and Mechanical

- [x] TTL validation is pure and centralized.
- [x] Zero TTL and above-max TTL reject with typed error.
- [x] Workflow maps guard TTL errors to replay workflow errors.

Evidence:
- `ops/replay/ttl.rs`
- `ops/replay/guard.rs`
- `workflow/rpc/request/handler/replay.rs`

### 3. Existing Slot Decisions Match Contract

- [x] Existing expired record yields `Expired`.
- [x] Same payload hash yields `DuplicateSame`.
- [x] Different payload hash yields `DuplicateConflict`.
- [x] Missing record yields `Fresh`.

Evidence:
- Decision logic in `ops/replay/guard.rs` (`resolve_existing`)
- Workflow mapping in `workflow/rpc/request/handler/replay.rs`

### 4. Payload Binding and Commit Semantics Are Correct

- [x] Replay payload digest comes from canonical capability command bytes.
- [x] Replay record persists `payload_hash` and `response_candid`.
- [x] Commit happens after successful execution path.

Evidence:
- `workflow/rpc/request/handler/replay.rs` (`hash_capability_payload`, `commit_replay`)
- `ops/replay/mod.rs` and `ops/replay/slot.rs`
- `storage/stable/replay.rs` (`RootReplayRecord`)

### 5. Store Boundedness and Expiry Purge Behavior

- [x] Bounded replay capacity check exists.
- [x] Expiry purge exists and respects scan limit behavior.
- [x] Replay capacity error maps to typed workflow error.

Evidence:
- `ops/replay/mod.rs`
- `ops/storage/replay.rs`
- `storage/stable/replay.rs`
- `workflow/rpc/request/handler/replay.rs`

### 6. Test Coverage for Replay Guarantees

- [x] Unit tests cover duplicate/conflict/expired/invalid-ttl behavior.
- [x] Root replay integration tests exist for replay semantics.
- [x] Targeted PocketIC replay integration reruns completed successfully.

Executed tests:
- `cargo test -p canic-core check_replay_rejects_duplicate_same_payload`
- `cargo test -p canic-core check_replay_rejects_conflicting_payload_for_same_request_id`
- `cargo test -p canic-core check_replay_rejects_invalid_ttl`
- `cargo test -p canic cycles_routes_through_dispatcher_and_replay_duplicate_same`
- `cargo test -p canic cycles_rejects_when_requested_above_root_balance`

## Findings

### High

- None.

### Medium

- None.

### Low

- Local sandbox runs can still vary with temporary-directory and target-dir
  layout; replay-focused reruns pass with the default workspace target dir.

## Verdict

- Replay integrity semantics: **Pass**
- TTL enforcement: **Pass**
- Replay integration reruns now pass in the current local environment.
