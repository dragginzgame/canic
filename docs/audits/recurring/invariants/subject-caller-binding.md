# Audit: Subject-Caller Binding Invariant

## Method Contract

- Audit ID: `CANIC-AUTH-SUBJECT-001`
- Method version: `1`
- Disposition: `revise`
- Owner: verified subject and transport caller binding
- Kind/profile: security `invariant`
- Trace mode: `code_trace`; focused wrong-caller execution may use PocketIC
- Cost/runtime: medium; 30-60 minutes
- Prerequisites: Git, ripgrep, caller/subject resolution code, macro paths, and
  wrong-caller rejection fixtures
- False-positive boundary: issuer, controller, parent, and delegated subject
  identities remain distinct unless the active contract explicitly equates
  them
- Shared contract: [AUDIT-HOWTO.md](../../AUDIT-HOWTO.md)

## Purpose

Ensure delegated tokens cannot become transferable bearer credentials.

## Risk Model / Invariant

A delegated token is accepted for endpoint auth only when the verified token
subject is the authenticated endpoint subject selected for the current call.

```text
verify token material -> bind verified subject -> authorize scope -> execute handler
```

For raw caller auth, the authenticated subject is `msg_caller()`. For delegated
sessions, the authenticated subject is the validated delegated session subject,
while topology/caller predicates still use the raw transport caller.

## Why This Matters

If subject binding is skipped or applied to the wrong identity lane, a stolen
delegated token can impersonate another principal or a wallet session can blur
transport and delegated-subject authority.

## Required Ordering

Subject binding must occur after cryptographic/material token verification and
before scope authorization, replay acceptance, endpoint dispatch, business
logic, or state mutation.

## Run This Audit After

- delegated-token verifier changes
- delegated-session identity resolution changes
- endpoint macro / access-expression changes
- `AccessContext` shape changes
- token DTO or canonical claims changes
- root proof provisioning or active proof install/status changes
- auth API helper visibility changes
- runtime changes affecting caller identity

## Report Preamble (Required)

Every report generated from this audit must include:

- Scope
- Compared baseline report path
- Code snapshot identifier
- Method tag/version
- Comparability status

## Audit Checklist

### 1. Canonical Enforcement

Search terms:

```text
claims.subject
DelegatedTokenClaims
VerifyDelegatedTokenRuntimeInput
delegated_token_verified
enforce_subject_binding
Authenticated { required_scope
```

Confirm:

- one canonical endpoint-auth enforcement path exists
- `VerifiedDelegatedToken.subject` is compared to the authenticated subject
- subject binding occurs before required-scope acceptance
- subject binding occurs before handler execution
- no alternate delegated-token endpoint path skips the check
- lower-level proof/material verifiers do not claim to be full endpoint auth

### 2. Identity Lane Separation

Search terms:

```text
AccessContext
transport_caller
authenticated_caller
authenticated_subject
resolve_authenticated_identity
AuthenticatedIdentitySource
```

Confirm:

- endpoint macros build `AccessContext` from resolved authenticated identity
- caller/topology predicates use raw transport caller
- authenticated predicates use the resolved authenticated subject
- delegated sessions cannot turn infrastructure/canister principals into user subjects
- delegated-session bootstrap checks the requested subject against verified token material

### 3. No Bearer Fallback Paths

Search terms:

```text
verify_token_material
AuthOps::verify_token
verify_delegated_token_cached_proof_identity
presenter_pid
relay
envelope
```

Confirm:

- no public helper verifies token material without caller/subject binding
- no production endpoint accepts delegated-token proof as bearer-only material
- positive-cache paths still rerun endpoint-local claims/scope checks
- private token-material helpers remain explicitly incomplete for endpoint auth

### 4. Provisioning Principal Separation

Search terms:

```text
issuer_pid
installed_by
ActiveDelegationProof
install_active_delegation_proof
RootDelegationProofBatch
```

Confirm:

- root-proof provisioning identities are not treated as delegated-token subjects
- `issuer_pid` binds a delegation certificate to the issuer canister, not to the
  end-user caller
- `installed_by` records the provisioning caller and is not an endpoint-auth subject
- active proof installation does not bypass normal signer-local delegated-token
  prepare/get verification or endpoint subject binding

### 5. Macro / DSL Preservation

For abstractions such as `authenticated(...)`, `requires_scope(...)`, and
generated endpoint wrappers, confirm macro expansion routes through
`AccessContext` and the canonical access verifier rather than duplicating
partial auth logic.

### 6. Test Expectations

Focused tests must prove:

```text
token for subject A + authenticated subject B => rejection
token for subject A + authenticated subject A => success
caller/topology predicates use raw transport caller
authenticated predicates use resolved delegated-session subject
delegated-session bootstrap rejects subject mismatch
active proof install rejects proofs for a different issuer canister
```

