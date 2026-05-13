# Audience Target Binding Invariant Audit - 2026-05-13

## Report Preamble

| Field | Value |
| --- | --- |
| Audit | `audience-target-binding` |
| Type | Recurring invariant |
| Method | `Method V4.1` |
| Auditor | `codex` |
| Timestamp | `2026-05-13T12:21:49Z` |
| Snapshot | `c533afd6` plus current `0.35.4` worktree |
| Worktree | dirty |
| Baseline | `docs/audits/reports/2026-05/2026-05-07/audience-target-binding.md` |
| Comparability | comparable |
| Verdict | PASS |
| Risk | 3 / 10 |

## Executive Summary

The audience-target binding invariant still holds. Role attestations, delegated
tokens, and capability proofs keep audience/target checks in the verification
path, and the targeted unit plus PocketIC checks reject the expected subject,
audience, hash, and expiry mismatches.

Risk stays at `3 / 10`. The main residual risk is structural rather than a
confirmed correctness defect: the auth/capability surface still spans DTOs,
ops, API verification, RPC workflow, and PocketIC support, so config vocabulary
or role/audience schema changes need the same targeted regression lane.

The current config vocabulary is strict: current configuration uses
`[auth.delegated_tokens]`, `[auth.role_attestation]`, and per-canister
`auth.role_attestation_cache` / `auth.delegated_token_signer` flags. Obsolete
per-canister verifier tables are not accepted by the schema.

## Audit Question

Can a valid-looking credential, proof, or capability envelope be replayed or
retargeted to a canister, role, or capability target that was not its intended
audience?

## Method

- Compared the current code against the 2026-05-07 audience-target-binding
  report and reran the current recurring evidence set.
- Scanned DTO, delegated-auth, role-attestation, capability-proof, and RPC
  request paths for audience, target, capability-hash, and role-hash checks.
- Checked config/docs vocabulary after the current schema hard cuts.
- Ran targeted unit tests for delegated-token audience narrowing and
  role-audience hash checks.
- Ran the PocketIC role-attestation and capability proof rejection paths.

## Invariant Readout

| Area | Status | Evidence |
| --- | --- | --- |
| Role-attestation local audience | PASS | `verify_role_attestation_claims` rejects `payload.audience != self_pid` in `crates/canic-core/src/ops/auth/verify/attestation.rs:32`. |
| Role-attestation subject binding | PASS | The same verifier rejects `payload.subject != caller` in `crates/canic-core/src/ops/auth/verify/attestation.rs:16`. |
| Delegation certificate audience shape | PASS | `validate_audience_shape` rejects empty role/principal audiences in `crates/canic-core/src/ops/auth/delegated/audience.rs:19`. |
| Delegation role audience hash | PASS | Role audiences require a matching role hash through `expected_role_hash_for_cert_audience` and `validate_cert_role_hash` in `crates/canic-core/src/ops/auth/delegated/audience.rs:33` and `:47`. |
| Delegated token audience narrowing | PASS | `verify_audience` requires token audience to be a subset of cert audience in `crates/canic-core/src/ops/auth/delegated/verify.rs:219`. |
| Delegated verifier membership | PASS | Token and certificate audiences must both include the local verifier in `crates/canic-core/src/ops/auth/delegated/verify.rs:222` and `:225`. |
| Delegated role verifier identity | PASS | Role audiences require a local role and cert role hash match in `crates/canic-core/src/ops/auth/delegated/verify.rs:210` and `:214`. |
| Capability payload hash binding | PASS | Capability proofs verify canonical target/version/request hash binding in `crates/canic-core/src/api/rpc/capability/proof.rs:69`. |
| Capability proof wire self-consistency | PASS | Role-attestation and delegated-grant proof blobs reject mismatched wire/payload hashes in `crates/canic-core/src/api/rpc/capability/proof.rs:112` and `:146`. |
| Delegated grant target binding | PASS | Grant issuer, subject, audience, and capability family are checked in `crates/canic-core/src/api/rpc/capability/grant.rs:51`, `:56`, `:61`, and `:71`. |
| Capability verifier ordering | PASS | Capability hash verification runs before role-attestation verification or grant verification in `crates/canic-core/src/api/rpc/capability/verifier.rs:58` and `:83`. |
| Outbound root response attestation audience | PASS | RPC requests ask root for an attestation whose `audience` is the target canister in `crates/canic-core/src/ops/rpc/mod.rs:150` and `:174`. |
| Cached root response attestation audience | PASS | Cache hits require matching root, audience, subject, role, epoch, and payload audience in `crates/canic-core/src/ops/rpc/mod.rs:327`. |
| Role-attestation request DTO shape | PASS | `RoleAttestationRequest.audience` is required, not optional, in `crates/canic-core/src/dto/auth.rs:170`. |

## Structural Hotspots

| Surface | Current pressure | Risk |
| --- | --- | --- |
| `crates/canic-core/src/ops/auth/delegated/*` | Audience shape, role-hash, certificate/token narrowing, and verifier membership checks are split by helper modules but still form one semantic unit. | Medium |
| `crates/canic-core/src/api/rpc/capability/*` | Capability hash binding, proof decoding, delegated grant checks, and verifier dispatch are correctly separated, but a capability DTO change still crosses several files. | Medium |
| `crates/canic-core/src/ops/rpc/mod.rs` | Outbound response capability calls own target-attestation acquisition and a small cache keyed by audience, subject, role, epoch, and root. | Medium |
| `crates/canic-tests/tests/pic_role_attestation_*` | End-to-end audience evidence is strongest here; keeping it fast and named to current semantics matters. | Medium |
| `docs/design/0.28-delegated-auth-lifecycle/*` | The old design text was updated to use the current per-canister `[auth]` flags for role-attestation cache behavior. | Low |

