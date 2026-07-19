# Audit: Token Trust Chain Invariant

## Method Contract

- Audit ID: `CANIC-AUTH-TRUST-001`
- Method version: `1`
- Disposition: `revise`
- Owner: configured root, issuer proof, canister signature, and token trust
  chain
- Kind/profile: security `invariant`
- Trace mode: `code_trace`; focused invalid-proof execution may use PocketIC
- Cost/runtime: medium/high; 30-90 minutes excluding PocketIC fixtures
- Prerequisites: Git, ripgrep, root/issuer/token proof code, trust config,
  generated interfaces, and invalid/mismatched/expired proof fixtures
- False-positive boundary: configured trust anchors and verified proof links
  are authoritative; convenient topology or caller assumptions are not
- Shared contract: [AUDIT-HOWTO.md](../../AUDIT-HOWTO.md)

## Purpose

Ensure delegated-token acceptance requires a complete verified chain from the
configured root authority to an issuer canister to the delegated-token claims.

## Risk Model / Invariant

A delegated token must be rejected unless the complete chain validates:

```text
configured IC root key + configured root canister id
  -> root canister signature over canonical delegation cert hash
  -> cert issuer and issuer-proof binding
  -> issuer canister signature over canonical claims hash
  -> claims bound to the certified delegation cert
```

Freshness, replay, subject binding, and audience binding have dedicated
invariant audits. This audit may cite those checks where they are part of token
acceptance, but a finding is scored here only when an invalid trust chain can
pass or when trust-chain evidence is ambiguous.

## Why This Matters

Subject binding and freshness are insufficient if an untrusted issuer, wrong
root canister, wrong IC root key, noncanonical certificate, or forged issuer
proof can produce an accepted delegated token.

## Run This Audit After

- root or issuer proof format changes
- canister-signature verification changes
- canonical cert/claims encoding changes
- active root delegation proof install/status changes
- delegated-token verifier or positive-cache changes
- delegated-token config or root-key handling changes
- endpoint macro or access guard ordering changes
- role-attestation proof verification changes

## Report Preamble (Required)

Every report generated from this audit must include:

- Scope
- Compared baseline report path
- Code snapshot identifier
- Method tag/version
- Comparability status

## Audit Checklist

### 1. Identify Trust Anchors

Search terms:

```text
AuthProofVerifierConfig
auth_proof_verifier_config
root_canister_id
ic_root_public_key_raw
BuildNetwork
MAINNET_IC_ROOT_PUBLIC_KEY_RAW
validate_build_network_root_key_pair
```

Confirm:

- the verifier uses explicit configured root canister id and raw IC root public
  key material
- `ic` requires the known mainnet IC root public key
- `local` requires an explicit non-mainnet root key
- no protected verification path implicitly falls back to `cdk::api::root_key()`
- runtime root-key injection remains a bootstrap/config step, not a verifier
  shortcut

### 2. Verify Root Proof Chain

Search terms:

```text
RootProof::IcChainKeyBatchSignatureV1
verify_chain_key_batch_root_proof
ChainKeyRootVerifierPolicy
ChainKeyBatchHeaderV1
ChainKeyDelegationCertV1
RoleAttestationRootProof
RootPayloadKind::RoleAttestation
cert_hash
```

Confirm:

- delegation cert hash is canonical and recomputed by the verifier
- `cert.root_pid` matches the configured root canister id before proof acceptance
- root canister-signature public key DER names the expected root canister id
- root proof seed/domain match the payload kind
- root proof verification uses the configured raw IC root public key
- role attestation uses a distinct root payload seed/domain

### 3. Verify Issuer Proof Chain

Search terms:

```text
IssuerProof::IcCanisterSignatureV1
verify_issuer_canister_signature_proof
issuer_canister_sig_seed
issuer_canister_sig_seed_hash
issuer_canister_sig_domain
IssuerPayloadKind::DelegatedTokenClaims
issuer_proof_binding_hash
claims_hash
IssuerPidMismatch
```

Confirm:

- certificate issuer proof binding hash is recomputed and checked
- claims issuer matches the certified issuer
- claims cert hash matches the canonical certificate hash
- issuer canister-signature public key DER names the expected issuer canister id
- issuer proof seed/domain match delegated-token claims
- issuer proof verifies over the canonical claims hash with the configured raw
  IC root public key

### 4. Verify Token Acceptance Ordering

Search terms:

