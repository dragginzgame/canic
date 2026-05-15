# Audit: Auth Abstraction Equivalence Invariant

## Purpose

Ensure macro-generated, DSL-generated, and helper-generated authentication paths preserve the security semantics of the canonical verifier.

## Risk Model / Invariant

All authentication abstractions must route through the canonical verifier and preserve identical authentication and authorization semantics.

Generated or helper-based endpoints must not weaken, bypass, or reorder verification logic relative to the canonical handwritten authentication path.

### Invariant

Authentication abstractions must not change verification semantics.
Macro-generated endpoints must enforce the same subject, scope, and claim
checks as the canonical verifier.

Required properties:

- canonical verifier usage
- trust-chain checks
- subject binding
- authorization ordering
- failure semantics

## Why This Matters

Security regressions frequently enter through convenience abstractions rather than verifier core code.

## Required Equivalence

For any authenticated endpoint implemented via abstraction, the abstraction path and the handwritten canonical path must exhibit identical behavior for:

- valid credentials
- invalid signatures
- mismatched subject/caller
- expired credentials
- insufficient scope
- update replay
- query statelessness
- delegated-session subject resolution

## Relationship to Canonical Auth Boundary

The Canonical Auth Boundary Invariant verifies that all authenticated paths converge on the canonical verifier.

This audit verifies semantic equivalence and failure-behavior parity between abstraction-generated and handwritten canonical authentication paths.

This audit must also check the mechanical trust-chain guards that prevent a
public material-only verifier from reappearing. The abstraction path is only
equivalent if endpoint-level subject binding and update-token replay
consumption remain part of the generated/helper boundary.

## Run This Audit After

- macro / DSL changes
- endpoint helper additions
- dispatcher wrapper changes
- auth abstraction refactors
- `AccessContext` field or identity-lane changes
- delegated-session bootstrap or resolution changes
- delegated-token replay, scope, or verifier-ordering changes

## Report Preamble (Required)

Every report generated from this audit must include:

- Scope
- Compared baseline report path
- Code snapshot identifier
- Method tag/version
- Comparability status

## Audit Checklist

### 1. Identify Auth Abstractions

Search terms:

```text
canic_update
canic_query
requires(auth::authenticated
auth::authenticated(
resolve_authenticated_identity
delegated_token_verified
AccessContext
```

### 2. Inspect Expansion or Equivalent Implementation

Inspect macro expansion (`cargo expand`) or equivalent code-generation output to verify abstraction paths ultimately call the canonical verifier without introducing alternate verification branches.

Confirm:

- generated handlers route through canonical verification
- no branch omits subject binding
- no convenience path weakens failure behavior
- the generated `AccessContext` preserves separate transport-caller and
  authenticated-subject lanes
- default app guards that do not use `authenticated(...)` keep
  `AuthenticatedIdentitySource::RawCaller`
- `authenticated(...)` endpoints require first argument type `DelegatedToken`

### 3. Verify Drift Risk

Confirm handwritten and generated paths share tests or shared internal helpers for parity.

### 4. Guardrail Expectations

Run the auth trust-chain guard and record the result:

```bash
bash scripts/ci/run-auth-trust-chain-guards.sh
```

This guard is expected to reject:

- public `AuthApi::verify_token` or public `verify_token_material` helpers
- auth DTO verification/signing/key-resolution/replay behavior
- delegated endpoint guard ordering drift
- broad role-attestation refresh behavior

Also scan for material-only verifier exposure:

```bash
rg -n 'pub(\([^)]*\))?\s+(async\s+)?fn\s+verify_token\b|pub(\([^)]*\))?\s+(async\s+)?fn\s+verify_token_material\b|AuthApi::verify_token\b' crates/canic-core/src/api/auth crates/canic/src -g '*.rs'
```

Expected result: no matches.

### 5. Test Expectations

At least one integration test must exercise an abstraction-generated authenticated endpoint and verify:

- `token.sub != caller` => rejection
- expired token => rejection
- missing scope => rejection
- valid token => success

And parity tests must confirm handwritten and generated paths fail identically under the same invalid auth inputs.

Current focused test bundle:

```bash
cargo test -p canic-macros authenticated -- --nocapture
cargo test -p canic-macros access_stage_ -- --nocapture
cargo test -p canic-core --lib access::auth -- --nocapture
cargo test -p canic-core --lib verify_delegated_token -- --nocapture
cargo test -p canic-core --lib caller_predicates_use_transport_caller_not_authenticated_subject -- --nocapture
```

The `access::auth` bundle must include
`delegated_auth_guard_preserves_verify_bind_scope_consume_order`, subject
binding, required scope, update replay, query statelessness, and delegated
session resolution tests.

## Structural Hotspots

List concrete files/modules/structs that carry abstraction-equivalence risk.

Detection commands (run and record output references):

```bash
rg -l 'access::expr|eval_access|AccessExpr|AccessPredicate|BuiltinPredicate' crates canisters fleets -g '*.rs'
rg -l 'access::auth|delegated_token_verified|resolve_authenticated_identity|AuthenticatedIdentitySource|ResolvedAuthenticatedIdentity' crates canisters fleets -g '*.rs'
rg -l 'DelegationProof' crates canisters fleets -g '*.rs'
rg -l 'DelegatedTokenClaims|VerifiedDelegatedToken|VerifyDelegatedToken' crates canisters fleets -g '*.rs'
git log --name-only -n 20 -- crates/canic-macros crates/canic-core/src/access crates/canic-core/src/api/auth crates/canic-core/src/ops/auth crates/canic-core/src/dto/auth.rs
```

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-macros/src/endpoint/expand.rs` | `access_stage`, `build_access_plan` | abstraction wiring into auth runtime | High |
| `crates/canic-macros/src/endpoint/validate.rs` | `validate_authenticated_args` | compile-time shape guard for token-bearing endpoints | Medium |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext`, `eval_access` | canonical predicate dispatch surface and caller-lane boundary | High |
| `crates/canic-core/src/access/expr/evaluators.rs` | `AuthenticatedEvaluator` | dispatch from abstraction evaluator to canonical auth verifier | High |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | canonical verifier behavior baseline | High |
| `crates/canic-core/src/api/auth/session/mod.rs` | delegated session bootstrap | convenience path that must not replace endpoint auth semantics | Medium |

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

- generated path bypasses canonical verifier
- generated failure mapping differs from handwritten behavior for same invalid input
- abstraction-specific branch skips trust/scope ordering
- generated access context collapses transport caller and authenticated subject
- a public material-only verifier is exposed outside endpoint binding/replay
- default app guard starts resolving delegated-session identity
- DTO auth types gain verification, signing, replay, or policy behavior

## Severity

High to Critical depending on abstraction coverage.

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
- add `+4` for any confirmed abstraction parity break
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
- Abstractions reviewed:
- Canonical verifier reference:
- Result: `PASS` | `FAIL` | `PARTIAL`
- Parity evidence:
- Structural Hotspots:
- Hub Module Pressure:
- Early Warning Signals:
- Dependency Fan-In Pressure:
- Risk Score:
- Verification Readout:
- Follow-up actions:
