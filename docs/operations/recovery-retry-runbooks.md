# Recovery and Retry Runbooks

These runbooks are the durable operator recovery reference for replay-sensitive
Canic release work.

They document existing behavior after the 0.61 replay-protection line. They
are intentionally not named after a release line; release numbers belong in
changelogs and status docs, not in the operational runbook entry point.

Current release-line context comes from `docs/status/current.md`. This file is
the canonical operator recovery reference for current release work.

## Scope

These runbooks cover manual retry and recovery decisions for:

- replay receipt conflicts;
- replay receipt pending and recovery-required states;
- operation-ID reuse;
- pending ICP refill operations under the selected ICP project root;
- delegated-auth caller and issuer binding failures;
- delegated-token prepare and issue replay;
- ICP refill and value-transfer replay;
- cost-boundary refusals;
- durable-publication ambiguity;
- recovery-watchdog takeover of an orphaned asynchronous timer attempt;
- same-release canister restart/upgrade or publication failures near
  replay-sensitive boundaries.

These runbooks do not change runtime behavior, Candid, CLI output, JSON/output
formats, package manifests, dependencies, lockfiles, fixtures, snapshots, or
generated artifacts.

## Related Evidence

- [0.61 replay-protection design](../design/archive/0.61-replay-protection/0.61-design.md)
  records the replay, receipt, operation-ID, and cost-guard model.
- [Release validation matrix](release-validation-matrix.md) records the
  validation gates used for slice close-out, RC promotion, and final release.
- [Expiry/replay invariant](../audits/recurring/invariants/expiry-replay-single-use.md)
  owns the current replay and freshness audit method.
- [Security boundary ordering](../audits/recurring/system/security-boundary-ordering.md)
  owns current cross-stage replay/auth ordering evidence.

## Operator Safety Rules

The primary retry invariant is:

```text
same operation ID, same actor, same payload
```

Safe retry means the operator repeats the same logical operation with the same
operation ID, same authenticated actor, same target, and same payload.

Do not change payload, caller, issuer, or target while reusing an operation ID.

Do not manually edit canister stable state, replay receipts, cost-guard intent
state, publication state, or pending operation stores as a recovery shortcut.

Do not treat a `RecoveryRequired` state as proof that the external effect did
not happen. It means Canic preserved uncertainty so the operation cannot be
silently repeated.

Do not retry a high-value operation with a new operation ID merely to bypass an
`OperationInProgress`, replay conflict, cost-boundary refusal, or
recovery-required state. Use a new operation ID only when the original operation
is known not to have crossed an external-effect boundary, or after a maintainer
has decided that a new logical operation is safe.

`pending_send` records under the selected ICP project root are host-side retry
aids. They are not canister stable state, and server correctness must not
depend on them.

These runbooks never authorize a cross-release upgrade or state transition.
Every pre-1.0 release transition starts from empty Fleet state; the restart and
upgrade guidance below applies only within one release.

## Runbook Template

Each runbook uses the same fields:

| Field | Meaning |
| --- | --- |
| Symptom | What the maintainer/operator sees. |
| Likely cause | The most likely class of failure. |
| Safety invariant | The state property that must be preserved. |
| Safe operator action | The action that preserves replay and cost-boundary safety. |
| Unsafe operator action | The action that risks duplicate effects or hidden drift. |
| Diagnostic, log, or public error to check | Existing evidence to inspect before acting. |
| Retry/idempotency rule | Whether retrying is safe, blocked, or requires escalation. |
| Relevant validation command | Existing gate that protects the behavior. |
| Escalation criteria | When to stop and ask for a focused defect fix or RC decision. |

## Runbooks

### Safe Retry After Network Or Client Failure

