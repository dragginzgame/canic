# Audit: Token Trust Chain Invariant

## Purpose

Ensure token acceptance requires a valid issuer trust chain from root authority to token issuer.

## Risk Model / Invariant

A token must be rejected unless the complete issuer trust chain validates from root authority to shard authority to token issuer.

Chain validation must verify:

- root authority identity
- shard authority certification by root
- token issuer certification by shard
- token signature under issuer key

Freshness checks are verified by the Expiry / Replay / Single-Use Invariant.

## Why This Matters

Subject binding alone is insufficient if untrusted issuers can mint tokens that pass verification.

## Run This Audit After

- root/shard trust model changes
- issuer/certificate format changes
- token verification pipeline updates
- trust cache or key refresh changes

## Report Preamble (Required)

Every report generated from this audit must include:

- Scope
- Compared baseline report path
- Code snapshot identifier
- Method tag/version
- Comparability status

## Audit Checklist

### 1. Identify Trust Anchors

Locate root and shard verification code and confirm trust anchors are explicitly defined and not derived from untrusted input or dynamic request context.

### 2. Verify Chain Validation

Search terms:

```text
verify_root
verify_shard
issuer
signature
certificate
```

Confirm:

- each trust layer is verified before token acceptance
- cached trust objects retain cryptographic integrity and do not bypass required verification steps
- issuer identity is bound to expected authority

### 3. Verify Negative Cases

Confirm rejection for:

- invalid root
- invalid shard
- invalid token signature
- unexpected issuer relationship
- expired or revoked issuer certificate

### 4. Test Expectations

- invalid shard under valid root => rejection
- valid shard under invalid root => rejection
- forged token under valid chain context => rejection

## Structural Hotspots

List concrete files/modules/structs that carry trust-chain validation risk.

Detection commands (run and record output references):

```bash
rg '^use ' crates/ -g '*.rs'
rg 'crate::workflow|crate::ops|crate::api|crate::policy' crates/ -g '*.rs'
rg 'pub struct|impl ' crates/ -g '*.rs'
git log --name-only -n 20 -- crates/
```

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `ops/auth/token.rs` | `verify_token`, structure checks | trust-chain orchestration entrypoint | High |
| `ops/auth/verify.rs` | `verify_delegation_signature`, `verify_token_sig` | root/shard cryptographic verification | High |
| `ops/storage/auth/*` | delegation key/proof state access | trust-anchor material sourcing | Medium |

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

- trust chain step skipped on an internal path
- token signature accepted without issuer-root linkage
- cache hit bypasses mandatory integrity checks
- issuer key accepted without verifying issuer authority
- trust anchor loaded from runtime configuration without validation

## Severity

Critical: untrusted issuers may mint accepted identities.

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
- add `+4` for any confirmed trust-chain validation break
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
