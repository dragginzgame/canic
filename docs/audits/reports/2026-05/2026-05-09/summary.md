# Audit Summary - 2026-05-09

## Run Contexts

| Report | Type | Scope | Snapshot | Worktree | Status |
| --- | --- | --- | --- | --- | --- |
| `docs-dry-consolidation.md` | Ad hoc | maintained docs, root docs, recurring audit templates | working tree | dirty | complete |
| `audit-template-maintenance.md` | Ad hoc | recurring audit templates and audit governance docs | working tree | dirty | complete |
| `auth-abstraction-equivalence.md` | Recurring invariant | macro auth expansion, access evaluator, delegated-token verifier parity, delegated sessions | `518f57dd` | dirty | complete |
| `bootstrap-lifecycle-symmetry.md` | Recurring system | lifecycle macros, core lifecycle adapters, root control-plane scheduling, optional init-block timer path | `518f57dd` | dirty | complete |
| `canonical-auth-boundary.md` | Recurring invariant | generated auth wrappers, canonical verifier ordering, public auth helper surfaces | `518f57dd` | dirty | complete |
| `capability-scope-enforcement.md` | Recurring invariant | delegated-token scopes, capability proofs, delegated grants, workflow authorization | `518f57dd` | dirty | complete |
| `expiry-replay-single-use.md` | Recurring invariant | delegated-token freshness, update-token single-use, capability replay metadata, root replay cache expiry | `518f57dd` | dirty | complete |
| `subject-caller-binding.md` | Recurring invariant | delegated-token subject binding, generated auth context, delegated sessions, role-attestation caller checks | `518f57dd` | dirty | complete |

## Risk Index Summary

| Report | Risk | Readout |
| --- | ---: | --- |
| `docs-dry-consolidation.md` | 4 / 10 | Current operator docs were mostly aligned; the audit identified stale recurring-template artifact and canister-layout vocabulary, then follow-up cleanup completed it. |
| `audit-template-maintenance.md` | 2 / 10 | Audit templates were broadly usable; cleanup removed stale layout wording, review-note prose, and replay freshness gaps. |
| `auth-abstraction-equivalence.md` | 3 / 10 | Invariant holds; generated authenticated endpoints still converge on the canonical evaluator and delegated-token verifier, with residual risk in concentrated macro/access/auth surfaces. |
| `bootstrap-lifecycle-symmetry.md` | 2 / 10 | Invariant holds after remediation; optional macro init blocks now run through zero-delay lifecycle timers before scheduling bootstrap continuation timers. |
| `canonical-auth-boundary.md` | 3 / 10 | Generated endpoint auth still reaches the full canonical boundary; the public partial `AuthApi::verify_token` helper found during the audit was removed in follow-up. |
| `capability-scope-enforcement.md` | 3 / 10 | Invariant holds after remediation; proof validation, verifier dispatch, logs, and metrics now share a validated proof view, with residual risk in broad capability DTO/workflow surfaces. |
| `expiry-replay-single-use.md` | 3 / 10 | Invariant holds after remediation; capability replay metadata and root replay cache records now expire at the same exclusive boundary as delegated tokens. |
| `subject-caller-binding.md` | 3 / 10 | Invariant holds; delegated-token subject binding remains canonical, and generated access context preserves separate transport-caller and authenticated-subject lanes. |

## Key Findings by Severity

### Medium

- `docs-dry-consolidation.md`: recurring audit templates still hardcode
  DFX-era artifact paths and should be consolidated around current ICP CLI
  artifact vocabulary.
- `docs-dry-consolidation.md`: canister layout guidance is split between
  `README.md`, `TESTING.md`, and recurring audit canonical maps.
- `canonical-auth-boundary.md`: public `canic::api::auth::AuthApi::verify_token`
  verified token material and scopes but could not perform endpoint subject
  binding or update-token consumption; follow-up removed that public helper and
  kept token-material verification private.
- `capability-scope-enforcement.md`: capability proof-mode dispatch previously
  required repeated raw DTO matching across validation, verifier routing, logs,
  and metrics; follow-up remediation centralized this behind
  `RootCapabilityProof` and `RootCapabilityProofMode`.
- `expiry-replay-single-use.md`: capability replay metadata and existing root
  replay records previously accepted `now == expires_at`; follow-up remediation
  changed both paths to the exclusive expiry boundary used by delegated tokens.
- `bootstrap-lifecycle-symmetry.md`: optional `start!` / `start_root!`
  `init = { ... }` blocks previously ran synchronously inside lifecycle hook
  bodies; follow-up remediation moved them behind zero-delay lifecycle timers.

### Low

- `docs-dry-consolidation.md`: old operations/design docs retain DFX-era command
  flows and should get historical banners instead of being rewritten as current
  docs.
