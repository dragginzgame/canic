# Expiry Replay Single-Use Invariant Audit - 2026-04-05

## Report Preamble

- Scope: delegated-token freshness, delegated-session bootstrap replay policy, root capability replay metadata, and shared replay-store semantics
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-10/expiry-replay-single-use.md`
- Code snapshot identifier: `c027b8df`
- Method tag/version: `Method V4.1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T16:48:44Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Expiry checks are still centralized | PASS | `verify_time_bounds(...)` remains the canonical freshness gate in `crates/canic-core/src/ops/auth/verify/token_chain.rs`, and delegated-token verification still flows through `crates/canic-core/src/ops/auth/token.rs`. |
| Replay checks are enforced where required | PASS | root/non-root capability ingress still projects replay metadata in `crates/canic-core/src/api/rpc/capability/replay.rs` before the workflow replay path in `crates/canic-core/src/workflow/rpc/request/handler/replay.rs` and `crates/canic-core/src/workflow/rpc/request/handler/nonroot_cycles.rs`. |
| Single-use / replay policy is explicit for delegated-session bootstrap | PASS | delegated-session bootstrap still routes through `enforce_bootstrap_replay_policy(...)` in `crates/canic-core/src/api/auth/session/mod.rs`, with idempotent same-wallet reuse allowed only while the matching active session exists and rejected after clear/conflict. |
| Replay state transitions are explicit | PASS | shared replay reservation/commit/abort still live in `crates/canic-core/src/ops/replay/mod.rs`, with deterministic slot state held in `crates/canic-core/src/storage/stable/replay.rs`. |
| Clock skew and TTL bounds remain bounded | PASS | `project_replay_metadata(...)` rejects expired and over-skew metadata in `crates/canic-core/src/api/rpc/capability/replay.rs`, and replay TTL validation still lives in `crates/canic-core/src/ops/replay/ttl.rs`. |
| Current tests prove reject-on-expiry and reject-on-reuse | PASS | PocketIC coverage in `crates/canic-core/tests/pic_role_attestation.rs` proves idempotent same-token replay, rejection after clear, rejection on wallet conflict, and failure-closed behavior for expired replay. |

## Comparison to Previous Relevant Run

- Stable: expiry and replay checks still converge on canonical auth/replay seams instead of being duplicated across endpoints.
- Stable: reserve/commit/abort replay state transitions remain explicit and separate from policy decisions.
- Improved: delegated-session bootstrap replay semantics are now stronger than the March baseline because the current PocketIC suite proves idempotent same-session reuse, clear-and-reject behavior, and conflict rejection in one runtime path.
- Stable: root capability replay metadata still rejects expired inputs before workflow execution and still binds nonce into the derived request identity.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/verify/token_chain.rs` | `verify_time_bounds` | canonical delegated-token expiry / lifetime gate | High |
| `crates/canic-core/src/api/auth/session/mod.rs` | `enforce_bootstrap_replay_policy` | delegated-session single-use / idempotence owner | High |
| `crates/canic-core/src/ops/replay/guard.rs` | `evaluate_root_replay`, `ReplayDecision` | duplicate/conflict/expired/in-flight replay classification | High |
| `crates/canic-core/src/ops/replay/mod.rs` | `reserve_root_replay`, `commit_root_replay`, `abort_root_replay` | replay state transition seam | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/replay.rs` | root replay preflight/commit flow | shared replay orchestration for root responses | Medium |
| `crates/canic-core/src/api/rpc/capability/replay.rs` | `project_replay_metadata` | expiry/skew/request-id projection before capability execution | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/api/auth/session/mod.rs` | `api, ops, storage, metrics` | 4 | 2 | 6 |
| `crates/canic-core/src/ops/replay/guard.rs` | `ops, storage, ids` | 3 | 1 | 6 |
| `crates/canic-core/src/workflow/rpc/request/handler/replay.rs` | `workflow, ops, dto, metrics` | 4 | 2 | 6 |
| `crates/canic-core/src/api/rpc/capability/replay.rs` | `api, dto, workflow` | 3 | 2 | 5 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| growing hub module | `crates/canic-core/src/api/auth/session/mod.rs` | owns bootstrap set/clear/prune plus replay policy and metrics hooks | Medium |
| growing hub module | `crates/canic-core/src/workflow/rpc/request/handler/replay.rs` | replay orchestration was touched across the `0.24.1` to `0.24.3` auth/capability line | Medium |
| enum shock radius | `crates/canic-core/src/ops/replay/guard.rs` | `ReplayDecision` and `ReplayPending` appear in `6` files across ops/workflow/tests | Medium |
| cross-layer struct spread | `ReplaySlotKey` | referenced in `7` files across storage/ops/workflow/tests | Low |
| capability surface growth | `crates/canic-core/src/api/auth/session/mod.rs` | `4` public functions; still controlled, but this is the clearest place a weaker replay bypass could appear | Low |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| replay guard lane | 6 | `ops/workflow/tests` | Rising pressure |
| root replay workflow lane | 5 | `workflow/tests` | Rising pressure |

### Struct Fan-In

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `ReplaySlotKey` | `crates/canic-core/src/storage/stable/replay.rs` | 7 | Low |
| `RoleAttestation` | `crates/canic-core/src/dto/auth.rs` | 35 | High |

## Risk Score

Risk Score: **4 / 10**

Score contributions:
- `+2` replay/auth hotspots remain concentrated in a few shared seams
- `+1` delegated-session bootstrap is a sensitive alternate ingress, even though it currently enforces the same freshness rules
- `+1` the broader auth DTO surface still has high reference radius

Verdict: **Invariant holds with low-but-real coupling pressure.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --test pic_role_attestation delegated_session_bootstrap_replay_policy_and_metrics -- --test-threads=1 --nocapture` | PASS | proves idempotent same-wallet replay, rejection after clear, and conflict rejection |
| `cargo test -p canic-core --test pic_role_attestation delegated_session_bootstrap_replay_with_expired_token_fails_closed -- --test-threads=1 --nocapture` | PASS | expired replay fails closed |
| `cargo test -p canic-core --lib project_replay_metadata_rejects_expired_metadata -- --nocapture` | PASS | replay metadata expiry is enforced before workflow execution |
| `cargo test -p canic-core --lib clamp_delegated_session_expires_at_rejects_expired_token -- --nocapture` | PASS | delegated-session TTL clamp still rejects already-expired tokens |
| `rg -n "verify_time_bounds|enforce_bootstrap_replay_policy|evaluate_root_replay|reserve_root_replay|commit_root_replay|abort_root_replay|project_replay_metadata" crates/canic-core/src -g '*.rs'` | PASS | canonical freshness/replay seams remain narrow and explicit |

## Follow-up Actions

1. Keep watching `crates/canic-core/src/api/auth/session/mod.rs`; it is still the most sensitive non-endpoint replay ingress.
2. Re-run this audit after any replay TTL, request-id derivation, or delegated-session bootstrap change.
