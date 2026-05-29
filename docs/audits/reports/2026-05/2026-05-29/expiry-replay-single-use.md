# Expiry Replay Single-Use Invariant Audit - 2026-05-29

## Report Preamble

- Definition path:
  `docs/audits/recurring/invariants/expiry-replay-single-use.md`
- Scope: delegated-token freshness, update-token single-use consumption,
  query-token statelessness, consumed-token pruning, root capability replay
  metadata, root replay cache expiry, delegated-grant expiry, and attestation
  expiry boundaries
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-09/expiry-replay-single-use.md`
- Code snapshot identifier: `9b435cac`
- Method tag/version: `Method V4.3`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp: `2026-05-29`
- Branch: `main`
- Worktree: `dirty`

## Audit Selection

This was selected as the next oldest tied recurring invariant audit after the
2026-05-29 `subject-caller-binding` rerun. Its previous dedicated report was
from 2026-05-09.

The run is comparable with the May baseline. The baseline had already fixed
exclusive expiry boundaries for capability replay metadata and existing root
replay records. This run checked whether that convention had remained aligned
across adjacent freshness credentials.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Expiry is enforced centrally for delegated tokens | PASS | `ops/auth/delegated/verify.rs` rejects cert and token windows at `now_secs >= expires_at`. |
| Update delegated tokens are single-use | PASS | `access/auth/token.rs` verifies token material, subject, and scope before `consume_update_token_once(...)`. |
| Query delegated tokens stay stateless | PASS | `query_token_consume_is_stateless` passed. |
| Consumed-token state is keyed and pruned correctly | PASS | Consumed-token markers prune at `now_secs >= expires_at`; replay/capacity tests passed. |
| Root replay per-caller capacity precedes global capacity | PASS | `reserve_root_replay_rejects_caller_capacity_before_global_capacity` passed. |
| Capability replay metadata uses exclusive expiry | PASS | `project_replay_metadata_rejects_expiry_boundary` passed and scan found `now_secs >= expires_at`. |
| Existing root replay records use exclusive expiry | PASS | `evaluate_root_replay_returns_expired_at_expiry_boundary` passed and scan found `now >= existing.expires_at`. |
| Delegated grants use exclusive expiry | FIXED | Audit found `now_secs > grant.expires_at`; changed to `now_secs >= grant.expires_at` and added exact-boundary coverage. |
| Role/internal attestations use exclusive expiry | FIXED | Audit found `now_secs > expires_at`; changed to `now_secs >= expires_at` and added exact-boundary coverage. |

## Scenario Matrix

| Scenario | Expected Behavior | Current Evidence | Result |
| --- | --- | --- | --- |
| Expired capability metadata | Reject before workflow execution | `project_replay_metadata_rejects_expired_metadata` passed | PASS |
| Capability metadata at exact expiry boundary | Reject before workflow execution | `project_replay_metadata_rejects_expiry_boundary` passed | PASS |
| Existing root replay record at exact expiry boundary | Treat as expired | `evaluate_root_replay_returns_expired_at_expiry_boundary` passed | PASS |
| Reused update delegated token | Reject active replay | `update_token_consume_rejects_active_replay` passed | PASS |
| Reused query delegated token | Succeed without durable consumption | `query_token_consume_is_stateless` passed | PASS |
| Consumed-token marker after expiry | Allow nonce after pruning expired marker | `consume_allows_nonce_after_expiry_prune` passed | PASS |
| Consumed-token capacity saturation | Fail closed | `consume_fails_closed_at_capacity` passed | PASS |
| Root replay global saturation by one caller | Caller cap rejects before global cap | `reserve_root_replay_rejects_caller_capacity_before_global_capacity` passed | PASS |
| Delegated grant at exact expiry boundary | Reject | `verify_root_delegated_grant_claims_rejects_expiry_boundary` added and passed | FIXED |
| Internal invocation attestation at exact expiry boundary | Reject | `internal_invocation_claims_reject_expiry_boundary` added and passed | FIXED |
| Role attestation at exact expiry boundary | Reject | `role_attestation_claims_reject_expiry_boundary` added and passed | FIXED |

## Remediation Applied

| Change | Files | Result |
| --- | --- | --- |
| Tightened delegated-grant expiry boundary from `now_secs > expires_at` to `now_secs >= expires_at` | `crates/canic-core/src/api/rpc/capability/grant.rs` | Delegated grants now expire at the same exclusive boundary as replay metadata and delegated tokens. |
| Tightened attestation expiry boundary from `now_secs > expires_at` to `now_secs >= expires_at` | `crates/canic-core/src/ops/auth/verify/attestation.rs` | Role attestations and internal invocation proofs now reject at the exact expiry timestamp. |
| Added exact-boundary regression tests | `crates/canic-core/src/api/rpc/capability/tests.rs`, `crates/canic-core/src/ops/auth/verify/attestation.rs` | Future changes must preserve exclusive expiry semantics for grants and attestations. |

## Comparison To Previous Relevant Run

- Stable: delegated-token cert/token freshness still uses exclusive expiry.
- Stable: root replay metadata and existing replay records kept the exclusive
  boundary fixed in the 2026-05-09 run.
- Stable: query delegated tokens remain stateless.
- Stable: root replay capacity still checks per-caller active entries before
  global capacity.
- Fixed: delegated grants and role/internal attestations now share the same
  exclusive expiry convention.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_delegated_token`, `verify_claims` | canonical cert/token freshness and scope gate | High |
