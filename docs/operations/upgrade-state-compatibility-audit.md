# Upgrade and State Compatibility Audit

This audit is the durable upgrade and state compatibility inventory for Canic
release work.

It documents existing repo evidence for replay-sensitive state after the 0.61
replay-protection line. It is intentionally not named after a release line;
release numbers belong in changelogs and status docs, not in the operational
audit entry point.

Current release-line context: 0.62 is using this audit for upgrade confidence
and release durability.

## Scope

This audit covers the state surfaces that intersect replay-sensitive behavior:

- replay receipt persistence and stable shape;
- operation-ID durability;
- project-local pending operation logs;
- delegated-auth and delegation-proof replay state;
- caller/shard binding assumptions;
- delegated-token mint/issue replay state;
- ICP refill and value-transfer replay state;
- cost-guard accounting and permit boundaries;
- response-idempotent canister upgrade requests;
- lifecycle post-upgrade ordering;
- durable-publication and wasm-store state;
- stable-memory ABI boundaries.

This audit does not change runtime behavior, Candid, CLI output, JSON/output
formats, package manifests, dependencies, lockfiles, fixtures, snapshots, or
generated artifacts.

## Compatibility Boundaries

Stable state compatibility is distinct from Candid compatibility.

- Stable state compatibility means persisted records either decode into the
  supported runtime shape or fail with a controlled compatibility error.
- Candid compatibility means public canister method signatures remain stable.
- CLI/JSON compatibility means operator-visible output and automation schemas
  remain stable.

0.62.2 changes none of those surfaces.

Old-state-to-new-binary compatibility is scoped to supported persisted state
schemas. Downgrade behavior is a non-goal unless separately approved. Operations
completed before 0.61 are not retroactively replay-protected, and old request
DTOs without required operation IDs are not compatibility shims.

The delegated-token verifier-local token-use store is intentionally hard-cut.
Old token-use markers are not replay receipts and must not be migrated into the
shared replay receipt store.

## Outcome Labels

Each area below uses one of these outcomes:

| Outcome | Meaning |
| --- | --- |
| Covered by existing tests/docs | Existing code, tests, and docs are enough for current 0.62 release confidence. |
| Covered by this audit | This docs/CI slice makes the compatibility boundary explicit without runtime changes. |
| Known gap, non-blocking for RC | The gap should be tracked in RC accounting but is not a release blocker by itself. |
| Release blocker | The area must be fixed or proven before RC promotion. |

## State Area Matrix

| Area | Evidence | Outcome | Notes |
| --- | --- | --- | --- |
| Replay receipt persistence and shape stability | `ReplayReceiptRecord` stores `schema_version`, command kind, 32-byte operation ID, actor, payload-hash schema, status, response schema/bytes, and effect in `crates/canic-core/src/storage/stable/replay.rs`. Tests cover CBOR round-trip, committed receipt round-trip, pending/recovery receipt round-trip, shared receipt conversion, actor/operation listing, and unsupported schema rejection. | Covered by existing tests/docs | Current schema compatibility is covered. A live PocketIC fixture that seeds historical replay receipt bytes before upgrade would be optional hardening unless RC validation finds drift. |
| Operation-ID durability | Runtime replay receipts persist `[u8; 32]` operation IDs. Root upgrade RPC tests prove upgrade requests carry replay metadata into the request. CLI ICP refill generated IDs are written before send in the pending operation log. | Covered by existing tests/docs | Operation IDs remain caller/client-owned for replay-sensitive commands. Server correctness does not depend on the CLI pending log. |
| Project-local pending operation log | `crates/canic-cli/src/cycles/convert/pending.rs` writes `.canic/operations/pending.json` with log and entry schema versions, atomic temp-file replace, directory sync, `pending_send`, and `completed` states. Tests cover project-local path, write-before-send, matching pending reuse, and completed entries not being reused. | Covered by existing tests/docs | This is host/operator durability, not canister stable state. Recovery wording is documented in [Recovery and retry runbooks](recovery-retry-runbooks.md). |
| Delegated-auth replay identity | Delegated-token issuance and verification use shared replay receipts plus issuer canister-signature proofs. Unit tests cover auth replay payload stability, caller/subject binding, and stable replay receipt round trips. | Covered by existing tests/docs | The post-upgrade runtime has one replay model. |
| Delegation proof caller/shard binding | `AuthApi::validate_delegation_request_caller` requires caller and `shard_pid` to match before proof issuance; unit tests assert mismatched callers fail. Replay payload hashing binds authoritative proof payload and excludes metadata. | Covered by existing tests/docs | This prevents replay/cost guards from minting proof material for a foreign shard request. |
| Delegated-token mint and issue replay state | `crates/canic-core/src/api/auth/mod.rs` tests metadata validation, payload-hash stability, committed replay returning the cached token, actor mismatch, payload mismatch, and in-progress duplicate blocking. | Covered by existing tests/docs | Shared replay receipts own token mint replay behavior after the hard cut. |
| ICP refill and value-transfer replay state | `crates/canic-core/src/workflow/ic/icp_refill/tests.rs` covers exact operation ID use, shared replay reserve input, payload hash behavior, terminal committed response replay, resumable response aborts, payload mismatch, value-transfer cost guard, effect marking, in-flight abort, and recovery-required preservation. | Covered by existing tests/docs | Dedicated `IcpRefillRecord` state remains the refill workflow record; shared replay receipts gate replay decisions. |
| Cost-guard accounting and permit boundaries | `CostGuardOps` reserves quota and cycle intents before effects, completes or recovers permits, and uses intent-store totals. Unit tests cover quota exhaustion, low cycle reserve rejection, abort release, and outstanding reservation accounting. Source guards pin private permit construction and permit-required ECDSA, value-transfer, and deployment boundaries. | Covered by existing tests/docs | Cost guard state is intent-store accounting, not a new public schema. |
| Response-idempotent canister upgrade requests | Root replay tests cover upgrade request routing, cached identical replay response, conflict rejection, and cross-variant request ID rejection. `CanisterLifecycleEvent::Upgrade` uses management deployment cost context for actual upgrade installs. | Covered by existing tests/docs | Already-current upgrade skips remain outside deployment quota/cycle reservation. |
| Lifecycle post-upgrade ordering | Root and non-root post-upgrade code initializes compiled config, initializes the memory registry, restores env, then runs post-upgrade runtime hooks before bootstrap scheduling/user hooks. PocketIC lifecycle tests cover phase-correct traps and repeated non-root post-upgrade readiness. | Covered by existing tests/docs | Hook bodies stay synchronous until timers schedule async work. |
| Durable-publication and wasm-store state | Replay-policy tests pin durable-publish entries to wasm-store/template publication surfaces. Control-plane subnet-state tests cover publication-store binding transitions. PocketIC root wasm-store reconcile tests prove post-upgrade preserves the current multi-store release binding. | Covered by existing tests/docs | The publication store is durable root-owned state and is already exercised by post-upgrade reconciliation coverage. |
| Stable-memory ABI boundary | `crates/canic-core/tests/stable_memory_abi_guard.rs` prevents Canic-managed runtime crates from bypassing the managed explicit-key stable-memory ABI. | Covered by existing tests/docs | This protects migration risk by keeping stable memory ownership centralized. |
| Candid, CLI, and JSON/output compatibility | 0.62.2 changes no Candid, CLI, JSON/output, or public API shape. | Covered by this audit | Any future stable output change requires separate approval, changelog/status coverage, and tests. |