## Hub Pressure

| Module | Why it matters | Pressure |
| --- | --- | --- |
| `ops/auth/delegated/verify.rs` | Combines signature/certificate checks with token validity, audience narrowing, role-hash validation, and scope subset checks. | Medium |
| `api/rpc/capability/verifier.rs` | Routes structural, role-attestation, and delegated-grant proof modes. Ordering is correct, but every new proof mode must preserve hash-first behavior. | Medium |
| `ops/rpc/mod.rs` | Combines RPC dispatch, root attestation acquisition, cache validation, and capability hash construction for outbound calls. | Medium |
| `dto/auth.rs` | Carries shared Candid contracts for delegation, tokens, role attestations, and signed keys. Boundary changes have broad fan-out. | Medium |

## Cross-Layer Spread

| Shape | Observed references | Readout |
| --- | ---: | --- |
| `DelegationAudience` | 13 files across DTO, ops, workflow, access, and tests | Expected for a boundary contract; changes need auth-focused tests. |
| `RoleAttestation*` | 34 files across DTO, config/bootstrap, API, ops, workflow, canister stubs, and tests | Broad but coherent; DTO requires audience and verifier code checks it locally. |
| `CapabilityProof` / `DelegatedGrant` / `capability_hash` | 20 files across DTO, API, ops, metrics, and tests | Moderate capability surface; hash-binding helpers remain centralized. |

## Early Warning Signals

- No confirmed audience-bypass bug was found.
- Obsolete per-canister verifier tables are rejected by the live schema.
- The role-attestation request authorization path checks caller, registration,
  role, subnet, and TTL, while the audience is a required DTO field and is
  verified by receivers. That split is intentional, but future request builders
  should continue setting audience from the exact target canister, not from a
  caller-provided default.
- Capability proof verification must keep capability-hash binding before
  accepting either role-attestation or delegated-grant modes.

## Dependency Fan-In Pressure

| Dependency / concept | Fan-in concern | Recommendation |
| --- | --- | --- |
| `DelegationAudience` | Token/cert audience schema changes can affect minting, verification, access guards, and tests. | Treat new audience variants as security changes; add unit tests for subset and local-verifier membership before merging. |
| `RoleAttestationRequest.audience` | Request builders own the target audience, while root verifies caller/role/subnet/TTL and receivers verify audience. | Keep builder helpers explicit about target principal; avoid optional/default audience fields. |
| `CapabilityProofBlob.capability_hash` | Hash mismatch rejection is the shared protection against retargeted proof payloads. | Keep hash canonicalization centralized and forbid verifier modes from bypassing it. |
| `auth.delegated_tokens` config | Downstream stale verifier-table snippets now fail parsing. | Keep docs pointing to current config tables and per-canister `auth` flags. |

## Verification Readout

| Command | Result | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib verify_root_delegated_grant_claims_rejects_audience_mismatch -- --nocapture` | PASS | Delegated grant audience mismatch is rejected. |
| `cargo test -p canic-core --lib verify_delegated_token_rejects_audience_subset_drift -- --nocapture` | PASS | Token audience cannot expand beyond certificate audience. |
| `cargo test -p canic-core --lib verify_delegated_token_rejects_missing_local_role_for_role_audience -- --nocapture` | PASS | Role audiences require local role context. |
| `cargo test -p canic-core --lib mint_delegated_token_rejects_audience_expansion -- --nocapture` | PASS | Minting rejects audience expansion. |
| `cargo test -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture` | PASS | PocketIC rejected subject mismatch, audience mismatch, stale epoch, and expiry. |
| `cargo test -p canic-tests --test pic_role_attestation capability_endpoint_role_attestation_proof_paths -- --test-threads=1 --nocapture` | PASS | PocketIC accepted a valid proof and rejected tampered signature, capability hash mismatch, audience mismatch, and expiry. |

## Comparability

This run is comparable with the 2026-05-07 report. The current worktree has
endpoint/access-control cleanup and audit probe changes in flight, but the
audience-target code paths under review are the same delegated-auth,
role-attestation, RPC capability, and proof verification surfaces covered by
the prior run.

The previous report had two blocked stale test filters. This run used current
filters, so the verification lane is stronger than the baseline while remaining
method-compatible.

## Risk Score

Risk: `3 / 10`

- `+0`: targeted and PocketIC evidence passed.
- `+1`: capability and role-attestation semantics still cross DTO/API/ops/
  workflow boundaries.
- `+1`: downstream config authors can still copy obsolete snippets from old
  local branches or external docs.
- `+1`: proof-mode additions remain sensitive to verifier ordering and hash
  canonicalization.

## Follow-up Actions

1. Keep `CONFIG.md` and active architecture docs explicit that current config
   uses `[auth.delegated_tokens]`, `[auth.role_attestation]`, and per-canister
   `auth` flags.
2. Rerun this audit after any `DelegationAudience`, `RoleAttestationRequest`,
   `CapabilityProof`, or delegated-grant shape change.
3. Add a focused test before introducing any new capability proof mode that
   proves capability-hash binding still runs before mode-specific acceptance.