| Field | Guidance |
| --- | --- |
| Symptom | The client process exits, the network drops, or the response is lost before the operator knows whether the update completed. |
| Likely cause | Transport uncertainty after the caller submitted a replay-sensitive operation. |
| Safety invariant | Retry only the same operation ID, same actor, same payload, and same target. |
| Safe operator action | Re-run the exact same command or request with the same operation ID. For CLI-generated ICP refill operations, reuse the matching `pending_send` entry under the selected ICP project root when present. |
| Unsafe operator action | Generating a new operation ID for the same uncertain operation, changing arguments, changing caller identity, or changing target canister. |
| Diagnostic, log, or public error to check | Operation ID printed by the CLI, `.canic/operations/pending.json` for live CLI refill, public conflict messages such as "already in progress" or "previously failed", and replay logs for `operation_in_progress` or `recovery_required`. |
| Retry/idempotency rule | A same-input retry is safe. A changed-input retry is a new operation and must not reuse the old operation ID. |
| Relevant validation command | `cargo test --locked -p canic-core replay_policy --lib -- --nocapture` |
| Escalation criteria | Escalate if the retry reports `RecoveryRequired`, payload mismatch, actor mismatch, or an unexplained terminal failure. |

### Duplicate Operation Or Committed Replay

| Field | Guidance |
| --- | --- |
| Symptom | A repeated request returns the already committed result or reports that the operation was already handled. |
| Likely cause | The replay receipt already committed a response for the same operation. |
| Safety invariant | Committed replay must return the cached response without re-running the external effect. |
| Safe operator action | Treat the committed response as the result of the original operation. Record the operation ID in the release or incident notes if the operation was high value. |
| Unsafe operator action | Retrying with a new operation ID to force the effect to run again without proving a new logical operation is intended. |
| Diagnostic, log, or public error to check | Replay receipt status `Committed`, public replay response, or tests that decode committed replay receipt bytes. |
| Retry/idempotency rule | Same-input replay is idempotent after commit. New operation IDs are separate operations. |
| Relevant validation command | `cargo test --locked -p canic-core storage::stable::replay --lib -- --nocapture` |
| Escalation criteria | Escalate if the committed response cannot be decoded or differs from the known original result. |

### Operation Already In Progress

| Field | Guidance |
| --- | --- |
| Symptom | The request fails with `OperationInProgress`, "already in progress", or equivalent conflict wording. |
| Likely cause | A receipt is still `Reserved` or `ExternalEffectInFlight`, or an equivalent workflow-specific pending state exists. |
| Safety invariant | Canic must not execute a second external effect while the first one is still uncertain. |
| Safe operator action | Wait, then retry the same operation ID with the same actor and payload. Check whether a local pending log or canister-side receipt later moves to committed, terminal failed, or recovery required. |
| Unsafe operator action | Reusing the operation ID with modified input, or creating a fresh operation ID to bypass the in-progress receipt. |
| Diagnostic, log, or public error to check | Public "already in progress" conflict, replay logs with `operation_in_progress`, and `.canic/operations/pending.json` for CLI refill. |
| Retry/idempotency rule | Retry later with the same operation ID. Do not change the request. |
| Relevant validation command | `cargo test --locked -p canic-core replay_policy --lib -- --nocapture` |
| Escalation criteria | Escalate if the state never progresses and no receipt, pending log, or workflow record can explain the operation. |

### Payload Or Caller Mismatch

| Field | Guidance |
| --- | --- |
| Symptom | The request fails with payload mismatch, actor mismatch, reused by a different caller, or reused with a different payload. |
| Likely cause | The operation ID was reused for a different logical operation, a different authenticated caller, or a changed request body. |
| Safety invariant | One operation ID identifies one actor-bound payload. |
| Safe operator action | Stop the retry. Compare the operation ID, caller identity, target, and payload against the original request. Use a new operation ID only for a confirmed new logical operation. |
| Unsafe operator action | Editing the request until the conflict disappears, or treating mismatch as a transient network failure. |
| Diagnostic, log, or public error to check | Public conflict text, replay logs with `payload_mismatch` or `actor_mismatch`, and the original command/request record. |
| Retry/idempotency rule | Same operation ID cannot be used after payload or actor drift. |
| Relevant validation command | `cargo test --locked -p canic-core replay_policy --lib -- --nocapture` |
| Escalation criteria | Escalate if the operator believes the payload and actor are identical but the system reports mismatch. |

### Expired Authorization Or Replay Metadata

| Field | Guidance |
| --- | --- |
| Symptom | The request fails because replay metadata, delegated authorization, or a receipt has expired. |
| Likely cause | The retry was attempted outside the validity window for the token, proof, permit, or replay receipt. |
| Safety invariant | Expired authorization must not be treated as a proof that an old effect is safe to replay. |
| Safe operator action | Confirm whether the original operation committed. If no external-effect boundary was crossed, submit a new logical operation with fresh authorization and a new operation ID. |
| Unsafe operator action | Reusing expired metadata, changing the request under the old operation ID, or bypassing authorization checks. |
| Diagnostic, log, or public error to check | Public expired receipt or authorization text, delegated-auth validation errors, and replay receipt timestamps where available. |
| Retry/idempotency rule | Expired metadata generally requires a new logical operation after the original outcome is understood. |
| Relevant validation command | `cargo test --locked -p canic-core ops::auth::delegated --lib -- --nocapture` |
| Escalation criteria | Escalate if an expired replay-sensitive operation may already have crossed an external-effect boundary. |

### Delegation Caller Or Issuer Mismatch

| Field | Guidance |
| --- | --- |
| Symptom | Delegation proof or token issue fails with caller mismatch, issuer mismatch, subject/caller mismatch, or proof replay conflict. |
| Likely cause | The requested issuer, authenticated subject, or transport caller does not match the replay-bound actor. |
| Safety invariant | Delegated-auth replay identity is bound to the verified caller/subject and issuer, not caller-provided fields alone. |
| Safe operator action | Verify the caller identity, issuer principal, subject, and operation ID. Retry only from the same authenticated context with the same payload. |
| Unsafe operator action | Reusing a proof or token request ID from a different caller, issuer, or subject. |
| Diagnostic, log, or public error to check | Public caller/issuer mismatch text, auth replay logs, and delegated-auth hard-cut guard coverage. |
| Retry/idempotency rule | Same caller/issuer/payload may retry. Different caller or issuer requires a new, authorized logical operation. |
| Relevant validation command | `cargo test --locked -p canic-core ops::auth::delegated --lib -- --nocapture` |
| Escalation criteria | Escalate if the verified caller/issuer binding cannot be reconstructed from available logs or request records. |

### ICP Project Root Pending ICP Refill

| Field | Guidance |
| --- | --- |
| Symptom | `canic cycles convert` reports that it reused a pending operation identity, or `.canic/operations/pending.json` under the selected ICP project root retains a `pending_send` ICP-refill record. |
| Likely cause | The CLI durably reserved the exact refill operation before sending it, but the process or transport ended before the terminal response was recorded locally. |
| Safety invariant | The same ICP project root, environment, Fleet, Root, source subaccount and amount must reuse the exact retained operation identity. |
| Safe operator action | Preserve the pending file and repeat the exact conversion command from the same ICP project root and authenticated identity. Confirm protected Root funding status before starting any competing refill. |
| Unsafe operator action | Deleting or editing the pending file, changing its operation identity, switching the Root/subaccount/amount, or sending a second refill while the retained outcome is uncertain. |
| Diagnostic, log, or public error to check | `.canic/operations/pending.json`, the printed operation-ID source, `canic cycles funding <fleet> --json`, and the Root's retained ICP-refill result. |
| Retry/idempotency rule | An exact retry reuses the pending identity. Changed input is a different logical operation and must not bypass unresolved retained work. |
| Relevant validation command | `cargo test --locked -p canic-cli cycles::convert --lib -- --nocapture` |
| Escalation criteria | Escalate before another value transfer when the pending log and protected Root status cannot prove whether Ledger transfer or CMC notification completed. |

### ICP Refill Recovery-Required State

| Field | Guidance |
| --- | --- |
| Symptom | ICP refill reports `RecoveryRequired`, "requires recovery before replay", or an external-effect status is unknown. |
| Likely cause | The workflow crossed or approached ledger transfer, CMC notify, or response commit boundaries and could not prove a terminal outcome. |
| Safety invariant | Do not repeat ledger transfer or notify while the original external effect may already have happened. |
| Safe operator action | Stop automatic retries. Inspect ledger/CMC evidence, the refill workflow record, and replay receipt status before deciding whether manual reconciliation is needed. |
| Unsafe operator action | Retrying with a fresh operation ID to force another refill, or assuming the external effect failed because the response was not committed. |
| Diagnostic, log, or public error to check | ICP refill replay conflict logs, refill records, replay receipt `RecoveryRequired(ExternalEffectStatusUnknown)` or `RecoveryRequired(ResponseCommitFailed)`, ledger transfer evidence, and CMC notify evidence. |
| Retry/idempotency rule | Recovery-required refill is not a blind retry case. It requires reconciliation before any new logical refill. |
| Relevant validation command | `cargo test --locked -p canic-core workflow::ic::icp_refill --lib -- --nocapture` |
| Escalation criteria | Escalate before any second value transfer if ledger/CMC evidence is incomplete or contradictory. |