```text
AuthOps::verify_token
verify_delegated_token
verify_delegated_token_cached_proof_identity
positive_cache_get
delegated_token_verified
enforce_subject_binding
enforce_required_scope
```

Confirm:

- delegated-token verifier config and verifier-canister gate run before proof
  acceptance
- positive cache keys bind proof hash, claims hash, issuer proof hash, and caller
- positive cache hits still rerun canonical token checks, local audience/grant
  checks, and required-scope checks
- endpoint access guard verifies token material before subject binding and scope
  authorization

### 5. Verify Active Proof Install Chain

Search terms:

```text
install_active_delegation_proof
ActiveDelegationProof
InstallActiveDelegationProofInput
InvalidRootAuthority
IssuerMismatch
```

Confirm:

- signer/issuer active proof install verifies the root proof before storing
  active proof state
- installed delegation certificate names the local issuer canister
- installed proof cert hash is canonical and stored with active proof state
- wrong-root and wrong-issuer proofs fail closed

### 6. Verify Negative Cases

Confirm rejection for:

- invalid root proof
- invalid issuer proof
- root canister id mismatch
- missing or wrong root key for the configured build network
- cert hash drift
- issuer pid mismatch
- issuer proof binding hash drift
- noncanonical cert or claims data
- missing proof bytes outside explicit positive-cache identity checks
- role-attestation subject, audience, epoch, expiry, and proof failures

### 7. Test Expectations

Focused evidence should include:

- self-contained delegated token with valid root and issuer proof => acceptance
- invalid root proof => rejection
- invalid issuer proof => rejection
- cert hash drift => rejection
- noncanonical cert/claims vectors => rejection
- configured root-key/network validation
- root and issuer canister-signature seed/domain/payload message checks
- active proof install rejects wrong issuer/root proof failure
- endpoint guard ordering check
- local root batch provisioning installs active proof and verifies signer-local
  delegated token
- role-attestation proof and claim rejection paths

Current suggested commands:

```bash
cargo test --locked -p canic-core --lib verify_delegated_token -- --nocapture
cargo test --locked -p canic-core --lib auth_proof_verifier_config -- --nocapture
cargo test --locked -p canic-core --lib root_canister_sig -- --nocapture
cargo test --locked -p canic-core --lib issuer_canister_sig -- --nocapture
cargo test --locked -p canic-core --lib cert_rules -- --nocapture
cargo test --locked -p canic-core --lib install_active_delegation_proof -- --nocapture
cargo test --locked -p canic-core --lib delegated_auth_guard_preserves_verify_bind_scope_order -- --nocapture
cargo test --locked -p canic-tests --test root_suite delegated_auth_chain_key_batch_renews_without_external_liveness -- --nocapture
cargo test --locked -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture
```

## Structural Hotspots

List concrete files/modules/structs that carry trust-chain validation risk.

Detection commands (run and record output references):

```bash
rg -n 'AuthProofVerifierConfig|auth_proof_verifier_config|validate_build_network_root_key_pair|ic_root_public_key_raw|root_canister_id' crates/canic-core/src -g '*.rs'
rg -n 'verify_chain_key_batch_root_proof|ChainKeyRootVerifierPolicy|RootProof::IcChainKeyBatchSignatureV1|ChainKeyBatchHeaderV1|ChainKeyDelegationCertV1' crates/canic-core/src -g '*.rs'
rg -n 'RoleAttestationRootProof|verify_root_canister_signature_proof|root_canister_sig_seed|root_canister_sig_domain|RootPayloadKind::RoleAttestation' crates/canic-core/src -g '*.rs'
rg -n 'verify_issuer_canister_signature_proof|issuer_canister_sig_seed|issuer_canister_sig_seed_hash|IssuerPayloadKind|IssuerProof::IcCanisterSignatureV1' crates/canic-core/src -g '*.rs'
rg -n 'cert_hash|claims_hash|issuer_proof_binding_hash|VerifyDelegatedTokenError|IssuerPidMismatch|CertHashMismatch' crates/canic-core/src/ops/auth -g '*.rs'
git log --name-only -n 20 -- crates/canic-core/src/ops/auth crates/canic-core/src/api/auth crates/canic-core/src/config/validation/auth.rs crates/canic-core/src/domain/auth.rs crates/canic-tests/tests/root_cases crates/canic-tests/tests/pic_role_attestation_cases
```

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/token.rs` | `AuthOps::verify_token`, `auth_proof_verifier_config_from`, `verify_with_embedded_proofs` | runtime config, root key, root proof, and issuer proof orchestration | High |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_delegated_token`, `verify_claims` | pure cert/claims/root/issuer verification ordering | High |
| `crates/canic-core/src/ops/auth/delegated/chain_key.rs` | `verify_chain_key_batch_root_proof` | delegated root proof policy, chain-key public key, signature, Merkle witness, and certificate equality | High |
| `crates/canic-core/src/ops/auth/root_canister_sig.rs` | `verify_root_canister_signature_proof` | role-attestation root proof canister id, seed/domain, message, and IC root-key verification | Medium |
| `crates/canic-core/src/ops/auth/issuer_canister_sig.rs` | `verify_issuer_canister_signature_proof` | issuer proof canister id, seed/domain, message, and IC root-key verification | High |
| `crates/canic-core/src/ops/auth/delegated/cert_rules.rs` | `validate_cert_issuance_rules` | root pid, TTL, audience/grant shape, and issuer binding hash checks | High |
| `crates/canic-core/src/ops/auth/delegated/canonical.rs` | `cert_hash`, `claims_hash`, `issuer_proof_binding_hash` | canonical hash material for root and issuer signatures | High |
| `crates/canic-core/src/ops/auth/delegated/active_proof.rs` | `install_active_delegation_proof` | issuer-local active proof validation before storage | Medium |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | endpoint guard integration after trust-chain verification | Medium |
| `crates/canic-core/src/ops/auth/attestation.rs` | `verify_role_attestation_cached` | role-attestation root proof verification uses the same trust-anchor config | Medium |

If none are detected in a given run, state: No structural hotspots detected in this run.

## Hub Module Pressure

Detect modules trending toward gravity-well behavior from import fan-in,
cross-layer coupling, and edit frequency.

Treat DTO fan-in differently from verifier fan-in:

- broad passive DTO fan-in is a watchpoint unless behavior, storage mutation, or
  validation logic moves onto the DTO type
- verifier/guard fan-in is scored as structural pressure because it can affect
  acceptance order
- tests and support canisters count as evidence, but do not by themselves make
  a DTO a production hub

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `<module>` | `<top import tokens>` | `<n>` | `<n>` | `<1-10>` |

Pressure score guidance:

- 1-3 = low
- 4-6 = moderate
- 7-10 = high

## Red Flags

- root proof skipped or accepted after token acceptance
- issuer proof skipped or accepted after token acceptance
- token-provided root key material replaces configured verifier trust anchors
- protected verification path calls `cdk::api::root_key()` directly
- canister-signature public key DER is not checked for expected canister id
- root or issuer seed/domain mismatch is ignored
- claims accepted without matching issuer pid and cert hash
- issuer proof binding hash drift is accepted
- canonical cert/claims encoding accepts unsorted or malformed data
- positive cache bypasses local canonical, audience, grant, or scope checks
- passive DTOs gain behavior or validation methods

## Severity

Critical: untrusted issuers may mint accepted identities.

## Early Warning Signals

Detect predictive architecture-decay patterns before they appear as friction or
failures.

Detection scans (run and record output references):

```bash
rg -l 'AuthProofVerifierConfig|auth_proof_verifier_config|ic_root_public_key_raw|validate_build_network_root_key_pair' crates canisters fleets -g '*.rs' | wc -l
rg -l 'verify_chain_key_batch_root_proof|RootProof::IcChainKeyBatchSignatureV1|ChainKeyRootVerifierPolicy|ChainKeyBatchHeaderV1|ChainKeyDelegationCertV1' crates canisters fleets -g '*.rs' | wc -l
rg -l 'RoleAttestationRootProof|verify_root_canister_signature_proof|RootPayloadKind::RoleAttestation|root_canister_sig_' crates canisters fleets -g '*.rs' | wc -l
rg -l 'verify_issuer_canister_signature_proof|IssuerPayloadKind|IssuerProof::IcCanisterSignatureV1|issuer_canister_sig_' crates canisters fleets -g '*.rs' | wc -l
rg -l 'cert_hash|claims_hash|issuer_proof_binding_hash|DelegationCert|DelegatedTokenClaims' crates canisters fleets -g '*.rs' | wc -l
rg -l 'DelegatedToken|DelegationProof|RootProof|IssuerProof|ActiveDelegationProof|SignedRoleAttestation' crates canisters fleets -g '*.rs' | wc -l
```

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| `<signal>` | `<path or module>` | `<scan evidence>` | `<Low/Medium/High>` |
| `dependency fan-in hub` | `<module path>` | `imported by <n> files across <subsystems>` | `<Low/Medium/High>` |

### Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `<EnumName>` | `<path>` | `<count>` | `<Low/Medium/High>` |

Thresholds:

- `0-5` references = normal
- `6-10` = coupling forming
- `10+` = architectural shock radius

### Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `<StructName>` | `<path>` | `<api/workflow/ops/policy>` | `<Low/Medium/High>` |

### Growing Hub Modules

| Module | Subsystems Imported | Recent Commits | Risk |
| --- | --- | ---: | --- |
| `<path>` | `<subsystems>` | `<count>` | `<Low/Medium/High>` |

### Capability Surface Growth

| Module | Public Items | Risk |
| --- | ---: | --- |
| `<path>` | `<count pub fn + pub struct>` | `<Low/Medium/High>` |

Thresholds:

- `0-10` = normal
- `10-20` = growing surface
- `20+` = risk

If no predictive signals are detected, state: No predictive architectural signals detected in this run.

## Dependency Fan-In Pressure

Detect modules and structs becoming architectural gravity wells before friction
increases.

Detection scans (run and record output references):

```bash
rg "use crate::" crates/ -g "*.rs"
rg "pub struct" crates/ -g "*.rs"
rg "<StructName>" crates/ -g "*.rs"
```

### Module Fan-In

Count how many files import each module; flag modules imported by `6+` files.

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `<module path>` | `<count>` | `<api/workflow/ops/policy/...>` | `<Low/Medium/High>` |

Pressure level rules:

- `0-3` imports = normal
- `4-6` imports = rising pressure
- `7-10` imports = hub forming
- `10+` imports = architectural gravity well

### Struct Fan-In

Count references for public structs; flag structs referenced in `6+` files.

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `<StructName>` | `<path>` | `<count>` | `<Low/Medium/High>` |

Interpretation:

- `6-8` references = coupling forming
- `9-12` = hub abstraction
- `12+` = system dependency center

If no modules exceed the fan-in threshold, state: No fan-in pressure detected in this run.

## Risk Score

Risk Score: **X / 10**

Interpretation scale:

- 0-2 = negligible risk
- 3-4 = low risk
- 5-6 = moderate risk
- 7-8 = high risk
- 9-10 = critical architectural risk

Score must be justified using checklist findings and Structural Hotspots
evidence. Separate the security verdict from structural watchpoints: a PASS can
still have structural pressure, but passive DTO fan-in alone must not dominate
the score.

Derivation guidance:

- start at `0`
- add `+4` for any confirmed trust-chain validation break
- add `+3` if a required trust-chain stage is present but ordered after token
  acceptance or endpoint execution
- add `+2` if verifier-local trust-anchor evidence is missing or ambiguous
- add `+2` if required unit verifier/root-key tests are not run or are blocked
- add `+2` if the local root batch provisioning or role-attestation PocketIC
  path is not run or is blocked
- add `+1` per medium/high verifier or endpoint-guard hotspot contribution
  (max `+2`)
- add `+1` if any verifier/guard hub module pressure score is `>= 7`
- add `+1` if enum shock radius is detected (`> 6` reference files)
- add `+1` if active verifier/guard structs spread across `>= 3` architecture
  layers
- add `+1` if growing verifier/guard hub module signal is detected
- add `+1` if capability public surface is `> 20` items
- add `+1` for passive DTO fan-in `12+` across multiple production subsystems
  only if the DTO remains behavior-free; score higher under red flags if it
  gains behavior
- clamp to `0..10`

If no confirmed findings and no hotspot/hub signals are present, score must remain `0-2`.

## Verification Readout

Use command outcomes with normalized statuses:

- `PASS`
- `FAIL`
- `BLOCKED`

## Follow-up Actions

If result is `FAIL`/`PARTIAL` or risk score is `>= 5`, include owner, action,
and target report run.

If no action is needed, state: `No follow-up actions required.`

## Reporting Template

- Scope:
- Commit:
- Trust anchors reviewed:
- Verification entrypoint:
- Result: `PASS` | `FAIL` | `PARTIAL`
- Chain evidence:
- Structural Hotspots:
- Hub Module Pressure:
- Early Warning Signals:
- Dependency Fan-In Pressure:
- Risk Score:
- Verification Readout:
- Follow-up actions:
