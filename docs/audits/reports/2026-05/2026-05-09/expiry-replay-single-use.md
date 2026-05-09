# Expiry Replay Single-Use Invariant Audit - 2026-05-09

## Report Preamble

- Definition path: `docs/audits/recurring/invariants/expiry-replay-single-use.md`
- Scope: delegated-token freshness, update-token single-use consumption,
  query-token statelessness, consumed-token pruning, root capability replay
  metadata, root replay cache expiry, and replay-store capacity ordering
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-05/expiry-replay-single-use.md`
- Code snapshot identifier: `518f57dd`
- Method tag/version: `Method V4.2`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-09T13:34:02Z`
- Branch: `main`
- Worktree: `dirty`

## Audit Selection

This was selected as the next oldest recurring invariant audit after
`capability-scope-enforcement`. Its latest previous report was
`docs/audits/reports/2026-04/2026-04-05/expiry-replay-single-use.md`.

The run is partially comparable with the April baseline because delegated-token
update replay state, root replay caller attribution, and session-bootstrap
capacity handling have changed during the current 0.33.1 work. The freshness
invariant itself remains directly comparable.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Expiry is enforced centrally for delegated tokens | PASS | `ops/auth/delegated/verify.rs` verifies cert time, token window, token TTL, cert bounds, and `now_secs >= claims.expires_at` before returning verified claims. |
| Update delegated tokens are single-use | PASS | `access/auth/token.rs:52-63` verifies token material, subject, and scope before calling `consume_update_token_once(...)`; update calls consume through `AuthOps::consume_delegated_token_use(...)`. |
| Query delegated tokens stay stateless | PASS | `access/auth/token.rs:70-80` only consumes token state for `EndpointCallKind::Update`; `query_token_consume_is_stateless` passed. |
| Consumed-token state is keyed and pruned correctly | PASS | `storage/stable/auth/token_uses.rs:21-35` prunes expired markers before replay/capacity checks; `same_token_use_key(...)` binds issuer shard, subject, cert hash, and nonce at `53-57`. |
| Root replay per-caller capacity precedes global capacity | PASS | `ops/replay/mod.rs:52-71` checks `active_root_slot_len_for_caller(...)` before `root_slot_len()`; targeted caller-capacity test passed. |
| Capability replay metadata uses bounded skew | PASS | `api/rpc/capability/replay.rs:19-37` rejects zero TTL, future issued-at beyond `MAX_CAPABILITY_CLOCK_SKEW_SECONDS`, overflow, and expired metadata. |
| Expiry boundary is exclusive | FIXED | This run found `now == expires_at` was accepted for capability replay metadata and existing root replay records. Remediation changed both to `>=` and added exact-boundary regression tests. |

## Scenario Matrix

| Scenario | Expected Behavior | Current Evidence | Result |
| --- | --- | --- | --- |
| Expired capability metadata | Reject before workflow execution | `project_replay_metadata_rejects_expired_metadata` passed | PASS |
| Capability metadata at exact expiry boundary | Reject before workflow execution | `project_replay_metadata_rejects_expiry_boundary` added and passed | FIXED |
| Existing root replay record at exact expiry boundary | Treat as expired, not duplicate/cached | `evaluate_root_replay_returns_expired_at_expiry_boundary` added and passed | FIXED |
| Reused update delegated token | Reject active replay | `update_token_consume_rejects_active_replay` and `consume_rejects_active_replay` passed | PASS |
| Reused query delegated token | Succeed without durable consumption | `query_token_consume_is_stateless` passed | PASS |
| Consumed-token marker after expiry | Allow nonce after pruning expired marker | `consume_allows_nonce_after_expiry_prune` passed | PASS |
| Consumed-token capacity saturation | Fail closed | `consume_fails_closed_at_capacity` passed | PASS |
| Root replay global saturation by one caller | Caller cap rejects before global cap | `reserve_root_replay_rejects_caller_capacity_before_global_capacity` passed | PASS |
| Runtime root replay expired request | Reject through PocketIC workflow path | `root_suite replay_rejects_expired_request` passed | PASS |
| Runtime root replay TTL above maximum | Reject through PocketIC workflow path | `root_suite replay_rejects_ttl_above_max` passed | PASS |

## Remediation Applied

| Change | Files | Result |
| --- | --- | --- |
| Tightened capability replay metadata expiry boundary from `now > expires_at` to `now >= expires_at` | `crates/canic-core/src/api/rpc/capability/replay.rs` | Capability replay metadata now expires at the same exclusive boundary as delegated tokens. |
| Tightened existing root replay record expiry boundary from `now > expires_at` to `now >= expires_at` | `crates/canic-core/src/ops/replay/guard.rs` | Cached or in-flight root replay records cannot be reused at their exact expiry timestamp. |
| Added exact-boundary regression tests | `crates/canic-core/src/api/rpc/capability/tests.rs`, `crates/canic-core/src/ops/replay/guard.rs` | Future changes must preserve exclusive expiry semantics for both projection and cached replay classification. |

## Comparison to Previous Relevant Run

- Stable: delegated-token cert/token freshness still runs through the canonical
  verifier before access-layer subject/scope checks.
- Improved: update-call delegated tokens now have durable single-use markers
  keyed by issuer shard, subject, cert hash, and nonce.
- Stable: query-call delegated tokens remain stateless.
- Improved: root replay capacity now checks per-caller active entries before
  global capacity.
- Fixed: root capability replay metadata and existing root replay records now
  share the exclusive expiry boundary used by token and consumed-token state.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_delegated_token`, `verify_claims` | canonical cert/token freshness and scope gate | High |
| `crates/canic-core/src/access/auth/token.rs` | `verify_token`, `consume_update_token_once` | update/query delegated-token consumption boundary | High |
| `crates/canic-core/src/storage/stable/auth/token_uses.rs` | `consume_delegated_token_use` | durable consumed-token marker insertion, pruning, and capacity checks | High |
| `crates/canic-core/src/api/rpc/capability/replay.rs` | `project_replay_metadata` | capability metadata freshness and nonce-to-request-id projection | High |
| `crates/canic-core/src/ops/replay/guard.rs` | `evaluate_root_replay`, `resolve_existing` | root replay classification and expiry boundary | High |
| `crates/canic-core/src/ops/replay/mod.rs` | `reserve_root_replay`, `commit_root_replay`, `abort_root_replay` | root replay reservation/commit/abort transitions | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/replay.rs` | `check_replay`, `commit_replay`, `abort_replay` | workflow integration point for replay decisions | Medium |

## Hub Module Pressure

| Module | Fan-In Evidence | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| delegated-token freshness lane | token/freshness scan found 24 direct files | 5 | 3 | 7 |
| root replay lane | replay scan found 25 direct files | 5 | 3 | 7 |
| workflow request handler | recent edit scan shows handler tests 8, execute 6, replay 5 | 4 | 2 | 7 |
| capability replay projection | capability replay metadata is shared by API, DTO, tests, and integration support | 4 | 2 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| expiry boundary drift | `api/rpc/capability/replay.rs`, `ops/replay/guard.rs` | found `>` expiry checks while delegated-token and consumed-token state used `>=` | Fixed |
| replay workflow edit pressure | `workflow/rpc/request/handler/*` | recent scan shows repeated changes in handler tests, execute, replay, and capability modules | Medium |
| delegated-token state fan-in | auth token-use scan | 24 direct files mention token use, claims, nonce, or delegated-token records | Medium |
| root replay DTO spread | `RootRequestMetadata` | referenced in 14 direct files across DTO, ops, API, workflow, and tests | Medium |

## Dependency Fan-In Pressure

### Module / Symbol Fan-In

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| delegated token / nonce / token-use group | 24 | `access`, `api`, `ops`, `storage`, `tests`, `testkit` | Architectural gravity well |
| root replay / replay metadata group | 25 | `api`, `dto`, `ops`, `workflow`, `tests`, `testkit` | Architectural gravity well |
| `ReplayDecision` | 3 | `ops`, `workflow` | Normal |
| `ReplaySlotKey` | 7 | `storage`, `ops`, `workflow`, `tests` | Hub forming |
| `RootRequestMetadata` | 14 | `dto`, `api`, `ops`, `workflow`, `tests` | Architectural gravity well |

## Risk Score

Initial Risk Score: **5 / 10**

Post-remediation Risk Score: **3 / 10**

Initial score contributions:

- `+2` exact-expiry replay boundary drift allowed replay metadata and cached
  root replay records at `now == expires_at`.