### Cost-Boundary Refusal

| Field | Guidance |
| --- | --- |
| Symptom | Signing, deployment, value-transfer, or durable-publication work is refused with quota, cycle reserve, resource exhaustion, or cost-boundary wording. |
| Likely cause | The cost guard refused to reserve quota or cycles before an expensive external effect. |
| Safety invariant | Expensive effects must not cross signing, deployment, value-transfer, or durable-publication boundaries without an explicit cost permit. |
| Safe operator action | Treat the refusal as a pre-effect denial unless logs show an external-effect boundary was marked. Wait for quota windows, fund the payer, or use approved release capacity planning. |
| Unsafe operator action | Bypassing the cost guard, editing intent state, or retrying many fresh operation IDs to evade quotas. |
| Diagnostic, log, or public error to check | `ResourceExhausted`, cost-guard logs, replay effect markers, and the relevant command kind/quota subject. |
| Retry/idempotency rule | Retry only after the cost condition changes, and preserve operation-ID rules if the original request reserved a replay receipt. |
| Relevant validation command | `cargo test --locked -p canic-core --test cost_guard_boundary_guard -- --nocapture` |
| Escalation criteria | Escalate if an expensive adapter can be reached without a `CostGuardPermit`, or if refusal happens after an unmarked external effect. |

### Durable-Publication Ambiguity

| Field | Guidance |
| --- | --- |
| Symptom | Exact root-local Store bootstrap or template publication appears partially applied or in conflict with a replay receipt during one installation. |
| Likely cause | Store attestation, durable-publication state, or replay receipt state must be checked before deciding whether the same-release operation completed. |
| Safety invariant | Publication should be reconciled from durable state, not repeated blindly. |
| Safe operator action | Inspect the exact root-local Store attestation, current template publication state, and replay receipt status. Retry only the same operation when the receipt allows it. |
| Unsafe operator action | Re-publishing with a fresh operation ID to force the desired state without checking current durable binding. |
| Diagnostic, log, or public error to check | Durable-publish replay policy entries, exact Store attestation state, and prepared-root Store verification tests. |
| Retry/idempotency rule | If durable state already reflects the intended publication, treat it as converged. If receipt state is recovery-required, reconcile before retry. |
| Relevant validation command | `cargo test --locked -p canic-testing-internal prepared_root_bootstraps_and_reverifies_its_exact_local_store --lib -- --test-threads=1 --nocapture` |
| Escalation criteria | Escalate if durable state and replay receipt state disagree and no existing test explains the case. |

### Same-Release Restart Or Upgrade Near Replay-Sensitive Operation

| Field | Guidance |
| --- | --- |
| Symptom | A same-release upgrade or restart happens while replay-sensitive work is pending, in flight, or near lifecycle restoration. |
| Likely cause | The canister restarted within one release around replay receipt, cost-guard, lifecycle, or durable-publication state. |
| Safety invariant | Post-upgrade initialization must restore Canic invariants before user hooks or async recovery work act on replay-sensitive state. |
| Safe operator action | Check lifecycle readiness, replay receipt state, cost-guard intent state, and the relevant durable publication or workflow record before retrying. |
| Unsafe operator action | Assuming restart cleared pending state, manually deleting state, or running user recovery before Canic invariants are restored. |
| Diagnostic, log, or public error to check | Lifecycle boundary tests, stable replay receipt tests, stable-memory ABI guard, and upgrade/state compatibility audit. |
| Retry/idempotency rule | Retry only after the same-release binary reports readiness and the original replay state is understood. Cross-release transition is reinstall-only. |
| Relevant validation command | `cargo test --locked -p canic-tests --test lifecycle_boundary -- --test-threads=1 --nocapture` |
| Escalation criteria | Escalate if the current release cannot decode its own replay state or lifecycle ordering is ambiguous. |

### Recovery Watchdog Takeover Of Async Work