## State-Invariant Checklist

- Replay receipt records remain schema-versioned.
- Unsupported replay receipt schemas fail in a controlled way.
- Committed replay receipts remain readable through stable serialization.
- Pending and recovery-required replay receipts remain pending or
  recovery-required through stable serialization.
- Operation IDs remain 32 bytes in runtime replay state.
- CLI-generated high-value operation IDs are written before live send.
- Old delegated-token use markers are not live replay state.
- Delegation proof replay is caller/shard bound.
- Delegated-token mint replay returns committed material without reminting.
- ICP refill replay marks external effects before value-transfer boundaries.
- Costed signing, deployment, value-transfer, and durable-publication paths
  require explicit replay/cost context.
- Post-upgrade lifecycle restores config, memory registry, and env before
  scheduling async bootstrap/user hooks.
- Downgrade behavior remains out of scope.

## Required RC Gates

Use these gates when validating upgrade/state compatibility for RC promotion:

```text
bash scripts/ci/check-upgrade-state-audit.sh
cargo test --locked -p canic-core --test stable_memory_abi_guard -- --nocapture
cargo test --locked -p canic-core storage::stable::replay --lib -- --nocapture
cargo test --locked -p canic-core ops::auth::delegated --lib -- --nocapture
cargo test --locked -p canic-core replay_policy --lib -- --nocapture
cargo test --locked -p canic-core --test cost_guard_boundary_guard -- --nocapture
cargo test --locked -p canic-tests --test lifecycle_boundary -- --test-threads=1 --nocapture
cargo test --locked -p canic-tests --test root_wasm_store_reconcile root_post_upgrade_preserves_multi_store_current_release_binding -- --test-threads=1 --nocapture
```

The `canic-tests` gates are PocketIC-backed and may be assigned to CI or an RC
validation environment if too expensive for an ordinary docs slice.

## Non-Goals

- No runtime behavior change.
- No Candid change.
- No CLI output change.
- No JSON/output format change.
- No dependency or lockfile change.
- No broad refactor.
- No downgrade compatibility claim.
- No migration shim for old delegated-token use markers.
- No retroactive replay protection for operations completed before 0.61.

## Outcome Summary

Release blockers: none found in this audit.

The current evidence is sufficient to continue 0.62 without opening another
runtime implementation slice. Recovery/runbook wording for pending or
recovery-required states is documented in
[Recovery and retry runbooks](recovery-retry-runbooks.md). Remaining work
belongs to diagnostic consistency review, package/install validation, or RC
accounting.
