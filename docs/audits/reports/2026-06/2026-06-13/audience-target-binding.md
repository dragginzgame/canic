# Audience Target Binding Invariant Audit - 2026-06-13

## Report Preamble

- Scope: delegated-token audience binding, delegated-token local-role grant
  binding, role-attestation audience binding, root capability target hashing,
  and current root capability proof-mode routing.
- Definition path:
  `docs/audits/recurring/invariants/audience-target-binding.md`
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-29/audience-target-binding.md`
- Code snapshot identifier: `ea21d8a0`
- Branch: `main`
- Method tag/version: `Method V4.1-current-surface`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp: `2026-06-13`
- Worktree: `dirty`

Comparability note: delegated-token and role-attestation checks remain
comparable with the prior run. The prior report's internal-invocation proof
surface and delegated-grant verifier surface were not found in current scans.
The active root capability endpoint now routes only structural proof mode at
runtime; target hash binding remains covered by unit tests as a helper surface.

## Executive Summary

Verdict: **PASS**.

The current audience-target binding invariant still holds for active verifier
surfaces. Delegated-token verification requires claim audience subset, local
runtime audience acceptance for both claim and cert audience, and local-role
grant lookup before required-scope authorization. Role attestations still bind
`audience` to the local canister before endpoint acceptance. Capability target
hashing still binds the target canister in the canonical hash helper, while the
active runtime capability endpoint currently accepts structural proof mode only.

Risk score: **3 / 10**.

No blocker was found. The main watchpoint is method drift: older audit wording
still names internal-invocation and delegated-grant surfaces that are absent
from current code scans.

## Findings

| ID | Status | Severity | Area | Finding | Resolution |
| --- | --- | --- | --- | --- | --- |
| ATB-2026-06-13-1 | PASS | High | Delegated-token verifier | `verify_audience_and_grants` still checks claims audience subset, local acceptance for claims and cert audiences, grant subset, and local-role grants before scope authorization. | Existing unit tests passed. |
| ATB-2026-06-13-2 | PASS | High | Delegated-token preparation | Token preparation still rejects audience expansion against the issuer cert. | Existing unit tests passed. |
| ATB-2026-06-13-3 | PASS | High | Role attestation verifier | `verify_role_attestation_claims` rejects audience mismatch against the local canister before successful endpoint verification. | PocketIC endpoint test passed and logged the expected audience-mismatch rejection. |
| ATB-2026-06-13-4 | PASS | Medium | Root capability hashing | `root_capability_hash` and `verify_capability_hash_binding` still bind target canister and reject mismatched hashes. | Existing unit tests passed. |
| ATB-2026-06-13-5 | INFO | Medium | Audit surface drift | Current scans did not find the prior internal-invocation proof or delegated-grant verifier surfaces. Runtime root capability proof routing currently accepts structural proof mode only. | Record as method drift; no active acceptance bypass found. |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/delegated/audience.rs` | `validate_audience_shape`, `audience_subset`, `audience_accepted`, `role_grants_subset`, `scopes_for_role` | Canonical audience shape, subset, local runtime matching, and grant lookup helpers. | High |
| `crates/canic-core/src/ops/auth/delegated/prepare.rs` | `prepare_delegated_token` path | Rejects token audience expansion before minting delegated tokens. | High |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_audience_and_grants` | Enforces audience subset, local audience acceptance, grant subset, and local-role grant binding before scope authorization. | High |
| `crates/canic-core/src/ops/auth/verify/attestation.rs` | `verify_role_attestation_claims` | Enforces subject, time, audience, subnet, and epoch bounds for role attestations. | High |
| `crates/canic-core/src/workflow/runtime/auth/mod.rs` | `verify_role_attestation`, rejection logging | Runtime bridge around role-attestation verification and failure telemetry. | Medium |
| `crates/canic-core/src/workflow/rpc/capability/hash.rs` | `root_capability_hash` | Canonical target-canister hash binding helper. | Medium |
| `crates/canic-core/src/workflow/rpc/capability/proof.rs` | `verify_capability_hash_binding`, structural proof helpers | Test-visible target hash verification and runtime structural proof checks. | Medium |
| `crates/canic-core/src/workflow/rpc/capability/verifier.rs` | `verify_root_capability_proof` | Runtime proof-mode routing for root capability envelopes. | Medium |
| `crates/canic-core/src/workflow/rpc/capability/root.rs` | `response_capability_v1_root` | Validates envelope, verifies proof, then dispatches root capability request. | Medium |
| `crates/canic-core/src/dto/auth.rs` | delegated token and role-attestation DTOs | Carries audience, subject, issuer, grant, and expiry claims. | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `ops/auth/delegated` | `dto::auth`, `ids`, `ops::auth`, canonical auth helpers | 4 | 2 | 6 |
| `workflow/runtime/auth` | config, ops auth, runtime metrics, logging | 4 | 3 | 5 |
| `workflow/rpc/capability` | dto, ops storage/ic, replay, metrics, request handler | 5 | 4 | 6 |

Pressure is moderate, not blocking. The main coupling risk is that capability
proof helpers live under workflow while some of their hash/codec behavior is
lower-level boundary logic.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| stale audit terminology | recurring definition and 2026-05 baseline | Current scans found no `internal_invocation` or root delegated-grant verifier surface. | Medium |
| proof-mode surface contraction | `workflow/rpc/capability/verifier.rs` | Runtime verifier currently matches only `RootCapabilityProof::Structural`. | Low |
| auth verifier edit activity | recent history under `crates/canic-core/src/ops/auth` | Recent commits touched auth verifier and token modules, so targeted invariant tests remain important. | Medium |

## Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `DelegationAudience` | `crates/canic-core/src/dto/auth.rs` | scan-observed across delegated auth modules | Medium |
| `RootCapabilityProof` | `crates/canic-core/src/workflow/rpc/capability` / DTO boundary | scan-observed in capability verifier and endpoint path | Low |

## Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `RoleAttestation` | `dto/auth.rs` | api, workflow, ops, tests | Medium |
| `DelegatedTokenClaims` | `dto/auth.rs` | ops, workflow/test support | Medium |
| `RootCapabilityEnvelopeV1` | `dto/capability` | api/workflow, tests | Medium |

## Capability Surface Growth

| Module | Public Items | Risk |
| --- | ---: | --- |
| `ops/auth/delegated/audience.rs` | moderate helper surface | Medium |
| `workflow/rpc/capability/mod.rs` | moderate facade surface | Medium |
| `ops/auth/verify/attestation.rs` | small verifier surface | Low |

No critical predictive architectural signal was detected in this run.

## Verification Readout

Selection evidence:

- Recurring report scan showed several audits tied at `2026-05-29`; the first
  alphabetical tied audit was `audience-target-binding`.
- Baseline compared:
  `docs/audits/reports/2026-05/2026-05-29/audience-target-binding.md`.

Structural scans run:

- `rg -n 'verify_audience|validate_audience|audience_subset|verifier_is_in_audience|verify_role_attestation_claims|verify_capability_hash_binding|verify_root_capability_proof|verify_root_delegated_grant_claims|authorize_' crates/canic-core/src crates/canic-core/tests crates/canic-tests/tests -g '*.rs'`
- `rg -n 'aud|audience|target_canister|issuer|subject|DelegatedToken|RoleAttestation|capability|grant' crates/canic-core/src -g '*.rs'`
- `rg -n 'internal_invocation|InternalInvocation|invocation proof|InvocationProof' crates/canic-core/src crates/canic-tests/tests -g '*.rs'`
- `rg -n 'root_delegated|delegated_grant|grant_claims|audience_mismatch|hash_binding|capability_hash' crates/canic-core/src crates/canic-tests/tests -g '*.rs'`
- `git log --name-only -n 20 -- crates/canic-core/src crates/canic-tests/tests crates/canic-host/src`

Commands passed:

- `cargo test --locked -p canic-core --lib audience -- --nocapture`
  - 7 passed; includes audience acceptance, subset, audience expansion, and
    delegated-token subset drift.
- `cargo test --locked -p canic-core --lib local_role -- --nocapture`
  - 3 passed; includes missing local role, local role outside token grants, and
    required scope outside local-role grant.
- `cargo test --locked -p canic-core --lib role_attestation_claims_reject -- --nocapture`
  - 3 passed; covers time-window rejection paths.
- `cargo test --locked -p canic-core --lib root_capability_hash_binds_target_canister -- --nocapture`
  - 1 passed.
- `cargo test --locked -p canic-core --lib verify_capability_hash_binding -- --nocapture`
  - 2 passed.
- `cargo test --locked -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture`
  - 1 passed; endpoint path rejected subject mismatch, audience mismatch,
    epoch floor, and expiry.
- `cargo test --locked -p canic-tests --test pic_role_attestation capability_endpoint_policy_and_structural_paths -- --test-threads=1 --nocapture`
  - 1 passed; current structural capability endpoint path succeeds.

Command reconciliation:

- The recurring definition still lists historical filters
  `verify_delegated_token_rejects_missing_local_role_for_role_audience`,
  `mint_delegated_token_rejects_audience_expansion`, and
  `capability_endpoint_role_attestation_proof_paths`.
- Current equivalent filters are `local_role`, `audience`, and
  `capability_endpoint_policy_and_structural_paths`.

## Residual Risk

No blocker remains. The next run should either update the recurring audit
definition to match the current active surfaces or keep recording the drift
explicitly. Re-run this audit after changes to `dto/auth.rs`,
`ops/auth/delegated`, `ops/auth/verify/attestation.rs`, or
`workflow/rpc/capability`.
