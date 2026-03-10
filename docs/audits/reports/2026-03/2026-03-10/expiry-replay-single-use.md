# Expiry Replay Single-Use Invariant Audit - 2026-03-10

## Report Preamble

- Scope: freshness controls (`exp/nbf/replay/single-use`) in auth + replay pipeline
- Compared baseline report path: `N/A` (first run for this scope on 2026-03-10)
- Code snapshot identifier: `fa06bfef`
- Method tag/version: `Method V4.0`
- Comparability status: `non-comparable` (method expanded with hotspots, predictive signals, fan-in pressure)
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-10T14:30:36Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Expiry/nbf checks are centralized | PASS | `ops/auth/verify.rs::verify_time_bounds` |
| Replay checks are enforced where required | PASS | `ops/replay/guard.rs` + workflow replay preflight path |
| Freshness state transitions are explicit | PASS | reserve/commit/abort flows in `ops/replay/mod.rs` |
| Atomicity expectation represented in flow | PASS | replay commit occurs with protected action completion path |
| Replay/freshness tests present | PASS | replay unit tests and integration tests in `canic-core` + `canic/tests/root_replay.rs` |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/verify.rs` | `verify_time_bounds` | canonical token freshness check | High |
| `crates/canic-core/src/ops/replay/guard.rs` | replay evaluation functions | duplicate/conflict/expiry decision surface | High |
| `crates/canic-core/src/workflow/rpc/request/handler/replay.rs` | replay preflight orchestration | freshness gate integration with workflow | Medium |
| `crates/canic-core/src/ops/replay/mod.rs` | commit/abort functions | single-use state transitions | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/ops/replay/guard.rs` | `ops, storage, workflow` | 3 | 1 | 6 |
| `crates/canic-core/src/workflow/rpc/request/handler/replay.rs` | `workflow, ops, dto` | 3 | 2 | 6 |
| `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` | `workflow, ops, dto` | 3 | 2 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| growing hub module | `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` | touched in `8` recent commits and appears in top amplification slices | Medium |
| enum shock radius | `crates/canic-core/src/dto/rpc.rs` | replay-related request/response enums referenced in `14`/`13` files | Medium |
| cross-layer struct spread | `ReplaySlotKey` | referenced in `7` files across `ops/workflow` | Low |

## Dependency Fan-In Pressure

### Module Fan-In

No fan-in pressure detected in this run.

### Struct Fan-In

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `ReplaySlotKey` | `crates/canic-core/src/ops/replay/key.rs` | 7 | Low |
| `RoleAttestation` | `crates/canic-core/src/dto/auth.rs` | 15 | Medium |

## Risk Score

Risk Score: **3 / 10**

Freshness invariants currently hold. Low risk remains from replay/workflow coupling pressure and repeated edits in replay-adjacent orchestration modules.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib --locked` | PASS | replay + freshness unit tests passed |
| `cargo test -p canic --test delegation_flow --locked` | PASS | token expiry rejection path passed |
| `cargo test -p canic --test lifecycle_boundary --locked` | PASS | unrelated guardrail still green |

## Follow-up Actions

1. Keep replay reserve/commit/abort sequence stable during workflow refactors.
2. Re-run this audit after any replay key or TTL policy changes.
