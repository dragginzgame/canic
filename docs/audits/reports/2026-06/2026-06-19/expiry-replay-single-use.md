# Expiry Replay Single-Use Invariant Audit - 2026-06-19

## Report Preamble

- Definition path:
  `docs/audits/recurring/invariants/expiry-replay-single-use.md`
- Scope: delegated-token bearer freshness, active root delegation proof
  install/status freshness, root delegation proof batch prepare/get/install
  replay and expiry semantics, replay policy inventory, root replay capacity
  ordering, capability replay metadata expiry, and root replay record expiry.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-29/expiry-replay-single-use.md`
- Code snapshot identifier: `16894709` with dirty worktree.
- Method tag/version: `Method V4.4`.
- Comparability status: `non-comparable`: the live audit definition was
  updated for the post-0.68 root proof provisioning model and now requires the
  current root proof batch replay/idempotency, active proof status, and
  verifier-local bearer-token statelessness checks. Core expiry-boundary checks
  remain comparable as mechanical context.
- Exclusions applied: generated output, target artifacts, `.icp` runtime cache,
  broad clippy/release validation, and unrelated dirty Rust edits outside the
  checked freshness/replay paths.
- Auditor: `codex`.
- Run timestamp: `2026-06-19`.
- Worktree: `dirty`; unrelated source edits were left untouched.

## Executive Summary

Verdict: **PASS**.

No expiry, replay, or single-use invariant break was found. Delegated tokens
remain TTL-bounded bearer credentials without verifier-local use state. Root
proof provisioning now adds a fresh replay-sensitive lane: batch prepare is
idempotency protected by `AuthRequestMetadata.request_id`, request fingerprint,
and bounded replay TTL; batch get/install reject expired retrieval windows,
expired certificates, proof mismatches, and stale pending metadata.

Overall risk score: **3 / 10**. The invariant holds, with moderate residual
pressure from broad delegated-token fan-in and recent root proof provisioning
churn.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Delegated-token verifier does not write token-use state | PASS | `delegated_auth_guard_has_no_verifier_local_use_store` passed. |
| Delegated-token cert/claims reject at expiry boundary | PASS | `verify_delegated_token_rejects_expired_token_at_boundary` passed; `verify_cert_time` and `verify_claims` use `now_ns >= expires_at_ns`. |
| Active root proof install rejects not-yet-valid/expired certs | PASS | `install_active_delegation_proof_rejects_time_bounds` passed. |
| Active root proof status reports valid/refresh-needed/expired states | PASS | `active_delegation_proof_status` filtered tests passed. |
| Root proof batch prepare is request-id replay protected | PASS | `batch_prepare_replays_same_request_id_without_resigning` and `batch_prepare_rejects_conflicting_request_id_reuse` passed. |
| Root proof batch get rejects expired pending metadata | PASS | `batch_get` filtered tests passed, including expired pending metadata. |
| Root proof batch install rejects proof mismatch/stale metadata | PASS | `batch_install` filtered tests passed, including stale pending metadata. |
| Expired pending root proof metadata and replay entries are pruned | PASS | `pending_batch_cleanup` filtered tests passed. |
| Endpoint replay policy inventory covers current endpoints | PASS | `replay_policy` filtered tests passed, 27 tests. |
| Root replay per-caller capacity precedes global capacity | PASS | `reserve_root_replay_rejects_caller_capacity_before_global_capacity` passed. |
| Root replay records expire at exact boundary | PASS | `evaluate_root_replay_returns_expired_at_expiry_boundary` passed. |
| Capability replay metadata rejects expired/exact-boundary/future-over-skew inputs | PASS | `project_replay_metadata_rejects` filtered tests passed, 3 tests. |
| Stale direct `now > expires_at` boundary checks | PASS | `rg -n "now_ns > .*expires_at|now_secs > .*expires_at|now > .*expires_at|> grant\\.expires_at|> payload\\.expires_at" crates/canic-core/src -g '*.rs'` returned no matches. |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_cert_time`, `verify_claims` | canonical delegated-token certificate and claim freshness checks | High |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | endpoint delegated-token bearer verification boundary | High |
| `crates/canic-core/src/ops/auth/delegated/active_proof.rs` | `install_active_delegation_proof` | signer active root proof install time-window gate | High |
| `crates/canic-core/src/ops/auth/delegation/mod.rs` | batch prepare/get/install helpers | root proof batch replay, retrieval expiry, cert expiry, and cleanup state | High |
| `crates/canic-core/src/replay_policy/endpoint_manifest.rs` | `ENDPOINT_REPLAY_POLICY_MANIFEST` | canonical endpoint replay classification inventory | High |
| `crates/canic-core/src/ops/replay/guard.rs` | `evaluate_root_replay` | duplicate/conflict/TTL root replay decision surface | High |
| `crates/canic-core/src/ops/replay/mod.rs` | `reserve_root_replay` | replay marker reservation and per-caller capacity ordering | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/replay.rs` | replay preflight/commit/abort helpers | root capability replay workflow integration | Medium |
| `crates/canic-core/src/workflow/rpc/capability/replay.rs` | `project_replay_metadata` | capability replay metadata projection and skew/expiry checks | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| delegated-token freshness lane | `rg -l "DelegatedToken|delegated_token|verify_delegated_token" ...` found 42 files | 6 | 4 | 7 |
| root proof provisioning lane | `rg -l "ActiveDelegationProof|RootDelegationProof|RootDelegationProofBatch|retrieval_expires_at_ns|replay_expires_at_ns|refresh_after_ns" ...` found 21 files | 7 | 4 | 7 |
| root replay/capability lane | `rg -l "ReplayDecision|RootRequestMetadata|AuthRequestMetadata|reserve_root_replay|evaluate_root_replay|request_id" ...` found 21 files | 5 | 3 | 6 |
| replay policy inventory | `cargo test --locked -p canic-core --lib replay_policy -- --nocapture` ran 27 tests | 3 | 2 | 6 |
| active proof storage/status lane | `rg -l "ActiveDelegationProof" ...` found 11 files | 5 | 3 | 5 |

## Risk Score

Risk Score: **3 / 10**.

Score basis:

- `+0` for confirmed expiry/replay/single-use breaks: none found.
- `+1` delegated-token and active-proof freshness remain high-impact shared
  auth seams.
- `+1` root proof provisioning replay/retrieval/install state is new and has
  recent edit pressure through the 0.68 line.
- `+1` fan-in pressure: delegated-token freshness terms appear in 42 files, and
  root proof provisioning terms appear in 21 files.

Verdict: invariant holds with moderate residual fan-in pressure.

## Amplification Drivers

- Delegated-token bearer freshness spans access guards, DTOs, ops verification,
  cache/proof helpers, macro endpoints, and protocol tests.
- Root proof provisioning added replay-sensitive prepare state, query retrieval
  windows, install preflight, cleanup, and active proof status in one release
  slice.
- Replay policy inventory now guards more auth endpoints, including proof
  batch prepare/get/install and active proof install/status behavior.
- Recent history shows repeated edits to `ops/auth/delegation/mod.rs` through
  `0.68.7` to `0.68.23`, so exact-boundary tests matter.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| root proof provisioning freshness fan-in | `ops/auth/delegation/mod.rs`, `dto/auth.rs`, `api/auth/mod.rs`, macro endpoints, provisioning workflow | root proof provisioning scan found 21 files | Medium |
| delegated-token freshness fan-in | access/auth, ops/auth/delegated, DTOs, tests, macros | delegated-token scan found 42 files | Medium |
| stale expiry-boundary drift | core freshness paths | direct stale `now > expires_at` scan returned no matches | Low |
| non-fatal lint expectation warnings during focused tests | `crates/canic-core/src/ops/runtime/metrics/delegated_auth.rs` | cargo test emitted four `unfulfilled_lint_expectations` warnings | Low for this invariant; separate clippy hygiene concern |
| recent replay/provisioning edit pressure | `git log --name-only -n 20 -- ...` | repeated `ops/auth/delegation/mod.rs`, replay, and replay-policy commits across 0.68 | Medium |

### Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `ReplayDecision` | `crates/canic-core/src/ops/replay/guard.rs` | 3 | Low |
| `ActiveDelegationProofStatus` | `crates/canic-core/src/dto/auth.rs` | included in 11-file active proof scan | Medium |
| `RootDelegationProofInstallOutcome` | `crates/canic-core/src/dto/auth.rs` | included in root proof provisioning lane | Medium |

### Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `AuthRequestMetadata` | `crates/canic-core/src/dto/auth.rs` | dto/api/ops/workflow/tests | Medium |
| `RootRequestMetadata` | `crates/canic-core/src/dto/rpc.rs` | dto/ops/workflow/tests | Medium |
| `ActiveDelegationProof` | `crates/canic-core/src/dto/auth.rs` | dto/api/ops/storage/workflow/tests | Medium |
| `RootDelegationProofBatch*` DTOs | `crates/canic-core/src/dto/auth.rs` | dto/api/ops/workflow/macros/tests | Medium |

### Growing Hub Modules

| Module | Subsystems Imported | Recent Commits | Risk |
| --- | --- | ---: | --- |
| `crates/canic-core/src/ops/auth/delegation/mod.rs` | ops/auth, dto/auth, policy/auth, workflow/runtime provisioning, tests | 11 hits in scoped recent history | Medium |
| `crates/canic-core/src/replay_policy/**` | replay policy manifests, cost guards, endpoint tests | 4 scoped recent commits | Medium |
| `crates/canic-core/src/ops/replay/**` | ops replay, workflow request handler, model/storage replay | 4 scoped recent commits | Medium |

### Capability Surface Growth

| Module | Public Items | Risk |
| --- | ---: | --- |
| `crates/canic-core/src/dto/auth.rs` | broad auth/proof DTO set including active proof and batch proof contracts | Medium |
| `crates/canic-core/src/ops/auth/delegation/mod.rs` | mostly internal ops helpers plus AuthOps facade methods | Medium |

## Dependency Fan-In Pressure

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| delegated token / delegated-token verifier group | 42 | access, api, dto, ops, macros, tests | Architectural gravity well |
| root proof provisioning group | 21 | dto, api, ops, storage, workflow, macros, tests | Architectural gravity well |
| root replay / request-id group | 21 | dto, ops, workflow, replay policy, tests | Hub forming |
| `RootRequestMetadata` / `AuthRequestMetadata` | 15 | dto, ops, workflow, tests | Architectural gravity well |
| `ReplayDecision` | 3 | ops, workflow | Normal |
| `ReplaySlotKey` | 0 | none in current tree | Removed/no current pressure |

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test --locked -p canic-core --lib delegated_auth_guard_has_no_verifier_local_use_store -- --nocapture` | PASS | Delegated-token guard has no verifier-local token-use store. |
| `cargo test --locked -p canic-core --lib verify_delegated_token_rejects_expired_token_at_boundary -- --nocapture` | PASS | Delegated-token exact expiry boundary rejects. |
| `cargo test --locked -p canic-core --lib install_active_delegation_proof_rejects_time_bounds -- --nocapture` | PASS | Active proof install rejects not-yet-valid/expired certificates. |
| `cargo test --locked -p canic-core --lib active_delegation_proof_status -- --nocapture` | PASS | Active proof status reports missing, valid, refresh-needed, and expired states. |
| `cargo test --locked -p canic-core --lib batch_prepare_replays_same_request_id_without_resigning -- --nocapture` | PASS | Same request id and same fingerprint returns cached metadata. |
| `cargo test --locked -p canic-core --lib batch_prepare_rejects_conflicting_request_id_reuse -- --nocapture` | PASS | Same request id with different request fingerprint rejects. |
| `cargo test --locked -p canic-core --lib batch_get -- --nocapture` | PASS | Includes expired pending metadata rejection. |
| `cargo test --locked -p canic-core --lib batch_install -- --nocapture` | PASS | Includes proof mismatch and stale pending metadata rejection. |
| `cargo test --locked -p canic-core --lib pending_batch_cleanup -- --nocapture` | PASS | Expired pending metadata/replay cleanup covered. |
| `cargo test --locked -p canic-core --lib replay_policy -- --nocapture` | PASS | 27 replay policy inventory tests passed. |
| `cargo test --locked -p canic-core --lib reserve_root_replay_rejects_caller_capacity_before_global_capacity -- --nocapture` | PASS | Per-caller replay cap checked before global cap. |
| `cargo test --locked -p canic-core --lib evaluate_root_replay_returns_expired_at_expiry_boundary -- --nocapture` | PASS | Existing root replay records expire at exact boundary. |
| `cargo test --locked -p canic-core --lib project_replay_metadata_rejects -- --nocapture` | PASS | Expired, exact-boundary expired, and future-over-skew capability metadata reject. |
| `rg -n "now_ns > .*expires_at\|now_secs > .*expires_at\|now > .*expires_at\|> grant\\.expires_at\|> payload\\.expires_at" crates/canic-core/src -g '*.rs'` | PASS | No stale direct `>` expiry-boundary checks found. |
| `git diff --check` | PASS | No whitespace errors. |

Focused cargo tests emitted the same four non-fatal `unfulfilled_lint_expectations`
warnings in `crates/canic-core/src/ops/runtime/metrics/delegated_auth.rs`.
Those warnings are outside the expiry/replay invariant and should be handled in
a lint/hygiene pass if still present under clippy `-D warnings`.

## Follow-up Actions

1. Keep delegated-token verifier state stateless; domain replay should remain
   in operation receipts or request-id replay mechanisms.
2. Keep root proof batch prepare keyed by request id plus request fingerprint.
3. Keep root proof batch get/install rejecting expired retrieval windows,
   expired certs, proof mismatches, and stale pending metadata.
4. Monitor `ops/auth/delegation/mod.rs` and `dto/auth.rs` fan-in as the 0.68 root
   proof provisioning slice stabilizes.
