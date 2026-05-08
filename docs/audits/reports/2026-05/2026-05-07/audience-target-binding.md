# Audience Target Binding Invariant Audit - 2026-05-07

## Report Preamble

- Scope: delegated-token audience binding, role-attestation audience binding, root capability proof target binding, delegated grant target binding, and outbound RPC attestation target selection.
- Definition path: `docs/audits/recurring/invariants/audience-target-binding.md`
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-05/audience-target-binding.md`
- Code snapshot identifier: `6e72960b`
- Method tag/version: `Method V4.1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-07T17:22:38Z`
- Branch: `main`
- Worktree: `dirty`
- Dirty files observed: `CHANGELOG.md`, `Makefile`, `crates/canic-cli/**`, `crates/canic-host/**`, `crates/canic/tests/reference_surface.rs`, `docs/changelog/0.32.md`, `scripts/app/reference_canisters.sh`, `scripts/ci/**`, `scripts/dev/README.md`

## Executive Summary

Verdict: **PASS**.

The audience-target binding invariant still holds. The code continues to bind
delegated credentials to the intended verifier or target canister at each
security boundary:

- Delegated-token certs and claims both carry explicit audiences.
- Delegated-token verification enforces claim audience subset, token verifier
  membership, and cert verifier membership.
- Role attestations carry an optional `audience`, but inter-service issuance now
  requires it, and verification rejects mismatches against the local canister.
- Root capability proof hashes include the target canister, service, capability
  version, and canonical request payload.
- Delegated grants bind issuer and audience to the target canister, and subject
  to the caller.
- Outbound RPC requests ask root for attestations scoped to the target canister
  and cache them only while all subject, role, epoch, root, and audience fields
  still match.

The main drift since the April baseline is structural, not behavioral:
`api/auth/proof_store` no longer exists as a separate proof-store hotspot. The
target binding previously attributed to that path now lives in the root
capability verifier path, especially `api/rpc/capability/proof.rs` and
`api/rpc/capability/verifier.rs`.

Risk score: **3 / 10**.

No immediate code fix is required. The residual risk is reference-radius and
test-name drift, not a detected invariant break.

## Audit Question

Can a delegated credential, role attestation, delegated grant, or capability
proof minted for one audience or canister be replayed against a different
audience or canister?

Expected answer: **no**.

## Method

This run repeated the April 2026 `audience-target-binding` invariant audit with
the same method tag and scope:

1. Enumerate all DTO fields carrying audience, target, issuer, subject, and
   capability hash data.
2. Trace issuance paths to verify that audiences are explicitly selected.
3. Trace verification paths to verify that local canister identity and caller
   identity are checked against proof contents.
4. Check root capability envelope hashing and delegated-grant claims.
5. Check outbound RPC code so generated attestations are scoped to the target
   canister rather than to root or a generic fleet audience.
6. Run targeted unit and PocketIC tests that exercise audience mismatch,
   audience expansion, required-audience enforcement, and capability hash
   mismatch behavior.
7. Compare findings against
   `docs/audits/reports/2026-04/2026-04-05/audience-target-binding.md`.

Commands used included:

- `rg "aud|audience|target_canister|issuer|DelegationProof|RoleAttestation|verify_self_audience|validate_claims_against_cert|delegated_grant|grant_claim" crates/canic-core crates/canic-tests crates/canic -n`
- `find crates/canic-core/src -path '*auth*' -o -path '*capability*' | sort`
- `rg "RoleAttestation|DelegationProof|Audience|audience|target" crates/canic-core/src/dto crates/canic-core/src/api crates/canic-core/src/ops crates/canic-core/src/workflow -n`
- `rg -l "RoleAttestation" crates/canic-core crates/canic-tests crates/canic -g '*.rs' | wc -l`
- `rg -l "DelegationProof" crates/canic-core crates/canic-tests crates/canic -g '*.rs' | wc -l`
- `rg -l "DelegationAudience" crates/canic-core crates/canic-tests crates/canic -g '*.rs' | wc -l`
- `rg -l "CapabilityProof" crates/canic-core crates/canic-tests crates/canic -g '*.rs' | wc -l`
- Targeted `cargo test` commands listed in the Verification Readout.

## Surface Inventory

### Credential DTOs

`crates/canic-core/src/dto/auth.rs` remains the canonical delegated auth wire
surface:

- `DelegationAudience` allows `Roles`, `Principals`, or `RolesOrPrincipals`.
- `DelegationCert` carries `aud` and `verifier_role_hash`.
- `DelegatedTokenClaims` carries its own `aud`.
- `DelegationProofIssueRequest`, `DelegatedTokenIssueRequest`, and
  `DelegatedTokenMintRequest` all carry requested audiences.
- `RoleAttestationRequest` and `RoleAttestation` carry a required
  `audience: Principal`.

Key evidence:

- `crates/canic-core/src/dto/auth.rs:16` defines `DelegationAudience`.
- `crates/canic-core/src/dto/auth.rs:67` defines `DelegationCert`.
- `crates/canic-core/src/dto/auth.rs:83` stores cert `aud`.
- `crates/canic-core/src/dto/auth.rs:84` stores cert `verifier_role_hash`.
- `crates/canic-core/src/dto/auth.rs:101` defines `DelegatedTokenClaims`.
- `crates/canic-core/src/dto/auth.rs:109` stores claims `aud`.
- `crates/canic-core/src/dto/auth.rs:170` defines `RoleAttestationRequest`.
- `crates/canic-core/src/dto/auth.rs:176` stores requested role-attestation `audience`.
- `crates/canic-core/src/dto/auth.rs:188` defines `RoleAttestation`.
- `crates/canic-core/src/dto/auth.rs:194` stores attestation `audience`.

`crates/canic-core/src/dto/capability/proof.rs` remains the canonical capability
proof wire surface:

- `RoleAttestationProof` carries a `capability_hash`.
- `DelegatedGrant` carries `issuer`, `subject`, `audience`, `scope`,
  `capability_hash`, quota, timing, and epoch.
- `DelegatedGrantProof` carries the outer `capability_hash`, grant, signature,
  and key id.

### Implementation Hotspots

Current implementation hotspots are:

- `crates/canic-core/src/ops/auth/delegated/audience.rs`
- `crates/canic-core/src/ops/auth/delegated/issue.rs`
- `crates/canic-core/src/ops/auth/delegated/mint.rs`
- `crates/canic-core/src/ops/auth/delegated/verify.rs`
- `crates/canic-core/src/ops/auth/verify/attestation.rs`
- `crates/canic-core/src/api/rpc/capability/proof.rs`
- `crates/canic-core/src/api/rpc/capability/verifier.rs`
- `crates/canic-core/src/api/rpc/capability/grant.rs`
- `crates/canic-core/src/ops/rpc/mod.rs`
- `crates/canic-core/src/workflow/rpc/request/handler/authorize.rs`

Removed or moved hotspot:

- The April report referenced `crates/canic-core/src/api/auth/proof_store/mod.rs`.
  That path is no longer present. The equivalent target-binding behavior is now
  visible in `api/rpc/capability/proof.rs` and
  `api/rpc/capability/verifier.rs`.

## Findings

| ID | Status | Severity | Area | Finding | Evidence |
| --- | --- | --- | --- | --- | --- |
| ATB-1 | PASS | High | Delegated-token DTOs | Delegated certs and delegated-token claims both carry explicit audiences; the token cannot rely on an implicit global target. | `dto/auth.rs:83`, `dto/auth.rs:109` |
| ATB-2 | PASS | High | Delegated-token verification | Verification requires token audience to be a subset of cert audience and requires the local verifier to be inside both audiences. | `ops/auth/delegated/verify.rs:201`, `verify.rs:219`, `verify.rs:222`, `verify.rs:225` |
| ATB-3 | PASS | High | Role-scoped delegated auth | Role audiences require a local role and require `verifier_role_hash` to match the local role hash. | `ops/auth/delegated/verify.rs:210`, `verify.rs:214`; `ops/auth/delegated/audience.rs:33` |
| ATB-4 | PASS | High | Delegated-token minting | Minting rejects audience expansion beyond the root-issued cert audience. | `ops/auth/delegated/mint.rs:100`, `mint.rs:101` |
| ATB-5 | PASS | High | Role attestation issuance | Inter-service role-attestation issuance refuses requests without an audience. | `workflow/rpc/request/handler/authorize.rs:153` |
| ATB-6 | PASS | High | Role attestation verification | Verification rejects an attestation whose audience is present and differs from the local canister id. | `ops/auth/verify/attestation.rs:32` |
| ATB-7 | PASS | High | Capability hash binding | Root capability proof hashes bind target canister, service, capability version, and canonical request bytes. | `api/rpc/capability/proof.rs:69`, `proof.rs:75`; `ops/rpc/mod.rs:380` |
| ATB-8 | PASS | High | Delegated grants | Delegated-grant claims require issuer to equal target, subject to equal caller, and audience to include target canister. | `api/rpc/capability/grant.rs:51`, `grant.rs:56`, `grant.rs:61` |
| ATB-9 | PASS | Medium | Outbound RPC | Non-cycle outbound RPCs request a root role attestation for the target pid, not a generic root audience. | `ops/rpc/mod.rs:140`, `ops/rpc/mod.rs:174`, `ops/rpc/mod.rs:178` |
| ATB-10 | PASS | Medium | Attestation cache | Cached outbound attestations are reused only when root, audience, subject, role, epoch, payload audience, and expiry still match. | `ops/rpc/mod.rs:327`, `ops/rpc/mod.rs:338`, `ops/rpc/mod.rs:345` |
| ATB-11 | PASS | Medium | Capability verifier routing | Role-attestation and delegated-grant proof modes both verify capability hash binding before proof-specific verification. | `api/rpc/capability/verifier.rs:63`, `verifier.rs:91` |
| ATB-12 | PASS | Medium | Wire headers | Role-attestation and delegated-grant proof blobs reject header/payload `capability_hash` mismatch. | `api/rpc/capability/proof.rs:112`, `proof.rs:146` |

## Detailed Evidence

### Delegated-token audience checks

The delegated auth audience helper now centralizes most audience-shape logic.
It rejects empty audiences, anonymous-principal audiences, multi-role cert
audiences where a single verifier role hash is required, and role-hash drift:

- `validate_audience_shape` checks all `DelegationAudience` variants.
- `expected_role_hash_for_cert_audience` derives the verifier role hash.
- `validate_cert_role_hash` compares the stored cert role hash with the
  expected role hash.
- `verifier_is_in_audience` checks either principal membership or role
  membership.
- `audience_subset` ensures token claims cannot expand beyond the cert.

Key evidence:

- `crates/canic-core/src/ops/auth/delegated/audience.rs:19`
- `crates/canic-core/src/ops/auth/delegated/audience.rs:33`
- `crates/canic-core/src/ops/auth/delegated/audience.rs:47`
- `crates/canic-core/src/ops/auth/delegated/audience.rs:67`
- `crates/canic-core/src/ops/auth/delegated/audience.rs:86`
- `crates/canic-core/src/ops/auth/delegated/audience.rs:130`
- `crates/canic-core/src/ops/auth/delegated/audience.rs:158`

Delegated-token verification performs timing and cert consistency checks before
audience checks, then checks audience and scopes before accepting the token:

- `verify_claims` checks issuer shard, cert hash, token TTL, cert window, token
  window, audience, and scopes.
- `verify_audience` requires a local role for role audiences.
- `verify_audience` rejects cert/token role-hash mismatch.
- `verify_audience` rejects token audience expansion.
- `verify_audience` rejects a local verifier outside the token audience.
- `verify_audience` rejects a local verifier outside the cert audience.

Key evidence:

- `crates/canic-core/src/ops/auth/delegated/verify.rs:168`
- `crates/canic-core/src/ops/auth/delegated/verify.rs:175`
- `crates/canic-core/src/ops/auth/delegated/verify.rs:188`
- `crates/canic-core/src/ops/auth/delegated/verify.rs:194`
- `crates/canic-core/src/ops/auth/delegated/verify.rs:201`
- `crates/canic-core/src/ops/auth/delegated/verify.rs:210`
- `crates/canic-core/src/ops/auth/delegated/verify.rs:219`
- `crates/canic-core/src/ops/auth/delegated/verify.rs:222`
- `crates/canic-core/src/ops/auth/delegated/verify.rs:225`

### Delegated-token issuance and minting

Root-side delegated proof issuance computes the cert verifier role hash from the
requested cert audience before signing:

- `prepare_delegation_cert` calls `expected_role_hash_for_cert_audience`.
- The resulting `DelegationCert` stores both `aud` and `verifier_role_hash`.

Token minting from a proof validates the requested token audience shape and
requires it to be a subset of the cert audience before building signed claims.

Key evidence:

- `crates/canic-core/src/ops/auth/delegated/issue.rs:94`
- `crates/canic-core/src/ops/auth/delegated/issue.rs:110`
- `crates/canic-core/src/ops/auth/delegated/issue.rs:111`
- `crates/canic-core/src/ops/auth/delegated/mint.rs:100`
- `crates/canic-core/src/ops/auth/delegated/mint.rs:101`
- `crates/canic-core/src/ops/auth/delegated/mint.rs:113`

### Role-attestation issuance and verification

Follow-up work after this audit hard-cut role attestations to encode
`audience: Principal` instead of `audience: Option<Principal>`. Missing audience
is now rejected by the wire/API shape rather than by a later policy branch.
Verification rejects any mismatch against the local canister principal.

Key evidence:

- `crates/canic-core/src/ops/auth/verify/attestation.rs:32`
- `crates/canic-tests/tests/pic_role_attestation_cases/capability.rs:99`
- `crates/canic-tests/tests/pic_role_attestation_cases/capability.rs:116`
- `crates/canic-tests/tests/pic_role_attestation_cases/verification.rs:37`

### Root capability target binding

Root capability hashes bind the target canister directly into the signed or
attested payload context. Both proof modes verify this before accepting the
proof:

- Role-attestation proof mode decodes the proof, verifies the capability hash
  against the local target canister, then verifies the role attestation.
- Delegated-grant proof mode decodes the proof, verifies the capability hash
  against the local target canister, checks grant/proof hash binding, and then
  checks grant claims/signature.

Key evidence:

- `crates/canic-core/src/api/rpc/capability/proof.rs:69`
- `crates/canic-core/src/api/rpc/capability/proof.rs:75`
- `crates/canic-core/src/api/rpc/capability/proof.rs:112`
- `crates/canic-core/src/api/rpc/capability/proof.rs:146`
- `crates/canic-core/src/api/rpc/capability/verifier.rs:63`
- `crates/canic-core/src/api/rpc/capability/verifier.rs:91`
- `crates/canic-core/src/api/rpc/capability/verifier.rs:122`
- `crates/canic-core/src/api/rpc/capability/verifier.rs:123`

The outbound caller computes the same hash over target canister, service,
capability version, and canonical request. That keeps request metadata out of
the capability hash while still binding the security-relevant target and
payload fields.

Key evidence:

- `crates/canic-core/src/ops/rpc/mod.rs:203`
- `crates/canic-core/src/ops/rpc/mod.rs:380`
- `crates/canic-core/src/ops/rpc/mod.rs:385`

### Delegated-grant target binding

Delegated grants have explicit claim-level checks:

- `grant.issuer == target_canister`
- `grant.subject == caller`
- `grant.audience.contains(target_canister)`
- `grant.scope.service == CapabilityService::Root`
- `grant.scope.capability_family == capability.family().label()`
- grant quota and time window are valid

This prevents using a grant minted for another target canister or another
caller, even if the grant blob is otherwise well formed.

Key evidence:

- `crates/canic-core/src/api/rpc/capability/grant.rs:51`
- `crates/canic-core/src/api/rpc/capability/grant.rs:56`
- `crates/canic-core/src/api/rpc/capability/grant.rs:61`
- `crates/canic-core/src/api/rpc/capability/grant.rs:66`
- `crates/canic-core/src/api/rpc/capability/grant.rs:71`
- `crates/canic-core/src/api/rpc/capability/grant.rs:88`
- `crates/canic-core/src/api/rpc/capability/grant.rs:93`

### Outbound RPC audience selection

For non-cycle outbound RPCs, `RpcOps::execute_response_rpc` requests a root
role attestation for the target pid. The request explicitly sets
`audience: audience_pid`, where `audience_pid` is the target canister.

Cached attestations are only accepted if all identity and audience dimensions
still match:

- root pid
- audience pid
- subject pid
- role
- epoch
- attestation payload subject
- attestation payload role
- attestation payload audience
- expiry

Key evidence:

- `crates/canic-core/src/ops/rpc/mod.rs:132`
- `crates/canic-core/src/ops/rpc/mod.rs:140`
- `crates/canic-core/src/ops/rpc/mod.rs:150`
- `crates/canic-core/src/ops/rpc/mod.rs:174`
- `crates/canic-core/src/ops/rpc/mod.rs:178`
- `crates/canic-core/src/ops/rpc/mod.rs:327`
- `crates/canic-core/src/ops/rpc/mod.rs:338`
- `crates/canic-core/src/ops/rpc/mod.rs:345`

## Drift From April Baseline

### Stable

- The main delegated auth audience invariant is stable.
- Role-attestation verification still rejects audience mismatch.
- Delegated grants still bind issuer, subject, and audience.
- Capability hashes still bind target canister.
- Runtime coverage still includes PocketIC role-attestation/capability paths.

### Changed

- `crates/canic-core/src/api/auth/proof_store/mod.rs` no longer exists. The
  current proof target-binding path is under `api/rpc/capability`.
- Delegated auth audience logic is more localized than in the April report:
  `ops/auth/delegated/audience.rs` owns shape, subset, membership, and role-hash
  helpers.
- Two April test filters no longer match any tests:
  - `audience_helpers_reject_claim_outside_cert_audience`
  - `verify_role_attestation_claims_rejects_audience_mismatch`
- Replacement tests now cover the same meaningful behavior:
  - `verify_delegated_token_rejects_audience_subset_drift`
  - `verify_delegated_token_rejects_missing_local_role_for_role_audience`
  - `role_attestation_verification_paths`
  - `capability_endpoint_role_attestation_proof_paths`

## Reference Radius

Current reference counts across `crates/canic-core`, `crates/canic-tests`, and
`crates/canic`:

| Symbol | Current files | April baseline | Delta | Readout |
| --- | ---: | ---: | ---: | --- |
| `RoleAttestation` | 37 | 35 | +2 | Slightly wider. Expected after bootstrap/config/test surface growth, but still worth watching. |
| `DelegationProof` | 10 | 25 | -15 | Improved localization. Delegated proof handling is more concentrated than baseline. |
| `DelegationAudience` | 13 | N/A | N/A | Reasonable current radius; centralized helpers reduce risk. |
| `CapabilityProof` | 13 | N/A | N/A | Reasonable current radius; proof modes are localized under capability APIs/tests. |

`RoleAttestation` remains the widest surface. That is acceptable today because
the main issuance and verification gates are explicit, but this is still the
first area to watch when changing auth, bootstrap config, or runtime tests.

## Residual Risks

| Risk | Severity | Detail | Recommendation |
| --- | --- | --- | --- |
| Role-attestation audience is now a hard DTO requirement | Low | Follow-up work removed the optional wire shape. Missing audience now fails at construction/decode rather than later policy. | Keep PocketIC audience-mismatch coverage. |
| Test-name drift | Low | Two April audit test filters now run zero tests. Replacement coverage exists, but recurring audits should track current test names. | Update future audit runbooks to use current replacement tests. |
| RoleAttestation reference radius | Low | `RoleAttestation` appears in 37 files. This is broad enough that future edits can miss one verification path. | Keep target/audience checks centralized in `ops/auth/verify` and capability verifier code. |
| Proof-store path removed | Low | April report references a path that no longer exists. The behavior appears preserved in capability proof verifier code. | Future audits should treat `api/rpc/capability/proof.rs` and `verifier.rs` as the proof-binding hotspot. |

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `git rev-parse --short HEAD` | PASS | Snapshot id recorded as `6e72960b`. |
| `git status --short` | PASS | Confirmed the audit ran against a dirty worktree with active CLI/host/script/changelog work. |
| `rg "aud|audience|target_canister|issuer|DelegationProof|RoleAttestation|verify_self_audience|validate_claims_against_cert|delegated_grant|grant_claim" crates/canic-core crates/canic-tests crates/canic -n` | PASS | Located current audience/target binding surfaces. |
| `find crates/canic-core/src -path '*auth*' -o -path '*capability*' \| sort` | PASS | Confirmed current auth/capability path inventory and absence of old proof-store module. |
| `rg -l "RoleAttestation" crates/canic-core crates/canic-tests crates/canic -g '*.rs' \| wc -l` | PASS | Current count: `37`. |
| `rg -l "DelegationProof" crates/canic-core crates/canic-tests crates/canic -g '*.rs' \| wc -l` | PASS | Current count: `10`. |
| `rg -l "DelegationAudience" crates/canic-core crates/canic-tests crates/canic -g '*.rs' \| wc -l` | PASS | Current count: `13`. |
| `rg -l "CapabilityProof" crates/canic-core crates/canic-tests crates/canic -g '*.rs' \| wc -l` | PASS | Current count: `13`. |
| `cargo test -p canic-core --lib audience_helpers_reject_claim_outside_cert_audience -- --nocapture` | BLOCKED | Filter matched zero tests; this April audit test name no longer exists. |
| `cargo test -p canic-core --lib verify_role_attestation_claims_rejects_audience_mismatch -- --nocapture` | BLOCKED | Filter matched zero tests; behavior is covered by current replacement tests below. |
| `cargo test -p canic-core --lib verify_root_delegated_grant_claims_rejects_audience_mismatch -- --nocapture` | PASS | `api::rpc::capability::tests::verify_root_delegated_grant_claims_rejects_audience_mismatch` passed. |
| `cargo test -p canic-core --lib verify_delegated_token_rejects_audience_subset_drift -- --nocapture` | PASS | Delegated token rejects claim audience expansion beyond cert audience. |
| `cargo test -p canic-core --lib verify_delegated_token_rejects_missing_local_role_for_role_audience -- --nocapture` | PASS | Role audience verification requires local verifier role. |
| `cargo test -p canic-core --lib authorize_rejects_role_attestation_when_audience_missing -- --nocapture` | BLOCKED | Follow-up work removed the missing-audience runtime branch because audience is now required by the DTO. |
| `cargo test -p canic-core --lib mint_delegated_token_rejects_audience_expansion -- --nocapture` | PASS | Minting rejects requested audience expansion beyond proof cert audience. |
| `cargo test -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture` | PASS | PocketIC path rejected subject mismatch, audience mismatch, epoch rejection, and expiry. |
| `cargo test -p canic-tests --test pic_role_attestation capability_endpoint_role_attestation_proof_paths -- --test-threads=1 --nocapture` | PASS | PocketIC capability endpoint accepted valid proof and rejected tampered signature, capability hash mismatch, audience mismatch, and expiry. |

## Follow-up Actions

No immediate code fix is required.

Carry-forward watch items:

1. Owner boundary: `canic-core` auth/capability maintainers.
   Action: Keep role-attestation issuance checks and verifier checks centralized.
   Target report date/run: next `audience-target-binding` recurring run.
2. Owner boundary: audit maintenance.
   Action: Replace stale April test filters with current test names in future
   runbooks and recurring audit prompts.
   Target report date/run: next `audience-target-binding` recurring run.
3. Owner boundary: audit maintenance.
   Action: Future runs should treat required role-attestation audience as the
   expected shape and focus runtime testing on mismatch rejection.
   Target report date/run: next `audience-target-binding` recurring run.