- `docs-dry-consolidation.md`: current README/config/status docs are mostly
  aligned with the named-fleet and ICP CLI direction.
- `audit-template-maintenance.md`: audit how-to layout wording, lifecycle audit
  commentary prose, freshness invariant replay details, and fleet/test/audit
  terminology needed small maintenance updates.
- `auth-abstraction-equivalence.md`: no auth bypass or helper-specific failure
  branch was found; remaining risk is watchpoint pressure around macro access
  generation, `AccessContext` caller lanes, and delegated-token verifier order.
- `subject-caller-binding.md`: no subject/caller bypass was found; remaining
  risk is watchpoint pressure around the two identity lanes in `AccessContext`
  and the private token-material verifier helper.

## Verification Rollup

| Report | PASS | BLOCKED | FAIL | Notes |
| --- | ---: | ---: | ---: | --- |
| `docs-dry-consolidation.md` | 3 | 0 | 0 | Read-only grep/size scans were recorded in the report. |
| `audit-template-maintenance.md` | 3 | 0 | 0 | Recurring audit template scans and whitespace validation passed. |
| `auth-abstraction-equivalence.md` | 13 | 0 | 0 | 9 targeted cargo test commands plus 4 symbol/fan-in scans passed. |
| `bootstrap-lifecycle-symmetry.md` | 11 | 0 | 0 | 5 lifecycle scans, formatting, `canic` tests, 3 targeted PocketIC lifecycle/fixture tests, and `canic` clippy passed. |
| `canonical-auth-boundary.md` | 17 | 0 | 0 | 8 targeted cargo test commands, 7 entrypoint/fan-in/edit-pressure/remediation scans, and 2 post-remediation build checks passed. |
| `capability-scope-enforcement.md` | 19 | 0 | 0 | 8 original targeted cargo test commands, 6 scope/capability/fan-in/edit-pressure scans, capability module tests, auth identity tests, storage helper tests, and `canic-core` clippy passed for lib and all targets. |
| `expiry-replay-single-use.md` | 17 | 0 | 0 | 10 targeted cargo test commands, `canic-core` clippy, and 6 freshness/replay fan-in/edit-pressure scans passed. |
| `subject-caller-binding.md` | 11 | 0 | 0 | 7 targeted cargo test commands and 4 subject/caller lane scans passed. |

## Follow-up Actions

Status: docs cleanup items completed; auth items are standing watchpoints.

1. Audit maintenance: update recurring audit templates to use current ICP CLI
   artifact paths and reference a single canonical build-artifact vocabulary.
2. Docs maintenance: make `TESTING.md` the canonical owner for non-fleet
   test/audit canister placement, then point README and audit maps at it.
3. Docs maintenance: add historical banners to old operations/design docs that
   still show DFX-era command flows.
4. `audit-template-maintenance.md`: no follow-up actions required.
5. `auth-abstraction-equivalence.md`: keep macro access generation,
   `AccessContext`, and delegated-token verifier ordering aligned when changing
   authenticated endpoint syntax or delegated-session behavior.
6. `canonical-auth-boundary.md`: keep private
   `AuthApi::verify_token_material(...)` private unless a future public helper
   performs the full endpoint boundary, including subject binding and update
   replay.
7. `bootstrap-lifecycle-symmetry.md`: completed optional init-block remediation
   by moving macro `init = { ... }` execution behind zero-delay lifecycle
   timers.
8. `bootstrap-lifecycle-symmetry.md`: keep optional init-block support behind
   lifecycle timers and re-run after changes to `start.rs`, lifecycle adapters,
   root control-plane scheduling, or role-attestation fixture setup.
9. `capability-scope-enforcement.md`: completed proof-mode remediation by
   routing validation, dispatch, logs, and metrics through
   `RootCapabilityProof` / `RootCapabilityProofMode`.
10. `capability-scope-enforcement.md`: keep `CapabilityProof`,
   `CapabilityService`, and capability envelope DTO changes coordinated across
   API, ops, workflow, metrics, and tests.
11. `expiry-replay-single-use.md`: completed expiry-boundary remediation by
   changing capability metadata projection and root replay cache classification
   to reject `now >= expires_at`.
12. `expiry-replay-single-use.md`: keep root replay metadata, delegated-token
    use markers, and session-bootstrap replay policy aligned on the same
    exclusive expiry convention.
13. `subject-caller-binding.md`: keep transport caller and authenticated
    subject lane semantics explicit when editing `AccessContext`, endpoint
    macro generation, or delegated-session resolution.
14. `subject-caller-binding.md`: keep private
    `AuthApi::verify_token_material(...)` private unless a future public helper
    performs subject binding, scope enforcement, and update replay consumption.