## Structural Hotspots

List concrete files/modules/structs that carry subject-binding risk.

Detection commands (run and record output references):

```bash
rg -n 'enforce_subject_binding|delegated_token_verified|VerifyDelegatedTokenRuntimeInput' crates/canic-core/src crates/canic-macros/src -g '*.rs'
rg -n 'AccessContext|authenticated_caller|authenticated_subject|transport_caller|resolve_authenticated_identity' crates/canic-core/src crates/canic-macros/src -g '*.rs'
rg -n 'verify_token_material|AuthOps::verify_token|verify_delegated_token_cached_proof_identity' crates/canic-core/src -g '*.rs'
rg -n 'issuer_pid|installed_by|ActiveDelegationProof|install_active_delegation_proof' crates/canic-core/src crates/canic/tests -g '*.rs'
git log --name-only -n 20 -- crates/canic-core/src/access crates/canic-core/src/api/auth crates/canic-core/src/ops/auth crates/canic-macros/src/endpoint/expand
```

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/access/auth/token.rs` | `verify_token`, `enforce_subject_binding` | canonical endpoint delegated-token subject binding | High |
| `crates/canic-core/src/access/expr/evaluators.rs` | authenticated predicate branch | passes authenticated subject into canonical verifier | High |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext` | stores raw transport and authenticated subject lanes | High |
| `crates/canic-macros/src/endpoint/expand/access.rs` | access stage expansion | generated endpoint wrappers construct access context | High |
| `crates/canic-core/src/access/auth/identity.rs` | `resolve_authenticated_identity_at` | delegated-session subject selection and fallback | Medium |
| `crates/canic-core/src/api/auth/session/mod.rs` | `set_delegated_session_subject` | bootstrap checks requested subject against verified token subject | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | `verify_token_material` | private partial verifier that must not become endpoint auth | Medium |
| `crates/canic-core/src/ops/auth/delegated/active_proof.rs` | `install_active_delegation_proof` | issuer-canister binding for installed root proof material | Medium |

If none are detected in a given run, state: No structural hotspots detected in this run.

## Hub Module Pressure

Detect modules trending toward gravity-well behavior from import fan-in,
cross-layer coupling, and edit frequency.

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `<module>` | `<top import tokens>` | `<n>` | `<n>` | `<1-10>` |

Pressure score guidance:

- 1-3 = low
- 4-6 = moderate
- 7-10 = high

## Red Flags

- signature verification succeeds without subject comparison
- endpoint authorization uses token subject but skips caller/authenticated-subject binding
- caller/topology predicates use delegated-session subject
- handler-local auth checks replace canonical binding
- public helpers expose token-material verification as complete endpoint auth
- root proof provisioning fields are reused as end-user auth subjects
- comments imply caller identity is trusted by convention

## Severity

Critical: enables cross-principal impersonation.

## Early Warning Signals

Detect predictive architecture-decay patterns before they appear as friction or
failures.

Detection scans (run and record output references):

```bash
rg -l 'access::expr|eval_access|AccessContext|BuiltinPredicate::Authenticated|authenticated_caller|transport_caller|authenticated_subject' crates canisters fleets -g '*.rs' | wc -l
rg -l 'access::auth|delegated_token_verified|resolve_authenticated_identity|enforce_subject_binding|AuthenticatedIdentitySource|ResolvedAuthenticatedIdentity' crates canisters fleets -g '*.rs' | wc -l
rg -l 'VerifyDelegatedTokenRuntimeInput|VerifiedDelegatedToken|verify_delegated_token|DelegatedTokenClaims|claims\.subject' crates canisters fleets -g '*.rs' | wc -l
rg -l 'subject mismatch|subject.*caller|caller.*subject|delegated token subject|subject.*must match caller|does not match caller' crates/canic-core/src crates/canic-tests/tests canisters/test fleets/test -g '*.rs' | wc -l
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

Score must be justified using checklist findings and Structural Hotspots evidence.

Derivation guidance:

- start at `0`
- add `+4` for any confirmed invariant break
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

If result is `FAIL`/`PARTIAL` or risk score is `>= 5`, include owner, action,
and target report run.

If no action is needed, state: `No follow-up actions required.`

## Reporting Template

- Scope:
- Commit:
- Canonical verifier reference:
- Enforcement location:
- Result: `PASS` | `FAIL` | `PARTIAL`
- Evidence:
- Structural Hotspots:
- Hub Module Pressure:
- Early Warning Signals:
- Dependency Fan-In Pressure:
- Risk Score:
- Verification Readout:
- Follow-up actions:
