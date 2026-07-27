# Audit: Audience Target Binding Invariant

## Method Contract

- Audit ID: `CANIC-AUTH-AUDIENCE-001`
- Method version: `3`
- Disposition: `revise`
- Owner: signed audience, runtime target, and local-role grant binding
- Kind/profile: security `invariant`
- Trace mode: `code_trace`; focused rejection execution may use PocketIC
- Cost/runtime: medium; 30-60 minutes excluding PocketIC fixtures
- Prerequisites: Git, ripgrep, current auth DTO/ops/access code, generated
  Candid, and focused audience rejection tests
- False-positive boundary: transport routing metadata is not a signed audience
  unless it participates in the authorization decision
- Shared contract: [AUDIT-HOWTO.md](../../AUDIT-HOWTO.md)

## Purpose

Ensure tokens and grants are accepted only in their intended audience/target execution context.

## Risk Model / Invariant

A token containing an audience or target binding claim must be rejected unless
that claim matches the immutable protected Fleet identity.

Canonical form:

`token.aud` must equal the current `FleetActivation` binding.

Audience/target binding must be enforced by the canonical verifier or by a verifier stage executed before authorization.

The canonical delegated-token runtime context contains:

- canonical network identity
- generated Fleet ID
- local canister role for grant selection

### Invariant

Delegated credentials must be bound to the exact network-qualified Fleet via
the audience (`aud`) claim and verified before any authorization or capability
evaluation occurs.

## Why This Matters

Without audience/target binding, a valid token may be replayed across services or canisters where it was never intended to authorize.

### Failure Modes

| Failure | Impact |
| --- | --- |
| audience not checked | cross-Fleet token replay |
| issuer not validated | forged delegation acceptance |
| subject not bound | privilege escalation |
| verification occurs after policy | confused-deputy risk |

### Verification Order

Order must be enforced before handler execution:

1. signature verification
2. expiration check
3. audience binding
4. issuer binding
5. subject binding
6. capability evaluation

## Run This Audit After

- token/grant claim schema changes
- Fleet activation identity changes
- capability scope model changes
- verifier pipeline refactors

## Report Preamble (Required)

Every report generated from this audit must include:

- Scope
- Compared baseline report path
- Code snapshot identifier
- Method tag/version
- Comparability status

## Audit Checklist

### 1. Locate Audience/Target Claims

Search terms:

```text
aud
audience
target_canister
service
issuer
```

Confirm:

- auth DTOs expose explicit audience/target fields where applicable
- verifier paths consume those fields during acceptance
- audience/target claims are classified as mandatory or optional and enforcement follows that contract

### 2. Verify Runtime Context Binding

Confirm verifier logic compares audience/target claims against the protected
Fleet activation identity before authorization or business logic.

Examples:

- delegated-token audience equals the protected Fleet
- root issuer policies and renewal templates name only the protected Fleet
- issuer/target relationship is validated where required
- active root delegation proof install binds the proof certificate issuer to
  the current signer canister before storing active proof state
- active root delegation proof install verifies the proof against the
  configured trusted root canister/root key before storing active proof state

### 3. Verify Failure Semantics

Confirm mismatched audience/target causes authentication failure before authorization checks or handler execution.

### 4. Verify Replay Surface

Confirm freshness controls are enforced for delegated credentials:

- expiry checks are mandatory in verifier path
- nonce/request-id replay controls are validated where the model requires them
- audience binding is not treated as a substitute for freshness enforcement

Cross-reference result against the Expiry / Replay / Single-Use Invariant.

### 5. Test Expectations

- valid token for Fleet A used in Fleet B => rejection
- issuer policy or renewal template for another Fleet => rejection before
  mutation
- valid token for correct audience/target => success
- active delegation proof for issuer A installed on issuer B => rejection
- active delegation proof signed by unexpected root => rejection

## Structural Hotspots

List concrete files/modules/structs that carry audience-target binding risk.

Detection commands (run and record output references):