- `+1` delegated-token and root replay freshness remain concentrated in a few
  shared seams.
- `+1` root replay and `RootRequestMetadata` have broad fan-in.
- `+1` workflow handler edit pressure remains active.

Remediation removed the expiry-boundary contribution by aligning capability
metadata projection and root replay cache classification with the exclusive
boundary already used by delegated tokens and consumed-token state.

Verdict: **Invariant holds after remediation with moderate residual fan-in
pressure.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib update_token_consume_rejects_active_replay -- --nocapture` | PASS | Update delegated-token nonce replay rejects. |
| `cargo test -p canic-core --lib query_token_consume_is_stateless -- --nocapture` | PASS | Query delegated-token use does not consume durable token state. |
| `cargo test -p canic-core --lib consume_rejects_active_replay -- --nocapture` | PASS | Consumed-token storage rejects active replay. |
| `cargo test -p canic-core --lib project_replay_metadata_rejects -- --nocapture` | PASS | Capability replay metadata rejects expired, exact-boundary expired, and future-over-skew inputs. |
| `cargo test -p canic-core --lib consume_allows_nonce_after_expiry_prune -- --nocapture` | PASS | Consumed-token marker is pruned at expiry boundary before reuse. |
| `cargo test -p canic-core --lib consume_fails_closed_at_capacity -- --nocapture` | PASS | Consumed-token state fails closed at capacity. |
| `cargo test -p canic-core --lib reserve_root_replay_rejects_caller_capacity_before_global_capacity -- --nocapture` | PASS | Root replay per-caller cap fires before global capacity. |
| `cargo test -p canic-core --lib evaluate_root_replay_returns_expired -- --nocapture` | PASS | Existing replay records are expired both after and at the expiry boundary. |
| `cargo test -p canic-tests --test root_suite replay_rejects_expired_request -- --test-threads=1 --nocapture` | PASS | PocketIC root workflow rejects expired replay request. |
| `cargo test -p canic-tests --test root_suite replay_rejects_ttl_above_max -- --test-threads=1 --nocapture` | PASS | PocketIC root workflow rejects replay TTL above max. |
| `cargo clippy -p canic-core --all-targets -- -D warnings` | PASS | Touched replay and capability tests pass clippy. |
| `rg -l 'DelegatedTokenUseRecord\|DelegatedTokenUseConsumeResult\|consume_delegated_token_use\|DelegatedTokenClaims\|nonce' crates/canic-core/src crates/canic-tests/tests -g '*.rs'` | PASS | Delegated-token freshness fan-in scan recorded 24 direct files. |
| `rg -l 'ReplayDecision\|ReplayPending\|ReplaySlotKey\|RootReplayGuardInput\|reserve_root_replay\|project_replay_metadata\|RootRequestMetadata' crates/canic-core/src crates/canic-tests/tests -g '*.rs'` | PASS | Root replay fan-in scan recorded 25 direct files. |
| `rg -l 'ReplayDecision' crates canisters fleets -g '*.rs'` | PASS | `ReplayDecision` direct reference count recorded as 3 files. |
| `rg -l 'ReplaySlotKey' crates canisters fleets -g '*.rs'` | PASS | `ReplaySlotKey` direct reference count recorded as 7 files. |
| `rg -l 'RootRequestMetadata' crates canisters fleets -g '*.rs'` | PASS | `RootRequestMetadata` direct reference count recorded as 14 files. |
| `git log --name-only -n 20 -- crates/canic-core/src/access/auth crates/canic-core/src/ops/auth crates/canic-core/src/storage/stable/auth crates/canic-core/src/api/rpc/capability crates/canic-core/src/ops/replay crates/canic-core/src/workflow/rpc/request/handler crates/canic-core/src/storage/stable/replay` | PASS | Recent edit-pressure scan recorded handler/auth/replay hotspots. |

## Follow-up Actions

1. Completed: align capability metadata and root replay cache expiry with the
   exclusive `now >= expires_at` boundary.
2. Keep root replay metadata, delegated-token use markers, and session-bootstrap
   replay policy on the same exclusive expiry convention.
3. Re-run this audit after changes to delegated-token verification,
   `project_replay_metadata`, `evaluate_root_replay`, replay capacity limits, or
   request-id derivation.