| `crates/canic-core/src/access/auth/token.rs` | `verify_token`, `consume_update_token_once` | update/query delegated-token consumption boundary | High |
| `crates/canic-core/src/storage/stable/auth/token_uses.rs` | `consume_delegated_token_use` | durable consumed-token marker insertion, pruning, and capacity checks | High |
| `crates/canic-core/src/api/rpc/capability/replay.rs` | `project_replay_metadata` | capability metadata freshness and nonce-to-request-id projection | High |
| `crates/canic-core/src/ops/replay/guard.rs` | `evaluate_root_replay`, `resolve_existing` | root replay classification and expiry boundary | High |
| `crates/canic-core/src/api/rpc/capability/grant.rs` | `verify_root_delegated_grant_claims` | delegated-grant freshness and audience/scope checks | High |
| `crates/canic-core/src/ops/auth/verify/attestation.rs` | `verify_attestation_time_window` | role/internal attestation freshness | High |
| `crates/canic-core/src/workflow/rpc/request/handler/replay.rs` | `check_replay`, `commit_replay`, `abort_replay` | workflow integration point for replay decisions | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| delegated-token freshness lane | token/freshness scan found 25 direct files | 5 | 3 | 7 |
| root replay lane | replay scan found 25 direct files | 5 | 3 | 7 |
| workflow request handler | recent edit scan shows handler tests 8, execute 5, mod 4, capability 3 | 4 | 2 | 7 |
| capability replay/grant projection | capability freshness is shared by API, DTO, tests, workflow support | 4 | 2 | 6 |
| attestation freshness lane | auth verifier and API mapping span ops/auth and api/auth | 3 | 2 | 6 |

## Risk Score

Initial Risk Score: **4 / 10**

Post-remediation Risk Score: **3 / 10**

Initial score contributions:

- `+1` delegated grants and attestations used inclusive acceptance at
  `now == expires_at`.
- `+1` delegated-token and root replay freshness remain concentrated in shared
  seams.
- `+1` root replay and `RootRequestMetadata` have broad fan-in.
- `+1` workflow handler edit pressure remains active.

Remediation removed the expiry-boundary contribution by aligning grants and
attestations with the exclusive `now >= expires_at` convention.

Verdict: **Invariant holds after remediation with moderate residual fan-in
pressure.**

## Amplification Drivers

- Freshness helpers are used across endpoint auth, root capability workflow,
  delegated grants, role attestations, and replay caches.
- Boundary-condition drift is easy to reintroduce because some checks live in
  API surfaces while others live in ops/storage layers.
- Replay state and credential DTOs have broad enough fan-in that exact-boundary
  tests are the main practical guard.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| expiry boundary drift | `capability/grant.rs`, `verify/attestation.rs` | found `>` expiry checks while token/replay/session state used `>=` | Fixed |
| replay workflow edit pressure | `workflow/rpc/request/handler/*` | recent scoped history shows repeated changes in handler tests, execute, mod, capability, authorize | Medium |
| delegated-token state fan-in | auth token-use scan | 25 direct files mention token use, claims, nonce, or delegated-token records | Medium |
| root replay DTO spread | `RootRequestMetadata` | referenced in 14 direct files across DTO, ops, API, workflow, and tests | Medium |

### Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `ReplayDecision` | `crates/canic-core/src/ops/replay/guard.rs` | 3 | Low |
| `AuthExpiryError` | `crates/canic-core/src/ops/auth/error.rs` | medium local auth fan-in | Medium |

### Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `RootRequestMetadata` | `crates/canic-core/src/dto/rpc.rs` | api/ops/workflow/tests | Medium |
| `ReplaySlotKey` | `crates/canic-core/src/storage/stable/replay.rs` | storage/ops/workflow/tests | Medium |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | api/access/ops/storage/tests | Medium |

### Growing Hub Modules

| Module | Subsystems Imported | Recent Commits | Risk |
| --- | --- | ---: | --- |
| `crates/canic-core/src/workflow/rpc/request/handler/tests.rs` | workflow replay/capability tests | 8 path hits in scoped history | Medium |
| `crates/canic-core/src/storage/stable/auth/mod.rs` | stable auth/session/token-use storage | 6 path hits in scoped history | Medium |
| `crates/canic-core/src/ops/auth/verify/attestation.rs` | attestation verification and tests | 4 path hits in scoped history | Medium |