```bash
rg '^use ' crates/ -g '*.rs'
rg 'crate::workflow|crate::ops|crate::api|crate::policy' crates/ -g '*.rs'
rg 'pub struct|impl ' crates/ -g '*.rs'
git log --name-only -n 20 -- crates/
```

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `ops/auth/delegated/audience.rs` | `audience_subset`, `audience_accepted`, `role_grants_subset`, `scopes_for_role` | delegated-token Fleet subset, protected-Fleet matching, grant subset, and local-role grant lookup | High |
| `ops/auth/delegated/verify.rs` | `verify_audience_and_grants` | delegated-token cert/claim audience, local verifier, grant subset, and scope checks | High |
| `ops/auth/delegated/prepare.rs` | `prepare_delegated_token` | rejects delegated-token audience expansion against the active issuer certificate | High |
| `ops/auth/delegated/active_proof.rs` | `install_active_delegation_proof` | binds active proof cert issuer to the local signer canister and verifies trusted root proof before storage | High |
| `ops/auth/delegation/active.rs` | `install_active_delegation_proof` | supplies current canister/root verifier context before persisting active proof | High |
| `domain/policy/pure/auth/root_provisioning.rs` | Fleet-binding and root proof policy validators | root issuer admission enforces the protected Fleet, issuer, enabled state, allowed grants, TTL, and refresh policy before mutation or proof preparation | High |
| `workflow/runtime/auth/root_issuer/mod.rs` | issuer policy and renewal-template upsert | resolves the protected Fleet and applies pure admission before mutation | High |
| `ops/auth/delegation/chain_key_batch.rs` | `build_chain_key_batch_leaf`, `prepare_due_chain_key_root_delegation_batch` | maps root renewal templates into root issuer policy decisions before preparing canonical chain-key batch leaves | High |
| `ops/auth/delegated/chain_key.rs` | `verify_cert_leaf_binding` | rejects root proofs whose signed leaf audience/grants/issuer binding do not exactly match the embedded delegation cert | High |
| `ops/auth/verify/attestation.rs` | `verify_role_attestation_claims` | role-attestation subject, timing, audience, subnet, and epoch checks | High |
| `ops/rpc/capability.rs` | `root_capability_hash` | canonical root capability hash binding to target canister, capability version, service, and canonical payload | High |
| `workflow/rpc/capability/proof.rs` | `verify_capability_hash_binding`, structural proof helpers | test-visible target hash verification and runtime structural proof checks | Medium |
| `workflow/rpc/capability/verifier.rs` | `verify_root_capability_proof` | active proof-mode routing; current runtime accepts structural proof mode only | Medium |
| `workflow/rpc/capability/root.rs` | `response_capability_v1_root` | validates envelopes, verifies structural proof mode, then dispatches root capability requests | Medium |
| `dto/auth`, `dto/capability/proof.rs` | delegated claim structs | Fleet audience and capability target field definitions | Medium |

If none are detected in a given run, state: No structural hotspots detected in this run.

## Hub Module Pressure

Detect modules trending toward gravity-well behavior from import fan-in, cross-layer coupling, and edit frequency.

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `<module>` | `<top import tokens>` | `<n>` | `<n>` | `<1-10>` |

Pressure score guidance:

- 1-3 = low
- 4-6 = moderate
- 7-10 = high

## Red Flags

- audience field present but not enforced in verifier
- service/target checks applied only in non-canonical path
- fallback path accepts token without audience/target comparison
- audience claim compared against caller or ambient canister/subnet metadata
  rather than the protected Fleet binding

## Severity

High to Critical depending on cross-service replay impact.

## Early Warning Signals

Detect predictive architecture-decay patterns before they appear as friction or failures.

Detection scans (run and record output references):

```bash
rg 'enum ' crates/ -g '*.rs'
rg 'pub struct|pub fn' crates/ -g '*.rs'
rg '^use ' crates/ -g '*.rs'
git log --name-only -n 20 -- crates/
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

## Recommended Verification Commands

Use current targeted tests rather than historical test names from older reports.

```bash
bash docs/audits/scripts/run-nonempty-cargo-test.sh --locked -p canic-core --lib audience -- --nocapture
bash docs/audits/scripts/run-nonempty-cargo-test.sh --locked -p canic-core --lib local_role -- --nocapture
bash docs/audits/scripts/run-nonempty-cargo-test.sh --locked -p canic-core --lib prepare_delegated_token_rejects_audience_expansion -- --nocapture
bash docs/audits/scripts/run-nonempty-cargo-test.sh --locked -p canic-core --lib install_active_delegation_proof_rejects_wrong_issuer -- --nocapture
bash docs/audits/scripts/run-nonempty-cargo-test.sh --locked -p canic-core --lib install_active_delegation_proof_rejects_root_proof_failure -- --nocapture
bash docs/audits/scripts/run-nonempty-cargo-test.sh --locked -p canic-core --lib root_prepare_policy_rejects_audience_or_grant_outside_policy -- --nocapture
bash docs/audits/scripts/run-nonempty-cargo-test.sh --locked -p canic-core --lib chain_key_batch_root_proof_rejects_wrong_audience -- --nocapture
bash docs/audits/scripts/run-nonempty-cargo-test.sh --locked -p canic-core --lib chain_key_batch_root_proof_rejects_wrong_grants -- --nocapture
bash docs/audits/scripts/run-nonempty-cargo-test.sh --locked -p canic-core --lib role_attestation_claims_reject -- --nocapture
bash docs/audits/scripts/run-nonempty-cargo-test.sh --locked -p canic-core --lib root_capability_hash_binds_target_canister -- --nocapture
bash docs/audits/scripts/run-nonempty-cargo-test.sh --locked -p canic-core --lib verify_capability_hash_binding -- --nocapture
bash docs/audits/scripts/run-nonempty-cargo-test.sh --locked -p canic-core --lib role_attestation_claims -- --nocapture
bash docs/audits/scripts/run-nonempty-cargo-test.sh --locked -p canic-core --lib role_attestation_verifier -- --nocapture
```

The wrapper is part of the method identity. A successful Cargo exit with zero
executed tests is `BLOCKED`, never passing evidence.
Registry-bound role-attestation PocketIC coverage remains required once the
root Component allocation lifecycle can create the issuer; the removed static
root/issuer fixture is not valid evidence.

## Dependency Fan-In Pressure

Detect modules and structs becoming architectural gravity wells before friction increases.

Detection scans (run and record output references):

```bash
rg "use crate::" crates/ -g "*.rs"
rg "pub struct" crates/ -g "*.rs"
# then: rg "<StructName>" crates/ -g "*.rs"
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

Score must be justified using checklist findings and Structural Hotspots evidence.

Derivation guidance (deterministic):

- start at `0`
- add `+4` for any confirmed audience-target binding break
- add `+2` per medium/high hotspot contribution (max `+4`)
- add `+2` if any hub module pressure score is `>= 7`
- add `+1` if enum shock radius is detected (`> 6` reference files)
- add `+1` if cross-layer struct spread is detected (`>= 3` architecture layers)
- add `+2` if growing hub module signal is detected
- add `+1` if capability public surface is `> 20` items
- add `+1` for fan-in `6-8` across multiple subsystems
- add `+2` for fan-in `9-12` across multiple subsystems
- add `+3` for fan-in `12+` across multiple subsystems
- clamp to `0..10`

If no confirmed findings and no hotspot/hub signals are present, score must remain `0-2`.

## Verification Readout

Use command outcomes with normalized statuses:

- `PASS`
- `FAIL`
- `BLOCKED`

## Follow-up Actions

If result is `FAIL`/`PARTIAL` or risk score is `>= 5`, include owner, action, and target report run.

If no action is needed, state: `No follow-up actions required.`

## Reporting Template

- Scope:
- Commit:
- Runtime context used for comparison:
- Result: `PASS` | `FAIL` | `PARTIAL`
- Audience/target evidence:
- Structural Hotspots:
- Hub Module Pressure:
- Early Warning Signals:
- Dependency Fan-In Pressure:
- Risk Score:
- Verification Readout:
- Follow-up actions:
