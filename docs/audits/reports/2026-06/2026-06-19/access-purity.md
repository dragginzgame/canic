# Access Purity Audit - 2026-06-19

## Report Preamble

- Scope: `crates/canic-core/src/access/**`, endpoint macro access lowering,
  and boundary comparison against ops/workflow/domain policy ownership.
- Compared baseline report path: `N/A`
- Code snapshot: `16894709`
- Method tag/version: `access-purity-current`
- Comparability status: `non-comparable` - the live audit definition was
  refreshed to remove comment-only workflow scan noise and to scan the current
  delegated-session/token-use auth-state surface explicitly.

## Run Context

- Definition: `docs/audits/recurring/system/access-purity.md`
- Previous retained report:
  `docs/audits/reports/2026-06/2026-06-01/access-purity.md`
- Branch: `main`
- Worktree: dirty during audit; unrelated user/session changes were preserved.
- Method: recurring access-boundary checklist plus focused access and endpoint
  macro tests.

## Executive Summary

Risk score: **2 / 10**.

Access remains a thin endpoint boundary. It resolves caller and authenticated
subject identity, performs delegated-token first-argument decoding, evaluates
access expressions, records access metrics through the access facade, and uses
narrow ops-owned auth/session helpers. No production stable-storage leakage,
workflow orchestration, domain policy ownership, broad DTO conversion, or
endpoint macro topology mutation was found.

The only cleanup was in the audit definition itself:

- the workflow drift scan now filters comment-only matches;
- the auth-state scan now explicitly covers delegated-session reads/writes and
  verifier-local token-use store names.

## Findings

### PASS - No Storage Or Stable Type Leakage

Access production code did not import stable storage or persisted record types.
The app and environment guards continue to depend on ops-owned snapshots and
state helpers rather than owning storage schema.

### PASS - No Workflow Orchestration Drift

The workflow scan produced no production code matches after excluding
comment-only lines. Access expression recursion remains local predicate
evaluation, not retry/recovery orchestration or multi-step workflow ownership.

### PASS - No Domain Policy Ownership

No access module defines `*Policy` types or imports the domain policy layer.
Access predicates remain endpoint-boundary rules.

### PASS WITH WATCHPOINT - Delegated Token Boundary Decode

`crates/canic-core/src/access/auth/token.rs` still decodes only the first
ingress argument as `DelegatedToken` with bounded Candid decoding. That remains
acceptable endpoint auth-material parsing and does not own general endpoint
payload conversion.

### PASS WITH WATCHPOINT - Narrow Auth State And Metrics

`access/auth/identity.rs` reads and clears delegated-session state through
`AuthStateOps`. This remains narrow endpoint-boundary identity resolution.
Runtime metric backend calls stay isolated in `access/metrics.rs`; expression
evaluation uses the access metrics facade. The delegated-token verifier guard
test still asserts that access does not introduce verifier-local token-use
storage.

### PASS - Endpoint Macro Lowering

Endpoint macro expansion still resolves authenticated identity, builds an
`AccessContext`, evaluates access expressions, and delegates to the user
handler. Protected internal and authenticated endpoint validation remains
structural; no workflow or topology mutation is hidden in macro output.

## Structural Hotspots

| Hotspot | Status | Evidence |
| --- | --- | --- |
| `access/auth/token.rs` | Accepted | Owns delegated-token first-argument decode plus verify/bind/scope ordering. |
| `access/auth/identity.rs` | Accepted | Owns delegated-session identity fallback and invalid-subject cleanup. |
| `access/expr/mod.rs` | Accepted | Owns access expression short-circuit evaluation and metrics recording. |
| `access/metrics.rs` | Accepted | Only access module allowed to call runtime metric backends directly. |
| `crates/canic-macros/src/endpoint/expand/access.rs` | Accepted | Emits structural access plumbing only. |

## Hub Module Pressure

Pressure score: **2 / 10**.

`crates/canic-core/src/access` currently contains 10 Rust files. Import pressure
is concentrated in `access/auth/token.rs`, `access/auth/identity.rs`, and
`access/expr/mod.rs`, but the dependencies point inward to ops-owned auth,
config, runtime, and storage helpers. No cross-layer ownership inversion was
found.

## Dependency Fan-In Pressure

Fan-in pressure is low. The access module is intentionally called by endpoint
macro output and delegates to ops. The audit found no new access-owned public
DTO conversion surface or storage schema surface.

## Early Warning Signals

- Keep delegated-token decode limited to the first auth argument.
- Keep delegated-session cleanup as fallback hygiene, not recovery workflow.
- Keep runtime metric backend calls behind `access/metrics.rs`.
- Keep endpoint macro output structural and free of proof lifecycle logic.

## Checklist Results

| Checklist Item | Status | Notes |
| --- | --- | --- |
| Storage and stable type leakage | PASS | No production matches. |
| Workflow/orchestration drift | PASS | No production matches after comment filtering. |
| Policy ownership | PASS | No `*Policy` ownership in access. |
| Transport and DTO boundary | PASS with watchpoint | Only bounded delegated-token first-argument Candid decode. |
| Auth state and metrics | PASS with watchpoints | Narrow delegated-session access and access metrics facade use only. |
| Endpoint macro lowering | PASS | Authenticate, evaluate access, delegate. |
| Stale delegated-token audience/version terms | PASS | No stale role-audience or version-specific terms in inspected boundary. |

## Verification Readout

| Check | Result |
| --- | --- |
| Access storage/stable leakage scan | PASS |
| Access workflow/orchestration scan | PASS |
| Access policy ownership scan | PASS |
| Access transport/DTO scan | PASS |
| Access auth-state/metrics scan | PASS |
| Endpoint macro lowering scan | PASS |
| Stale delegated-token audience/version scan | PASS |
| `cargo test --locked -p canic-core access:: --lib -- --nocapture` | PASS |
| `cargo test --locked -p canic-macros endpoint --lib -- --nocapture` | PASS |

Known non-fatal warning during the `canic-core` focused test: delegated-auth
metrics still emit unfulfilled lint expectation warnings under this test
configuration. That warning is already tracked as a focused lint/hygiene
watchpoint.

## Follow-up Actions

No required follow-up action from this audit. Keep the existing delegated-auth
metrics lint-expectation cleanup watchpoint.

## Final Verdict

Pass with watchpoints - access remains a thin endpoint boundary.
