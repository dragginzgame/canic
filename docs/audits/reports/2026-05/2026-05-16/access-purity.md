# Access Purity Audit - 2026-05-16

## Run Context

- Definition: `docs/audits/recurring/system/access-purity.md`
- Related baseline:
  `docs/audits/reports/2026-05/2026-05-16/ops-purity.md`
- Snapshot: `92ec102b`
- Branch: `main`
- Worktree: dirty
- Method: V1.0, focused access responsibility scan
- Scope: `crates/canic-core/src/access/**`, with comparisons against ops,
  domain policy, workflow, and endpoint macros

## Executive Summary

Initial risk: **4 / 10**.

Post-remediation risk: **3 / 10**.

The audit found two concrete access-boundary leaks:

- app-mode endpoint guards imported stable storage `AppMode` directly;
- the whitelist predicate read the raw `Config` model directly.

Both were moved behind ops helpers. Access still owns endpoint authorization
messages and subject/caller binding, but it no longer depends on those storage
or configuration internals.

## Findings

### FIXED - App Guards Imported Stable App Mode

Severity: **Medium**.

`access/app.rs` matched on `storage::stable::state::app::AppMode`. That made an
endpoint guard depend on a stable storage type.

Remediation:

- Added narrow `AppStateOps::{is_query_allowed,is_update_allowed,is_readonly}`
  helpers.
- Updated app access guards to ask ops for app-mode facts while preserving the
  existing access-denial messages.
- Added a layering guard that rejects production access imports of stable
  storage or record types.

### FIXED - Whitelist Predicate Read Raw Config

Severity: **Low**.

`access/auth/predicates.rs` called `Config::try_get()` directly for whitelist
checks. That bypassed the ops configuration boundary.

Remediation:

- Added `ConfigOps::is_whitelisted`.
- Updated the whitelist access predicate to use `ConfigOps`.

### ACCEPTED - Access Expressions Short-Circuit Predicate Lists

Severity: **Low**.

`access/expr` loops over `All` and `Any` predicates. This is accepted access
evaluation behavior, not workflow orchestration or retry/recovery.

### ACCEPTED - Delegated Token Boundary Decode

Severity: **Watchpoint**.

`access/auth/token.rs` decodes the delegated token from the first ingress
argument using bounded Candid decoding. This is acceptable boundary
unmarshalling. Keep the scope limited to the auth token; endpoint payload
parsing belongs elsewhere.

### ACCEPTED - Delegated Session Cleanup

Severity: **Watchpoint**.

`access/auth/identity.rs` clears invalid delegated sessions after rejecting a
stored subject that resolves to infrastructure/canister identity. This is a
narrow auth-boundary cleanup. It should not grow into broader auth-state
recovery or lifecycle behavior.

## Checklist Results

### Storage And Stable Type Leakage

Status: **Pass after remediation**.

Access production code no longer imports stable storage or record types.
Test-only fixtures in `access/expr/mod.rs` still import stable records under
`#[cfg(test)]`.

### Workflow / Orchestration Drift

Status: **Pass with accepted expression loops**.

No workflow calls, retry loops, backoff, recovery, or cross-domain
orchestration were found in access production code. The only loop-like
behavior is access-expression predicate evaluation.

### Policy Ownership

Status: **Pass**.

No domain policy definitions or `*Policy` types were found in access
production code.

### Transport And DTO Boundary

Status: **Pass with delegated-token watchpoint**.

The only transport parsing hotspot is bounded delegated-token first-argument
decode. No broad endpoint payload parsing or DTO conversion ownership was
found.

### Auth State And Metrics

Status: **Pass with watchpoints**.

Access emits denial metrics through `access/metrics.rs`. Delegated-token replay
consumption and delegated-session cleanup remain narrow auth-boundary state
calls.

## Residual Watchpoints

| Area | Risk | Note |
| --- | --- | --- |
| `access/auth/token.rs` | Medium | Must preserve verify, subject bind, scope check, update-token consume order. |
| `access/auth/identity.rs` | Medium | Invalid delegated-session cleanup must stay narrow and not become recovery orchestration. |
| `access/expr/mod.rs` | Low | Expression composition is acceptable, but avoid business predicates that hide workflow rules. |
| `access/metrics.rs` | Low | Keep runtime metrics backend hidden behind the access metrics facade. |

## Verification Readout

| Check | Result |
| --- | --- |
| Access stable/storage scan | PASS after remediation |
| Access workflow/orchestration scan | PASS with accepted expression loops |
| Access policy ownership scan | PASS |
| Layering guard | PASS |

## Final Verdict

Pass with watchpoints.

Access remains a thin endpoint boundary after removing stable app-mode and raw
config reads. The main long-term pressure is delegated auth: keep token decode,
verification ordering, subject binding, and replay consumption explicit and
covered by tests.
