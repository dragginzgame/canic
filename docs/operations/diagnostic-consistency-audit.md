# Diagnostic Consistency Audit

This audit is the durable diagnostic inventory for replay-sensitive Canic
release work.

It documents whether existing public errors, internal logs, metrics, tests, and
operator docs distinguish the major failure classes after the 0.61
replay-protection line. It is intentionally not named after a release line;
release numbers belong in changelogs and status docs, not in the operational
audit entry point.

Current release-line context: 0.62 is using this audit for release durability,
operator recovery, and RC validation.

## Scope

This audit covers diagnostics for:

- duplicate or committed replay;
- missing operation IDs;
- invalid operation IDs;
- expired replay metadata, receipts, or delegated authorization;
- wrong caller or actor mismatch;
- wrong shard or delegated-auth shard mismatch;
- delegation-proof replay;
- delegated-token mint or issue replay;
- pending operation already exists;
- operation completed and receipt replayed;
- recovery-required operation state;
- value-transfer cost refusal;
- upgrade or permit-boundary refusal;
- durable-publication conflict or ambiguity.

This audit does not change runtime behavior, Candid, CLI output, JSON/output
formats, package manifests, dependencies, lockfiles, fixtures, snapshots, or
generated artifacts.

## Public-Output Boundary

Diagnostic surfaces have different compatibility weight:

| Surface | Compatibility note |
| --- | --- |
| Internal runtime log | May be improved when needed, but should avoid sensitive payloads and must state whether log text is the only changed output. |
| Developer-only tool output | Requires changelog/status coverage when changed. |
| CLI text output | Stable user-visible output; requires explicit approval, changelog/status coverage, and focused tests. |
| JSON output | Stable automation surface; requires explicit approval, schema impact notes, changelog/status coverage, and focused tests. |
| Candid/public API | Stable canister contract; no change expected in 0.62 without explicit approval. |
| Metric label | Treat as stable operational output; changing or adding labels requires focused approval and tests. |
| No public-output impact | Docs/test/CI-only clarification or existing-surface inventory. |

Public output changes are not included in this audit. If a future slice changes
CLI text, JSON fields, Candid signatures, public error codes, or metric labels,
that slice must include an explicit public-output impact statement and tests for
the changed surface.

## Outcome Labels

| Outcome | Meaning |
| --- | --- |
| Existing diagnostic sufficient | Current public errors, logs, metrics, tests, or docs are enough for RC operation. |
| Clarified by this audit | This docs/CI slice explains how to interpret existing diagnostics without changing them. |
| Needs RC accounting | The gap should be tracked during RC validation, but is not a reason to continue implementation slicing by itself. |
| Public output change proposed separately | A stable/user-visible change would need separate approval and test coverage. |
| Release blocker | The area must be fixed or proven before RC promotion. |

## Diagnostic Matrix

