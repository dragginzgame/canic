# Expiry Replay Single-Use Invariant Audit - 2026-06-28

## Report Preamble

- Definition path:
  `docs/audits/recurring/invariants/expiry-replay-single-use.md`
- Scope: delegated-token bearer freshness, active delegation proof install and
  status freshness, root delegation proof batch prepare/get/install replay and
  expiry semantics, root-managed renewal retrieval/install gates, replay
  policy inventory, root replay capacity ordering, capability replay metadata
  expiry, and root replay receipt expiry.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/expiry-replay-single-use.md`
- Code snapshot identifier: `b140a86c` with dirty worktree.
- Method tag/version: `Method V4.4 / root-renewal split refresh`.
- Comparability status: `comparable`. The audit definition and core
  freshness/replay checks match the June 19 run; current notes are
  path-adjusted for the root-renewal directory-module split and added
  root-managed renewal tests.
- Exclusions applied: generated output, target artifacts, `.icp` runtime
  cache, broad clippy/release validation, historical audit reports except as
  baselines, and unrelated dirty source/changelog edits.
- Auditor: `codex`.
- Run timestamp: `2026-06-28T14:42:40Z`.
- Worktree: `dirty`; existing changelog, root-renewal, blob-storage, and
  audit-report edits were preserved.

Verification status: **PASS**.

No expiry, replay, or single-use invariant break was found. No source cleanup
was applied from this audit.

## Executive Summary

Verdict: **PASS**.

Delegated tokens remain TTL-bounded bearer credentials without verifier-local
token-use state. Delegated-token cert and claims checks still reject at the
exact expiry boundary with `now_ns >= expires_at_ns`. Active delegation proof
install rejects not-yet-valid and expired certs, and active proof status still
uses explicit `refresh_after_ns` and `expires_at_ns` boundaries.

Root proof batch prepare remains idempotency-protected by
`AuthRequestMetadata.request_id`, request fingerprint, and bounded replay TTL.
Batch get/install reject expired retrieval windows, expired certs, stale
pending metadata, and proof mismatches. The current root-renewal split adds
scheduled renewal retrieval/install freshness gates but does not introduce a
second replay owner.

Overall risk score: **3 / 10**. The invariant holds, with moderate residual
fan-in pressure from delegated-token, root proof, root-renewal, and replay DTO
surfaces.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Delegated-token verifier does not write token-use state | PASS | `delegated_auth_guard_has_no_verifier_local_use_store` passed; scan found nonce generation/canonicalization but no production token-use store. |
| Delegated-token cert/claims reject at expiry boundary | PASS | `verify_delegated_token_rejects_expired_token_at_boundary` passed; `verify_cert_time` and `verify_claims` use `now_ns >= expires_at_ns`. |
| Active proof install rejects not-yet-valid/expired certs | PASS | `install_active_delegation_proof_rejects_time_bounds` passed. |
| Active proof status reports valid/refresh-needed/expired states | PASS | `active_delegation_proof_status` filtered tests passed. |
| Root proof batch prepare is request-id replay protected | PASS | `batch_prepare_replays_same_request_id_without_resigning` and `batch_prepare_rejects_conflicting_request_id_reuse` passed. |
| Root proof batch get rejects expired or missing pending metadata | PASS | `batch_get` filtered tests passed, including root-renewal scheduled retrieval tests. |
| Root proof batch install rejects proof mismatch/stale metadata | PASS | `batch_install` filtered tests passed, including root-renewal scheduled install gate tests. |
| Expired pending root proof metadata and replay entries are pruned | PASS | `pending_batch_cleanup` filtered tests passed. |
| Endpoint replay policy inventory covers current endpoints | PASS | `replay_policy` filtered tests passed, 28 tests. |
| Root replay per-caller capacity precedes global capacity | PASS | `reserve_root_replay_rejects_caller_capacity_before_global_capacity` passed. |
| Root replay records expire at exact boundary | PASS | `evaluate_root_replay_returns_expired_at_expiry_boundary` passed. |
| Capability replay metadata rejects expired/exact-boundary/future-over-skew inputs | PASS | `project_replay_metadata_rejects` filtered tests passed, 3 tests. |
| Stale direct `now > expires_at` boundary checks | PASS | Direct scan for stale `>` expiry-boundary patterns in `crates/canic-core/src` returned no matches. |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_cert_time`, `verify_claims` | canonical delegated-token certificate and claim freshness checks | High |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | endpoint delegated-token bearer verification boundary | High |
| `crates/canic-core/src/ops/auth/delegated/active_proof.rs` | `install_active_delegation_proof` | signer active root proof install time-window gate | High |
| `crates/canic-core/src/ops/auth/delegation/batch.rs` | batch prepare/get/install helpers | root proof batch replay, retrieval expiry, cert expiry, and install preflight | High |
| `crates/canic-core/src/ops/auth/delegation/pending.rs` | pending batch replay/cache/cleanup helpers | request-id replay fingerprinting and pending metadata TTL pruning | High |
| `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/retrieval.rs` | scheduled renewal proof batch retrieval | scheduled renewal retrieval and install-deadline gates | Medium |
| `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/install.rs` | scheduled/manual renewal install recording | scheduled proof freshness, template fingerprint, and install deadline checks | Medium |
| `crates/canic-core/src/replay_policy/endpoint_manifest.rs` | `ENDPOINT_REPLAY_POLICY_MANIFEST` | canonical endpoint replay classification inventory | High |
| `crates/canic-core/src/ops/replay/guard.rs` | `evaluate_root_replay` | duplicate/conflict/TTL root replay decision surface | High |
| `crates/canic-core/src/ops/replay/mod.rs` | `reserve_root_replay` | replay marker reservation and per-caller capacity ordering | Medium |
| `crates/canic-core/src/workflow/rpc/capability/replay.rs` | `project_replay_metadata` | capability replay metadata projection and skew/expiry checks | Medium |

