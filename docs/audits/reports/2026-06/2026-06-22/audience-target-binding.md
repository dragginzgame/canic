# Audience Target Binding Invariant Audit - 2026-06-22

## Report Preamble

- Scope: delegated-token audience and local-role grant binding, active root
  delegation proof issuer/root binding, root issuer policy audience/grant
  binding for batch prepare, role-attestation audience binding, and structural
  root capability target hash binding.
- Definition path:
  `docs/audits/recurring/invariants/audience-target-binding.md`
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/audience-target-binding.md`
- Code snapshot identifier: `4bcad983`
- Branch: `main`
- Method tag/version: `audience-target-binding-current`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-06-22T11:38:54Z`
- Worktree: `dirty`

## Method Changes

This run refreshed the recurring definition before execution. The invariant is
unchanged, but current coverage now explicitly includes active root delegation
proof installation:

- signer install must reject a proof whose certificate issuer is not the local
  signer canister
- signer install must verify the proof against the configured trusted root
  canister/root key before storing active proof state
- root batch install preflight must reject retrieved proofs whose issuer,
  certificate hash, or certificate payload do not match pending batch metadata

Release-to-release result is partially comparable with the 2026-06-19 run:
delegated-token, root issuer policy, role-attestation, and capability target
binding outcomes remain comparable; active-proof install and batch-install
metadata matching are now first-class method checks.

## Executive Summary

Verdict: **PASS**.

No audience/target binding break was found. Delegated-token preparation and
verification still prevent audience/grant expansion and require local audience
acceptance. Active root delegation proof install now has direct audit coverage
for issuer self-binding and trusted-root verification before state is stored.
Root proof batch prepare remains policy-gated by issuer registry audience,
grant, TTL, and refresh constraints, and batch install rejects proof/pending
metadata mismatches. Role attestation still rejects wrong audiences at the
endpoint verification path, and root capability hashes still bind target
canister context.

Risk score: **3 / 10**.

Residual risk is fan-in pressure around delegated auth/root provisioning DTOs
and the capability workflow/ops split, not a confirmed invariant failure.

## Findings