### Capability Surface Growth

No new credential/replay capability surface was added by this remediation.

## Dependency Fan-In Pressure

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| delegated token / nonce / token-use group | 25 | `access`, `api`, `ops`, `storage`, `tests`, `testkit` | Architectural gravity well |
| root replay / replay metadata group | 25 | `api`, `dto`, `ops`, `workflow`, `tests`, `testkit` | Architectural gravity well |
| `ReplayDecision` | 3 | `ops`, `workflow` | Normal |
| `ReplaySlotKey` | 7 | `storage`, `ops`, `workflow`, `tests` | Hub forming |
| `RootRequestMetadata` | 14 | `dto`, `api`, `ops`, `workflow`, `tests` | Architectural gravity well |

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo +1.96.0 test -p canic-core --lib update_token_consume_rejects_active_replay --locked -- --nocapture` | PASS | Update delegated-token nonce replay rejects. |
| `cargo +1.96.0 test -p canic-core --lib query_token_consume_is_stateless --locked -- --nocapture` | PASS | Query delegated-token use does not consume durable token state. |
| `cargo +1.96.0 test -p canic-core --lib consume_rejects_active_replay --locked -- --nocapture` | PASS | Consumed-token storage rejects active replay. |
| `cargo +1.96.0 test -p canic-core --lib project_replay_metadata_rejects --locked -- --nocapture` | PASS | Capability replay metadata rejects expired, exact-boundary expired, and future-over-skew inputs. |
| `cargo +1.96.0 test -p canic-core --lib consume_allows_nonce_after_expiry_prune --locked -- --nocapture` | PASS | Consumed-token marker is pruned at expiry boundary before reuse. |
| `cargo +1.96.0 test -p canic-core --lib consume_fails_closed_at_capacity --locked -- --nocapture` | PASS | Consumed-token state fails closed at capacity. |
| `cargo +1.96.0 test -p canic-core --lib reserve_root_replay_rejects_caller_capacity_before_global_capacity --locked -- --nocapture` | PASS | Root replay per-caller cap fires before global capacity. |
| `cargo +1.96.0 test -p canic-core --lib evaluate_root_replay_returns_expired --locked -- --nocapture` | PASS | Existing replay records are expired both after and at the expiry boundary. |
| `cargo +1.96.0 test -p canic-core --lib verify_root_delegated_grant_claims_rejects_expiry_boundary --locked -- --nocapture` | PASS | Delegated grants reject at the exact expiry boundary. |
| `cargo +1.96.0 test -p canic-core --lib internal_invocation_claims_reject_expiry_boundary --locked -- --nocapture` | PASS | Internal invocation attestations reject at the exact expiry boundary. |
| `cargo +1.96.0 test -p canic-core --lib role_attestation_claims_reject_expiry_boundary --locked -- --nocapture` | PASS | Role attestations reject at the exact expiry boundary. |
| `cargo +1.96.0 fmt --all --check` | PASS | Formatting remained clean. |
| `rg -n 'now_secs > expires_at\|now > expires_at\|> grant\.expires_at\|> payload\.expires_at' crates/canic-core/src -g '*.rs'` | PASS | No stale direct `>` expiry checks remained in the scanned core freshness paths. |
| `rg -l 'DelegatedTokenUseRecord\|DelegatedTokenUseConsumeResult\|consume_delegated_token_use\|DelegatedTokenClaims\|nonce' crates/canic-core/src crates/canic-tests/tests -g '*.rs' \| wc -l` | PASS | Delegated-token freshness fan-in scan recorded 25 direct files. |
| `rg -l 'ReplayDecision\|ReplayPending\|ReplaySlotKey\|RootReplayGuardInput\|reserve_root_replay\|project_replay_metadata\|RootRequestMetadata' crates/canic-core/src crates/canic-tests/tests -g '*.rs' \| wc -l` | PASS | Root replay fan-in scan recorded 25 direct files. |
| `rg -l 'ReplayDecision' crates canisters fleets -g '*.rs' \| wc -l` | PASS | `ReplayDecision` direct reference count recorded as 3 files. |
| `rg -l 'ReplaySlotKey' crates canisters fleets -g '*.rs' \| wc -l` | PASS | `ReplaySlotKey` direct reference count recorded as 7 files. |
| `rg -l 'RootRequestMetadata' crates canisters fleets -g '*.rs' \| wc -l` | PASS | `RootRequestMetadata` direct reference count recorded as 14 files. |

## Follow-up Actions

1. Keep delegated-token verification, delegated grants, role attestations,
   replay metadata, sessions, and consumed-token markers on the same exclusive
   `now >= expires_at` boundary.
2. Rerun this audit after changes to delegated-token verification,
   `project_replay_metadata`, `evaluate_root_replay`, delegated grants,
   role-attestation verification, replay capacity limits, or request-id
   derivation.
