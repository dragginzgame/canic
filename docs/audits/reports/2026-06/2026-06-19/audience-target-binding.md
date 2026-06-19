# Audience Target Binding Invariant Audit - 2026-06-19

## Report Preamble

- Scope: delegated-token audience and local-role grant binding, root issuer
  policy audience/grant binding for root proof batch prepare, role-attestation
  audience binding, and structural root capability target hash binding.
- Definition path:
  `docs/audits/recurring/invariants/audience-target-binding.md`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `ef55e53c`
- Branch: `main`
- Method tag/version: `audience-target-binding-current`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-06-19T13:28:38Z`
- Worktree: `dirty`

## Method Changes

This run refreshed the recurring definition before execution. The active
invariant is unchanged, but the surface map no longer names retired
delegated-grant verifier commands or stale `api/rpc/capability/*` paths.
Current coverage explicitly includes root issuer policy checks for root proof
batch prepare and `ops/rpc/capability.rs` as the canonical capability hash
owner.

Release-to-release result is partially comparable with
`docs/audits/reports/2026-06/2026-06-13/audience-target-binding.md`: delegated
token, role-attestation, and capability target-binding outcomes remain
comparable; structural hotspot names changed to match the current 0.68 root
proof provisioning and capability split.

## Executive Summary

Verdict: **PASS**.

No audience/target binding break was found. Delegated-token verification still
requires claims audience to be a subset of the issuer certificate audience,
requires both claim and certificate audiences to match local verifier context,
and resolves local-role grants before required-scope authorization. Root proof
batch prepare is now covered directly: issuer policy rejects unregistered,
disabled, wrong-audience, wrong-grant, and TTL/refresh policy violations before
proof metadata is prepared. Role attestation still rejects audience mismatch at
the endpoint verification path, and root capability hashes still bind the target
canister.

Risk score: **3 / 10**.

Residual risk is fan-in pressure around delegated auth/root provisioning DTOs
and workflow/ops capability split, not a confirmed invariant failure.

## Findings

| ID | Status | Severity | Area | Finding | Resolution |
| --- | --- | --- | --- | --- | --- |
| ATB-2026-06-19-1 | PASS | High | Delegated-token verifier | `verify_audience_and_grants` enforces audience subset, local audience acceptance for claims and cert audiences, grant subset, local-role grant lookup, and required scope checks. | Unit tests passed. |
| ATB-2026-06-19-2 | PASS | High | Delegated-token preparation | `prepare_delegated_token` still rejects audience expansion against the active issuer cert. | Focused unit test passed. |
| ATB-2026-06-19-3 | PASS | High | Root proof provisioning policy | `validate_root_delegation_proof_prepare_policy` and batch preflight reject audience/grant policy violations before root proof batch metadata is prepared. | Root policy and batch preflight tests passed. |
| ATB-2026-06-19-4 | PASS | High | Role attestation verifier | Role-attestation endpoint verification rejects audience mismatch against the local canister. | PocketIC endpoint test passed after unsandboxed retry. |
| ATB-2026-06-19-5 | PASS | Medium | Root capability target binding | `root_capability_hash` binds target canister, capability version, root service, and canonical payload; mismatch verification rejects wrong hashes. | Capability hash unit tests passed. |
| ATB-2026-06-19-6 | INFO | Medium | Audit definition drift | The recurring definition still pointed at retired delegated-grant and stale capability paths before this run. | Definition refreshed before execution. |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/delegated/audience.rs` | `audience_subset`, `audience_accepted`, `role_grants_subset`, `scopes_for_role` | Canonical delegated-token audience and grant matching helpers. | High |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_audience_and_grants` | Central verifier stage before local scope authorization. | High |
| `crates/canic-core/src/ops/auth/delegated/prepare.rs` | `prepare_delegated_token` | Prevents issuer-local token audience/grant expansion. | High |
| `crates/canic-core/src/domain/policy/auth/root_provisioning.rs` | `validate_root_delegation_proof_prepare_policy` | Root issuer registry authorization for proof preparation. | High |
| `crates/canic-core/src/ops/auth/delegation/batch.rs` | root proof batch prepare preflight | Maps request DTOs through root issuer policy decisions before proof metadata creation. | High |
| `crates/canic-core/src/ops/auth/verify/attestation.rs` | `verify_role_attestation_claims` | Enforces role-attestation subject, audience, subnet, epoch, and time bounds. | High |
| `crates/canic-core/src/ops/rpc/capability.rs` | `root_capability_hash` | Owns canonical capability target hash encoding. | Medium |
| `crates/canic-core/src/workflow/rpc/capability/*` | structural proof routing and hash-binding tests | Runtime capability path now accepts structural proof mode only; target binding remains test-covered. | Medium |
| `crates/canic-core/src/dto/auth.rs` | `DelegationAudience`, `DelegatedRoleGrant`, root proof DTOs | Boundary data carries audience, issuer, grant, and proof metadata. | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `ops/auth/delegated` | `dto::auth`, `ids`, verifier cache/cert/canonical helpers | 5 | 2 | 6 |
| `ops/auth/delegation` | root proof DTOs, root issuer policy, auth storage, policy decisions | 5 | 3 | 6 |
| `workflow/rpc/capability` | dto, ops RPC hash facade, replay, metrics, request dispatch | 5 | 4 | 6 |

Pressure remains moderate. The main mitigation from recent cleanup is that
canonical capability hash encoding now lives in `ops/rpc/capability.rs`, while
workflow keeps proof orchestration and structural proof routing.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| delegated-auth fan-in | `dto/auth.rs`, `ops/auth/delegated`, `ops/auth/delegation` | Audience/grant DTO references span access, domain policy, ops, storage, workflow, tests, and protocol-surface checks. | Medium |
| root provisioning policy adjacency | `domain/policy/auth/root_provisioning.rs`, `ops/auth/delegation/batch.rs` | Root proof prepare policy and batch preflight are tightly coupled by design; keep conversions in ops and decisions in policy. | Medium |
| capability split watchpoint | `ops/rpc/capability.rs`, `workflow/rpc/capability/*` | Hash ownership moved to ops; workflow still exposes a compatibility facade and test-only hash-binding verifier. | Low |

## Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `DelegationAudience` | `crates/canic-core/src/dto/auth.rs` | 29 scan-observed files | Medium |
| `RootDelegationAudiencePolicy` | `crates/canic-core/src/domain/policy/auth/root_provisioning.rs` | 16 scan-observed files | Medium |
| `CapabilityProof` | `crates/canic-core/src/dto/capability/mod.rs` | 16 scan-observed files | Low |

## Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `DelegatedRoleGrant` | `dto/auth.rs` | dto, domain policy mapper, ops, storage, workflow/test support | Medium |
| `RootDelegationProofBatchPrepareRequest` | `dto/auth.rs` | endpoint/API, ops, tests, protocol-surface checks | Medium |
| `RoleAttestation` | `dto/auth.rs` | API, workflow, ops, tests | Medium |

## Capability Surface Growth

| Module | Public Items | Risk |
| --- | ---: | --- |
| `ops/auth/delegated/audience.rs` | moderate helper surface | Medium |
| `ops/auth/delegation/mod.rs` | facade over focused root proof provisioning modules | Medium |
| `workflow/rpc/capability/mod.rs` | moderate facade surface | Medium |

No high or critical predictive architectural signal was detected in this run.

## Dependency Fan-In Pressure

Scan evidence:

- Delegated audience/grant terms appear in 29 scan-observed files across access,
  domain policy, DTOs, ops, storage, workflow, tests, and protocol-surface
  checks.
- Root issuer policy/proof prepare terms appear in 17 scan-observed files
  across API, domain policy, DTOs, ops, storage, tests, and root endpoint
  macro surface.
- Capability hash/proof terms appear in 16 scan-observed files across API,
  DTOs, ops, workflow, replay tests, and PocketIC support.

This is expected 0.68 auth/provisioning fan-in and should be watched, but it is
not currently evidence of a binding break.

## Risk Score

Risk Score: **3 / 10**

Derivation:

- `+0` confirmed audience-target binding breaks.
- `+2` for medium/high hotspot contribution around delegated-token and root
  proof provisioning binding surfaces.
- `+1` for enum/struct spread across multiple architecture layers.
- `+0` for hub pressure at or above 7; observed pressure stayed at 6.

## Verification Readout

Structural scans run:

- `rg -n "verify_audience_and_grants|audience_subset|audience_accepted|role_grants_subset|prepare_delegated_token_rejects_audience_expansion|root_prepare_policy_rejects_audience_or_grant_outside_policy|batch_prepare_preflight_rejects_grant_outside_issuer_policy|verify_role_attestation_claims|root_capability_hash_binds_target_canister|verify_capability_hash_binding" crates/canic-core/src crates/canic-tests/tests crates/canic/tests -g '*.rs'`
- `rg -n "api/rpc/capability|verify_root_delegated_grant_claims|capability_endpoint_role_attestation_proof_paths|delegated grant claim verifier|verifier_is_in_audience|verify_audience\\b|mint_delegated_token" docs/audits/recurring/invariants/audience-target-binding.md`
- `rg -l "ops::auth::delegated|auth::delegated|DelegationAudience|DelegatedRoleGrant" crates/canic-core/src crates/canic/src crates/canic/tests crates/canic-tests/tests -g '*.rs'`
- `rg -l "RootIssuerPolicy|RootDelegationAudiencePolicy|RootDelegationProofPreparePolicyInput|RootDelegationProofBatchPrepare" crates/canic-core/src crates/canic/src crates/canic/tests crates/canic-tests/tests -g '*.rs'`
- `rg -l "root_capability_hash|verify_capability_hash_binding|RootCapabilityProof|CapabilityProof::Structural|CAPABILITY_VERSION_V1" crates/canic-core/src crates/canic-tests/tests -g '*.rs'`

Commands:

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test --locked -p canic-core --lib audience -- --nocapture` | PASS | 8 passed; includes audience subset, acceptance, role-grant subset, delegated-token subset drift, non-matching project audience, and token audience expansion rejection due filter overlap. |
| `cargo test --locked -p canic-core --lib local_role -- --nocapture` | PASS | 3 passed; missing local role, local role outside token grants, and required scope outside local-role grant. |
| `cargo test --locked -p canic-core --lib prepare_delegated_token_rejects_audience_expansion -- --nocapture` | PASS | 1 passed. |
| `cargo test --locked -p canic-core --lib root_prepare_policy_rejects_audience_or_grant_outside_policy -- --nocapture` | PASS | 1 passed. |
| `cargo test --locked -p canic-core --lib batch_prepare_preflight_rejects_grant_outside_issuer_policy -- --nocapture` | PASS | 1 passed. |
| `cargo test --locked -p canic-core --lib role_attestation_claims_reject -- --nocapture` | PASS | 3 passed. |
| `cargo test --locked -p canic-core --lib root_capability_hash_binds_target_canister -- --nocapture` | PASS | 1 passed. |
| `cargo test --locked -p canic-core --lib verify_capability_hash_binding -- --nocapture` | PASS | 2 passed. |
| `cargo test --locked -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture` | PASS | Initial sandboxed run was blocked by PocketIC server bind failure; unsandboxed retry passed. |
| `cargo test --locked -p canic-tests --test pic_role_attestation capability_endpoint_policy_and_structural_paths -- --test-threads=1 --nocapture` | PASS | Unsandboxed retry passed. |

## Follow-up Actions

No follow-up actions required.