## Hub Module Pressure

| Module / Lane | Import / Scan Signal | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| delegated-token freshness lane | `rg -l "DelegatedToken|delegated_token|verify_delegated_token" ...` found 44 files | 6 | 4 | 7 |
| root proof and active proof lane | `rg -l "ActiveDelegationProof|RootDelegationProof|RootDelegationProofBatch|retrieval_expires_at_ns|replay_expires_at_ns|refresh_after_ns" ...` found 34 files | 7 | 4 | 7 |
| root replay/request-id lane | `rg -l "ReplayDecision|RootRequestMetadata|AuthRequestMetadata|reserve_root_replay|evaluate_root_replay|request_id" ...` found 30 files | 5 | 3 | 6 |
| replay policy inventory | `cargo test --locked -p canic-core --lib replay_policy -- --nocapture` ran 28 tests | 3 | 2 | 6 |
| active proof storage/status lane | filtered scan around `ActiveDelegationProofStatus` and active proof storage | 5 | 3 | 5 |

## Risk Score

Risk Score: **3 / 10**.

Score basis:

- `+0` for confirmed expiry/replay/single-use breaks: none found.
- `+1` delegated-token and active-proof freshness remain high-impact shared
  auth seams.
- `+1` root proof batch prepare/get/install and root-renewal scheduled
  retrieval/install remain replay- and freshness-sensitive.
- `+1` fan-in pressure increased since the June 19 baseline: delegated-token
  terms appear in 44 files, root proof/active proof terms in 34, and
  replay/request-id terms in 30.

Verdict: invariant holds with moderate residual fan-in pressure.

## Amplification Drivers

- Delegated-token bearer freshness spans access guards, DTOs, ops
  verification, cache/proof helpers, macro endpoints, and tests.
- Root proof provisioning and root-managed renewal now share a freshness
  neighborhood: request-id replay, retrieval windows, cert expiry, install
  deadlines, template fingerprints, and active proof status all move around
  auth delegation.
- Replay policy inventory continues to guard proof batch prepare/get/install,
  active proof install/status, renewal config upserts, and root capability
  command dispatch.
- Recent scoped history shows repeated edits through the 0.74 root-renewal
  line, so exact-boundary tests remain important.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| root proof and renewal freshness fan-in | `ops/auth/delegation/*`, `dto/auth.rs`, `api/auth/mod.rs`, macros, tests | root proof/active proof scan found 34 files | Medium |
| delegated-token freshness fan-in | access/auth, ops/auth/delegated, DTOs, macros, tests | delegated-token scan found 44 files | Medium |
| replay/request-id fan-in | DTOs, ops replay, workflow RPC, replay policy, tests | replay/request-id scan found 30 files | Medium |
| stale expiry-boundary drift | core freshness paths | direct stale `now > expires_at` scan returned no matches | Low |
| recent replay/provisioning edit pressure | scoped `git log --name-only -n 20` | repeated 0.74 root-renewal and 0.68 replay/provisioning commits | Medium |

### Enum Shock Radius

| Enum | Defined In | Reference Surface | Risk |
| --- | --- | --- | --- |
| `ReplayDecision` | `crates/canic-core/src/ops/replay/guard.rs` | ops replay and workflow replay handler | Low |
| `ActiveDelegationProofStatus` | `crates/canic-core/src/dto/auth.rs` | API, ops active status, macro endpoints, tests | Medium |
| `RootDelegationProofInstallOutcome` | `crates/canic-core/src/dto/auth.rs` | ops preflight, provisioning workflow, PocketIC root cases, tests | Medium |

### Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `AuthRequestMetadata` | `crates/canic-core/src/dto/auth.rs` | dto/api/ops/workflow/tests | Medium |
| `RootRequestMetadata` | `crates/canic-core/src/dto/rpc.rs` | dto/ops/workflow/tests | Medium |
| `ActiveDelegationProof` | `crates/canic-core/src/dto/auth.rs` | dto/api/ops/storage/workflow/tests | Medium |
| `RootDelegationProofBatch*` DTOs | `crates/canic-core/src/dto/auth.rs` | dto/api/ops/workflow/macros/tests | Medium |

### Growing Hub Modules

| Module | Subsystems Imported / Recent Signal | Risk |
| --- | --- | --- |
| `crates/canic-core/src/ops/auth/delegation/*` | batch, pending, active, root issuer policy, and root-renewal children all participate in proof freshness | Medium |
| `crates/canic-core/src/replay_policy/**` | endpoint, pool-admin, root-capability, cost, and coverage manifests tested by 28 replay-policy cases | Medium |
| `crates/canic-core/src/ops/replay/**` | root replay guard, receipt, slot reservation, expiry, and capacity ordering | Medium |

## Dependency Fan-In Pressure

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| delegated token / delegated-token verifier group | 44 | access, api, dto, ops, macros, tests | Architectural gravity well |
| root proof / active proof / renewal group | 34 | dto, api, ops, storage, workflow, macros, tests | Architectural gravity well |
| root replay / request-id group | 30 | dto, ops, workflow, replay policy, tests | Hub forming |
| `RootRequestMetadata` / `AuthRequestMetadata` | included in replay/request-id scan | dto, ops, workflow, tests | Architectural gravity well |
| `ReplayDecision` | narrow ops/workflow surface | ops, workflow | Normal |

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test --locked -p canic-core --lib delegated_auth_guard_has_no_verifier_local_use_store -- --nocapture` | PASS | Delegated-token guard has no verifier-local token-use store. |
| `cargo test --locked -p canic-core --lib verify_delegated_token_rejects_expired_token_at_boundary -- --nocapture` | PASS | Delegated-token exact expiry boundary rejects. |
| `cargo test --locked -p canic-core --lib install_active_delegation_proof_rejects_time_bounds -- --nocapture` | PASS | Active proof install rejects not-yet-valid/expired certificates. |
| `cargo test --locked -p canic-core --lib active_delegation_proof_status -- --nocapture` | PASS | Active proof status reports missing, valid, refresh-needed, and expired states. |
| `cargo test --locked -p canic-core --lib batch_prepare_replays_same_request_id_without_resigning -- --nocapture` | PASS | Same request id and same fingerprint returns cached metadata. |
| `cargo test --locked -p canic-core --lib batch_prepare_rejects_conflicting_request_id_reuse -- --nocapture` | PASS | Same request id with different request fingerprint rejects. |
| `cargo test --locked -p canic-core --lib batch_get -- --nocapture` | PASS | Includes expired pending metadata and root-renewal scheduled retrieval tests. |
| `cargo test --locked -p canic-core --lib batch_install -- --nocapture` | PASS | Includes proof mismatch, stale pending metadata, and scheduled renewal install-gate tests. |
| `cargo test --locked -p canic-core --lib pending_batch_cleanup -- --nocapture` | PASS | Expired pending metadata/replay cleanup covered. |
| `cargo test --locked -p canic-core --lib replay_policy -- --nocapture` | PASS | 28 replay policy inventory tests passed. |
| `cargo test --locked -p canic-core --lib reserve_root_replay_rejects_caller_capacity_before_global_capacity -- --nocapture` | PASS | Per-caller replay cap checked before global cap. |
| `cargo test --locked -p canic-core --lib evaluate_root_replay_returns_expired_at_expiry_boundary -- --nocapture` | PASS | Existing root replay records expire at exact boundary. |
| `cargo test --locked -p canic-core --lib project_replay_metadata_rejects -- --nocapture` | PASS | Expired, exact-boundary expired, and future-over-skew capability metadata reject. |
| `rg -n "now_ns > .*expires_at\|now_secs > .*expires_at\|now > .*expires_at\|> grant\\.expires_at\|> payload\\.expires_at" crates/canic-core/src -g '*.rs'` | PASS | No stale direct `>` expiry-boundary checks found. |

## Follow-up Actions

1. Keep delegated-token verifier state stateless; domain replay should remain
   in operation receipts or request-id replay mechanisms.
2. Keep root proof batch prepare keyed by request id plus request fingerprint.
3. Keep root proof batch get/install rejecting expired retrieval windows,
   expired certs, proof mismatches, and stale pending metadata.
4. Keep root-renewal scheduled retrieval/install gates tied to
   retrieval expiry, install deadlines, prepared cert expiry, and template
   fingerprints.
5. Monitor `ops/auth/delegation/*` and `dto/auth.rs` fan-in as the root
   renewal line stabilizes.