| Failure class | Existing diagnostic evidence | Public-output impact | Outcome | Notes |
| --- | --- | --- | --- | --- |
| Duplicate or committed replay | Root replay records duplicate metrics with `duplicate`; auth token replay logs `committed replay returned`; ICP refill logs `icp refill committed replay returned`; stable replay tests cover committed receipt decoding. | None in this audit | Existing diagnostic sufficient | Same-input committed replay is distinguishable from conflict and in-progress decisions. |
| Missing operation ID | Public `ErrorCode::OperationIdRequired` returns message `operation_id is required for this command`; root RPC, pool, and auth tests pin the code/message. | None in this audit | Existing diagnostic sufficient | Missing replay metadata should not leak internal `MissingReplayMetadata` wording. |
| Invalid operation ID | `OperationId` parsing distinguishes invalid byte length, invalid hex length, and invalid hex characters; invalid request metadata maps through invalid-input paths where used. | None in this audit | Clarified by this audit | RC validation should check any CLI/API path that accepts textual operation IDs before release, but no current release blocker is identified here. |
| Expired replay metadata or receipt | Root replay records `expired` metrics; root RPC errors include replay expiration; auth/token and ICP refill replay decisions return expired conflict wording. | None in this audit | Existing diagnostic sufficient | Expiration is distinguishable from actor mismatch, payload mismatch, pending, and recovery-required states. |
| Expired delegated authorization | Delegated-auth verifier metrics distinguish `cert_expired` and `token_expired`; auth errors include expired cert/token wording. | None in this audit | Existing diagnostic sufficient | This is auth validity, not proof that an external effect did not happen. |
| Wrong caller or actor mismatch | Shared receipt decisions expose `ActorMismatch`; auth/token, pool, and ICP refill public conflict messages say the request ID or operation ID was reused by a different caller; auth logs record `actor_mismatch`. | None in this audit | Existing diagnostic sufficient | The recovery runbooks require operators to stop and compare actor identity before retrying. |
| Wrong shard or delegated-auth shard mismatch | Root RPC errors include `delegation request caller ... must match shard_pid ...`; delegated-auth metrics include issuer shard and shard-key mismatch labels. | None in this audit | Existing diagnostic sufficient | Wrong-shard diagnostics are distinguishable from generic replay conflict by auth/shard wording and metrics labels. |
| Delegation-proof replay | Delegation proof replay decisions distinguish committed replay, operation in progress, actor mismatch, payload mismatch, expired receipt, recovery-required state, terminal failure, and pending quotas. | None in this audit | Existing diagnostic sufficient | Public conflict text remains stable; no new Candid or output shape is introduced. |
| Delegated-token mint or issue replay | Token replay logs record reserved, blocked decision, effect marked, recovery required, response commit failed, and response committed; public messages distinguish in-progress, caller mismatch, payload mismatch, expired, recovery required, terminal failed, and quota exhaustion. | None in this audit | Existing diagnostic sufficient | Existing logs include command kind, operation ID, and caller, but avoid token payload bytes. |
| Pending operation already exists | Shared replay decisions use `OperationInProgress`; public messages say already in progress and retry later with the same operation/request ID; CLI refill pending log records `pending_send` locally. | None in this audit | Existing diagnostic sufficient | Project-local pending logs are host-side retry aids, not server authority. |
| Operation completed and receipt replayed | Committed replay returns cached response and logs committed replay where implemented; stable replay tests cover committed receipt round-trip. | None in this audit | Existing diagnostic sufficient | This is distinct from `TerminalFailed`, `OperationInProgress`, and `RecoveryRequired`. |
| Recovery-required operation state | Shared receipt status records `RecoveryRequired` with reason; auth and ICP refill public conflict wording says recovery is required before replay; recovery logs include effect, command kind, operation ID, and error class/origin. | None in this audit | Existing diagnostic sufficient | Operators must treat this as uncertainty, not as proof the external effect failed. |
| Value-transfer cost refusal | `CostGuardOps` returns resource/capacity failures before effect boundaries; public exhausted errors use `ErrorCode::ResourceExhausted`; cost-guard boundary tests pin permit-required value-transfer adapters. | None in this audit | Existing diagnostic sufficient | Cost refusal should be interpreted as pre-effect unless logs show an external-effect boundary was marked. |
| Upgrade or permit-boundary refusal | Management deployment, signing, and value-transfer adapters require `CostGuardPermit`; source guards pin private permit construction and permitted adapter call sites. | None in this audit | Existing diagnostic sufficient | This audit does not add logs; it records current executable boundary evidence. |
| Durable-publication conflict or ambiguity | Replay policy tests pin durable-publication endpoints and root wasm-store reconcile tests cover current release binding after upgrade. | None in this audit | Needs RC accounting | Existing tests cover policy/state evidence. Operator ambiguity still belongs in RC validation and runbook use if a real incident occurs. |

## Required RC Gates

Use these gates when validating diagnostic consistency before RC promotion:

```text
bash scripts/ci/check-diagnostic-consistency-audit.sh
cargo test --locked -p canic-core replay_policy --lib -- --nocapture
cargo test --locked -p canic-core --test cost_guard_boundary_guard -- --nocapture
cargo test --locked -p canic-core --test delegated_auth_hard_cut_guard -- --nocapture
cargo test --locked -p canic-core storage::stable::replay --lib -- --nocapture
cargo test --locked -p canic-core workflow::ic::icp_refill --lib -- --nocapture
cargo test --locked -p canic-core workflow::rpc::request::handler --lib -- --nocapture
cargo test --locked -p canic-core workflow::pool --lib -- --nocapture
cargo test --locked -p canic-tests --test root_wasm_store_reconcile root_post_upgrade_preserves_multi_store_current_release_binding -- --test-threads=1 --nocapture
```

The `canic-tests` gate is PocketIC-backed and may be assigned to CI or an RC
validation environment when too expensive for an ordinary docs slice.

## Diagnostic Change Rules

Every future diagnostic change must declare its output class:

- internal runtime log;
- developer-only tool output;
- CLI text;
- JSON schema;
- Candid/public API;
- metric label;
- no public-output impact.

Stable or user-visible diagnostic changes require:

- explicit approval;
- changelog and status coverage;
- focused tests for the changed surface;
- confirmation that no sensitive payload, token, module bytes, or private key
  material is logged or printed;
- confirmation that JSON/output shape changes are intentional, not incidental.

## Non-Goals

- No runtime behavior change.
- No Candid change.
- No CLI output change.
- No JSON/output format change.
- No dependency or lockfile change.
- No generated artifact change.
- No metric label change.
- No public error-code change.
- No new diagnostic command.
- No automatic recovery promise.

## Outcome Summary

Release blockers: none found in this audit.

The current diagnostic evidence is sufficient to continue 0.62 without opening
another runtime implementation slice. Remaining work belongs to package/install
validation, RC accounting, or focused defect handling if a concrete
release-blocking diagnostic gap is found.