| Field | Guidance |
| --- | --- |
| Symptom | Root issuer renewal, automatic cycle top-up, placement-receipt acknowledgement, or root Canister-pool maintenance remains in flight after an inter-canister continuation traps or disappears. |
| Likely cause | The asynchronous continuation did not return to its scheduler callback, leaving its durable attempt lease active until bounded takeover. |
| Safety invariant | A live lease coalesces competing work. After expiry, takeover must reuse the same durable operation identity, and a late stale completion must not clear or reschedule the takeover. |
| Safe operator action | Allow the retained recovery watchdog to advance. Inspect the shared timer inventory, the owner's replay or journal state, and the original operation identity before considering manual intervention. |
| Unsafe operator action | Disabling the watchdog, deleting the stable recovery record, forcing a second operation identity, or treating a missing callback response as proof that no external effect occurred. |
| Diagnostic, log, or public error to check | The `canic/async_recovery/watchdog` timer status, owner-specific retry or terminal-failure status, replay receipts, pool journals, and the original operation ID. |
| Retry/idempotency rule | Automatic takeover is bounded by the five-minute lease and reuses the exact operation identity. An operator retry must still preserve the owner-specific replay or journal contract. |
| Relevant validation command | `cargo test --locked -p canic-tests --test timer_authority -- --nocapture` |
| Escalation criteria | Escalate if the watchdog is not scheduled while durable recovery demand exists, operation identity changes across takeover, more than one takeover commits, or current owner state cannot determine whether an external effect completed. |

### Receipt Mismatch Or Unexpected Receipt State

| Field | Guidance |
| --- | --- |
| Symptom | A replay receipt decodes to an unsupported schema, cannot decode, has an unexpected status, or does not match the operator's known operation. |
| Likely cause | Current-schema drift, corrupted state, or an incorrect operation ID/caller/payload assumption. |
| Safety invariant | Unknown receipt state must fail controlled and must not silently re-run the external effect. |
| Safe operator action | Stop retries for that operation. Capture the operation ID, command kind, caller, payload hash if available, receipt status, and same-release lifecycle context for a focused defect investigation. |
| Unsafe operator action | Deleting the receipt, changing the operation ID, or manually marking the operation committed without proof. |
| Diagnostic, log, or public error to check | Receipt decode errors, current-schema errors, stable replay tests, and lifecycle audit entries. |
| Retry/idempotency rule | Unexpected receipt state is an escalation case, not a normal retry case. |
| Relevant validation command | `cargo test --locked -p canic-core storage::stable::replay --lib -- --nocapture` |
| Escalation criteria | Treat decode failure for current-release state or a mismatched committed response as a release blocker candidate. |

## Validation Gates

Runbook close-out uses the documentation guard:

```text
bash scripts/ci/check-recovery-runbooks.sh
```

RC promotion should also account for the focused gates most directly tied to
the runbooks:

```text
cargo test --locked -p canic-core replay_policy --lib -- --nocapture
cargo test --locked -p canic-core --test cost_guard_boundary_guard -- --nocapture
cargo test --locked -p canic-core ops::auth::delegated --lib -- --nocapture
cargo test --locked -p canic-core storage::stable::replay --lib -- --nocapture
cargo test --locked -p canic-cli cycles::convert --lib -- --nocapture
cargo test --locked -p canic-core workflow::ic::icp_refill --lib -- --nocapture
cargo test --locked -p canic-tests --test timer_authority -- --nocapture
cargo test --locked -p canic-tests --test lifecycle_boundary -- --test-threads=1 --nocapture
cargo test --locked -p canic-testing-internal prepared_root_bootstraps_and_reverifies_its_exact_local_store --lib -- --test-threads=1 --nocapture
```

The `canic-tests` and `canic-testing-internal` gates are PocketIC-backed and may
be assigned to CI or an RC validation environment when too expensive for an
ordinary docs slice.

## Non-Goals

- No runtime behavior change.
- No Candid change.
- No CLI output change.
- No JSON/output format change.
- No dependency or lockfile change.
- No generated artifact change.
- No automatic recovery promise beyond behavior already implemented.
- No manual stable-state editing procedure.
- No release of a new replay semantic under the recovery label.

## Outcome Summary

The runbooks remain the operator procedure; they do not issue a release
verdict. Current diagnostic and recovery sufficiency is established by dated
audit evidence and the active release-line closeout.