| ID | Status | Severity | Area | Finding | Resolution |
| --- | --- | --- | --- | --- | --- |
| ATB-2026-06-22-1 | PASS | High | Delegated-token verifier | `verify_audience_and_grants` enforces audience subset, local audience acceptance for claims and cert audiences, grant subset, local-role grant lookup, and required scope checks. | Unit tests passed. |
| ATB-2026-06-22-2 | PASS | High | Delegated-token preparation | `prepare_delegated_token` rejects audience expansion against the active issuer certificate. | Focused unit test passed. |
| ATB-2026-06-22-3 | PASS | High | Active root delegation proof install | Signer install rejects wrong certificate issuer and proof verification failure against trusted root context. | Focused active-proof tests passed. |
| ATB-2026-06-22-4 | PASS | High | Root proof batch install | Batch install preflight rejects submitted proofs whose issuer, certificate hash, or certificate payload do not match pending metadata. | Focused batch preflight test passed. |
| ATB-2026-06-22-5 | PASS | High | Root proof provisioning policy | Root issuer policy rejects audience/grant policy violations before root proof batch metadata is prepared. | Root policy and batch preflight tests passed. |
| ATB-2026-06-22-6 | PASS | High | Role attestation verifier | Role-attestation endpoint verification rejects audience mismatch against the local canister. | PocketIC endpoint test passed after unsandboxed retry. |
| ATB-2026-06-22-7 | PASS | Medium | Root capability target binding | `root_capability_hash` binds target canister, capability version, root service, and canonical payload; mismatch verification rejects wrong hashes. | Capability hash unit and PocketIC endpoint tests passed. |
| ATB-2026-06-22-8 | INFO | Medium | Audit definition drift | The recurring definition did not yet name active root proof install issuer/root binding or batch install proof/pending metadata matching. | Definition refreshed before execution. |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/delegated/audience.rs` | `audience_subset`, `audience_accepted`, `role_grants_subset`, `scopes_for_role` | Canonical delegated-token audience and grant matching helpers. | High |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_audience_and_grants` | Central verifier stage before local scope authorization. | High |
| `crates/canic-core/src/ops/auth/delegated/prepare.rs` | `prepare_delegated_token` | Prevents issuer-local token audience/grant expansion. | High |
| `crates/canic-core/src/ops/auth/delegated/active_proof.rs` | `install_active_delegation_proof` | Binds active proof certificate issuer to the local signer canister and verifies trusted root proof before storage. | High |
| `crates/canic-core/src/ops/auth/delegation/active.rs` | `install_active_delegation_proof` | Supplies current canister and trusted root verifier context before persisting active proof state. | High |
| `crates/canic-core/src/domain/policy/auth/root_provisioning.rs` | `validate_root_delegation_proof_prepare_policy` | Root issuer registry authorization for proof preparation. | High |
| `crates/canic-core/src/ops/auth/delegation/batch.rs` | `preflight_delegation_proof_batch_prepare_request` | Maps request DTOs through root issuer policy decisions before proof metadata creation. | High |
| `crates/canic-core/src/ops/auth/delegation/batch.rs` | `preflight_delegation_proof_batch_install_proof` | Rejects proof values that do not match pending batch metadata. | High |
| `crates/canic-core/src/ops/auth/verify/attestation.rs` | `verify_role_attestation_claims` | Enforces role-attestation subject, audience, subnet, epoch, and time bounds. | High |
| `crates/canic-core/src/ops/rpc/capability.rs` | `root_capability_hash` | Owns canonical capability target hash encoding. | Medium |
| `crates/canic-core/src/workflow/rpc/capability/*` | structural proof routing and hash-binding tests | Runtime capability path accepts structural proof mode; target binding remains test-covered. | Medium |
| `crates/canic-core/src/dto/auth.rs` | `DelegationAudience`, `DelegatedRoleGrant`, root proof DTOs | Boundary data carries audience, issuer, grant, and proof metadata. | Medium |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| delegated-auth fan-in | `dto/auth.rs`, `ops/auth/delegated`, `ops/auth/delegation` | Audience/grant/root proof DTO references span access, domain policy, ops, storage, workflow, tests, and protocol-surface checks. | Medium |
| active proof install authority | `ops/auth/delegated/active_proof.rs`, `ops/auth/delegation/active.rs` | Local signer principal and trusted root verifier context must stay explicit at install. | Medium |
| root provisioning policy adjacency | `domain/policy/auth/root_provisioning.rs`, `ops/auth/delegation/batch.rs` | Root proof prepare policy and batch preflight are tightly coupled by design; keep conversions in ops and decisions in policy. | Medium |
| capability split watchpoint | `ops/rpc/capability.rs`, `workflow/rpc/capability/*` | Hash ownership lives in ops; workflow still orchestrates structural proof routing. | Low |

## Dependency Fan-In Pressure

Scan evidence:

- Delegated audience/grant terms appear in 25 scan-observed files across access,
  DTOs, ops, storage, workflow, tests, and protocol-surface checks.
- Root issuer policy/proof prepare terms appear in 19 scan-observed files
  across API, domain policy, DTOs, ops, storage, workflow, tests, and root
  endpoint macro surface.
- Capability hash/proof terms appear in 16 scan-observed files across API,
  DTOs, ops, workflow, replay tests, and PocketIC support.
- The combined binding evidence scan reported 518 references across delegated
  audience/grant, root issuer policy, root proof, and capability proof terms.

This remains expected auth/provisioning fan-in and should be watched, but it is
not currently evidence of a binding break.

## Risk Score

Risk Score: **3 / 10**

Derivation:

- `+0` confirmed audience-target binding breaks.
- `+2` for medium/high hotspot contribution around delegated-token, active
  proof install, and root proof provisioning binding surfaces.
- `+1` for enum/struct spread across multiple architecture layers.
- `+0` for high-pressure hub growth beyond the existing watchpoints.

## Verification Readout

Structural scans run:

- `rg -n 'verify_audience_and_grants|audience_subset|audience_accepted|role_grants_subset|prepare_delegated_token_rejects_audience_expansion|install_active_delegation_proof_rejects_wrong_issuer|install_active_delegation_proof_rejects_root_proof_failure|root_prepare_policy_rejects_audience_or_grant_outside_policy|batch_prepare_preflight_rejects_grant_outside_issuer_policy|batch_install_preflight_rejects_proof_mismatch|verify_role_attestation_claims|root_capability_hash_binds_target_canister|verify_capability_hash_binding' crates/canic-core/src crates/canic-tests/tests crates/canic/tests -g '*.rs'`
- `rg -n 'api/rpc/capability|verify_root_delegated_grant_claims|capability_endpoint_role_attestation_proof_paths|delegated grant claim verifier|verifier_is_in_audience|verify_audience\b|mint_delegated_token' docs/audits/recurring/invariants/audience-target-binding.md`
- `rg -l 'ops::auth::delegated|auth::delegated|DelegationAudience|DelegatedRoleGrant' crates/canic-core/src crates/canic/src crates/canic/tests crates/canic-tests/tests -g '*.rs'`
- `rg -l 'RootIssuerPolicy|RootDelegationAudiencePolicy|RootDelegationProofPreparePolicyInput|RootDelegationProofBatchPrepare|preflight_delegation_proof_batch_install_proof|InstallActiveDelegationProofInput' crates/canic-core/src crates/canic/src crates/canic/tests crates/canic-tests/tests -g '*.rs'`
- `rg -l 'root_capability_hash|verify_capability_hash_binding|RootCapabilityProof|CapabilityProof::Structural|CAPABILITY_VERSION_V1' crates/canic-core/src crates/canic-tests/tests -g '*.rs'`
- `rg -n 'issuer_pid != input.this_canister|InvalidRootAuthority|verify_root_canister_signature_proof|proof.proof.cert.issuer_pid != proof.issuer_pid|pending.prepared.cert != proof.proof.cert|cert_hash != proof.cert_hash|pending.prepared.cert_hash != proof.cert_hash' crates/canic-core/src/ops/auth -g '*.rs'`

Commands:

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test --locked -p canic-core --lib audience -- --nocapture` | PASS | 8 passed; includes audience subset, local audience acceptance, role-grant subset, and overlapping delegated-token preparation checks. |
| `cargo test --locked -p canic-core --lib local_role -- --nocapture` | PASS | 3 passed. |
| `cargo test --locked -p canic-core --lib prepare_delegated_token_rejects_audience_expansion -- --nocapture` | PASS | 1 passed. |
| `cargo test --locked -p canic-core --lib install_active_delegation_proof_rejects_wrong_issuer -- --nocapture` | PASS | 1 passed. |
| `cargo test --locked -p canic-core --lib install_active_delegation_proof_rejects_root_proof_failure -- --nocapture` | PASS | 1 passed. |
| `cargo test --locked -p canic-core --lib root_prepare_policy_rejects_audience_or_grant_outside_policy -- --nocapture` | PASS | 1 passed. |
| `cargo test --locked -p canic-core --lib batch_prepare_preflight_rejects_grant_outside_issuer_policy -- --nocapture` | PASS | 1 passed. |
| `cargo test --locked -p canic-core --lib batch_install_preflight_rejects_proof_mismatch -- --nocapture` | PASS | 1 passed. |
| `cargo test --locked -p canic-core --lib role_attestation_claims_reject -- --nocapture` | PASS | 3 passed. |
| `cargo test --locked -p canic-core --lib root_capability_hash_binds_target_canister -- --nocapture` | PASS | 1 passed. |
| `cargo test --locked -p canic-core --lib verify_capability_hash_binding -- --nocapture` | PASS | 2 passed. |
| `POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic cargo test --locked -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture` | PASS | Initial sandboxed run failed because PocketIC could not bind a local server socket; unsandboxed retry passed. |
| `POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic cargo test --locked -p canic-tests --test pic_role_attestation capability_endpoint_policy_and_structural_paths -- --test-threads=1 --nocapture` | PASS | Initial sandboxed run failed because PocketIC could not bind a local server socket; unsandboxed retry passed. |

## Follow-up Actions

- Keep active proof issuer/root binding and batch install proof/pending metadata
  matching in recurring coverage.
- Continue watching delegated-auth DTO fan-in as root proof provisioning and
  delegated-token issuance evolve.
