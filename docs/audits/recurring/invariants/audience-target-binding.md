# Audit: Audience Target Binding Invariant

## Purpose

Ensure tokens and grants are accepted only in their intended audience/target execution context.

## Risk Model / Invariant

A token containing an audience or target binding claim must be rejected unless that claim matches the current runtime execution context.

Canonical form:

`token.aud` (or equivalent audience/target field) must include the current runtime target.

Audience/target binding must be enforced by the canonical verifier or by a verifier stage executed before authorization.

Runtime context may include:

- current canister principal
- service identifier
- RPC target
- execution environment identifier

### Invariant

Delegated credentials must be bound to the target canister via the audience
(`aud`) claim and verified before any authorization or capability evaluation
occurs.

## Why This Matters

Without audience/target binding, a valid token may be replayed across services or canisters where it was never intended to authorize.

### Failure Modes

| Failure | Impact |
| --- | --- |
| audience not checked | cross-canister token replay |
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
- service routing changes
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

Confirm verifier logic compares audience/target claims against the current runtime execution context before authorization or business logic.

Examples:

- delegated grant audience includes target canister
- service scope matches expected service
- issuer/target relationship is validated where required

### 3. Verify Failure Semantics

Confirm mismatched audience/target causes authentication failure before authorization checks or handler execution.

### 4. Verify Replay Surface

Confirm freshness controls are enforced for delegated credentials:

- expiry checks are mandatory in verifier path
- nonce/request-id replay controls are validated where the model requires them
- audience binding is not treated as a substitute for freshness enforcement

Cross-reference result against the Expiry / Replay / Single-Use Invariant.

### 5. Test Expectations

- valid token for audience A used in audience B => rejection
- valid token for wrong target canister => rejection
- valid token for correct audience/target => success

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
| `ops/auth/delegated/audience.rs` | `validate_audience_shape`, `audience_subset`, `verifier_is_in_audience` | delegated-token audience shape, subset, and verifier membership checks | High |
| `ops/auth/delegated/verify.rs` | `verify_audience` | delegated-token cert/claim audience and local verifier checks | High |
| `ops/auth/verify/attestation.rs` | `verify_role_attestation_claims` | role-attestation subject, timing, audience, subnet, and epoch checks | High |
| `api/rpc/capability/proof.rs` | `verify_capability_hash_binding` | root capability hash binding to target canister and canonical payload | High |
| `api/rpc/capability/verifier.rs` | `verify_root_capability_proof` | proof-mode routing and target-binding verification before proof-specific checks | High |
| `api/rpc/capability/grant.rs` | delegated grant claim verifier | target/issuer/subject binding enforcement | High |
| `ops/rpc/mod.rs` | outbound root-attestation request/cache helpers | request-time target audience selection and cache audience matching | Medium |
| `dto/auth.rs`, `dto/capability/proof.rs` | delegated claim structs | audience and target field definitions | Medium |

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
- audience claim compared against caller rather than runtime service/target context

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
cargo test -p canic-core --lib verify_root_delegated_grant_claims_rejects_audience_mismatch -- --nocapture
cargo test -p canic-core --lib verify_delegated_token_rejects_audience_subset_drift -- --nocapture
cargo test -p canic-core --lib verify_delegated_token_rejects_missing_local_role_for_role_audience -- --nocapture
cargo test -p canic-core --lib mint_delegated_token_rejects_audience_expansion -- --nocapture
cargo test -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture
cargo test -p canic-tests --test pic_role_attestation capability_endpoint_role_attestation_proof_paths -- --test-threads=1 --nocapture
```

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
