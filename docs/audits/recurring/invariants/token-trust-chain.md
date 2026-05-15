# Audit: Token Trust Chain Invariant

## Purpose

Ensure token acceptance requires a valid issuer trust chain from root authority
to shard authority to the delegated-token claims.

## Risk Model / Invariant

A token must be rejected unless the complete issuer trust chain validates from
root authority to shard authority to token claims.

Chain validation must verify:

- root authority identity
- root public key trust anchor sourced from verifier-local state
- shard authority certification by root signature over canonical cert hash
- delegated-token claims bound to the signed cert
- token claims signature under the certified shard key
- shard key binding to Canic's configured signing key and derivation path

Freshness checks are verified by the Expiry / Replay / Single-Use Invariant.
This audit may cite freshness/replay checks as evidence only where they are
part of the delegated-token acceptance path; do not score a freshness-only
finding here unless it also lets an invalid trust chain pass.

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

Current expected hotspots:

- `crates/canic-core/src/ops/auth/token.rs`
- `crates/canic-core/src/ops/auth/delegated/verify.rs`
- `crates/canic-core/src/ops/auth/delegated/root_key.rs`
- `crates/canic-core/src/access/auth/token.rs`
- `crates/canic-core/src/ops/auth/attestation.rs`

### 2. Verify Chain Validation

Search terms:

```text
verify_root
verify_shard
issuer
signature
certificate
cert_hash
root_sig
shard_sig
RootTrustAnchor
verify_delegated_token
verify_token
```

Confirm:

- each trust layer is verified before token acceptance
- verifier-local trust objects retain cryptographic integrity and do not bypass required verification steps
- issuer identity is bound to expected authority
- root key identity checks cover root pid, key id, key hash, algorithm, and time window
- claims bind issuer shard pid and cert hash back to the signed cert
- shard key binding matches configured key name and shard derivation path
- endpoint guard code verifies the token before subject binding, required-scope
  checks, and update-call replay consumption
- role-attestation verification uses cached trusted keys and only refreshes on
  unknown key id

Record the chain evidence as an ordered stage table. At minimum it must show:

1. runtime config gate
2. shard key binding
3. verifier-local root trust anchor
4. root key identity/window resolution
5. canonical certificate hash
6. root signature
7. claims-to-cert binding
8. canonical claims hash
9. shard signature
10. endpoint subject/scope/replay boundary

### 3. Verify Negative Cases

Confirm rejection for:

- invalid root
- invalid shard
- invalid token signature
- unexpected issuer relationship
- expired or revoked issuer certificate
- cert hash drift
- noncanonical certificate or claims data
- missing root or shard signature

### 4. Test Expectations

- self-contained delegated token with valid root and shard signatures => acceptance
- invalid root signature => rejection
- invalid shard signature => rejection
- cert hash drift => rejection
- noncanonical cert or claims vectors => rejection
- root pid mismatch => rejection
- runtime root-key propagation path => acceptance
- role-attestation subject/audience/epoch/expiry/signature rejection paths => rejection
- stale proof-store/current-proof trace helpers are absent unless the verifier
  model has deliberately changed again

Current suggested commands:

```bash
cargo test -p canic-core --lib verify_delegated_token -- --nocapture
cargo test -p canic-core --lib resolve_root_key -- --nocapture
cargo test -p canic-tests --test root_suite delegated_token_verification_uses_cascaded_subnet_state_root_key -- --nocapture
cargo test -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture
rg "trace_token_trust_chain|token_chain|proof_state|verify_delegation_signature|verify_token_sig|authenticated_guard_checks_current_proof" crates -n
```

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
| `ops/auth/token.rs` | `verify_token`, `root_trust_anchor`, `verify_shard_key_binding` | runtime trust-chain orchestration entrypoint | High |
| `ops/auth/delegated/verify.rs` | `verify_delegated_token`, `verify_claims` | pure root/shard/token verification order | High |
| `ops/auth/delegated/root_key.rs` | `resolve_root_key` | root trust-anchor identity and validity checks | High |
| `access/auth/token.rs` | `delegated_token_verified`, `enforce_subject_binding` | endpoint guard integration | Medium |
| `ops/auth/attestation.rs` | `verify_role_attestation_cached` | role-attestation key and signature verification | Medium |

If none are detected in a given run, state: No structural hotspots detected in this run.

## Hub Module Pressure

Detect modules trending toward gravity-well behavior from import fan-in, cross-layer coupling, and edit frequency.

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

- trust chain step skipped on an internal path
- token signature accepted without issuer-root linkage
- verifier-local trust anchor bypassed by token-provided key material
- shard key accepted without checking configured key name and derivation path
- claims accepted without matching issuer shard pid and cert hash
- trust anchor loaded from runtime configuration without validation
- passive DTOs gaining behavior or validation methods
- role-attestation refresh paths retrying on errors other than unknown key id

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

Score must be justified using checklist findings and Structural Hotspots
evidence. Separate the security verdict from structural watchpoints: a PASS can
still have structural pressure, but passive DTO fan-in alone must not dominate
the score.

Derivation guidance (deterministic):

- start at `0`
- add `+4` for any confirmed trust-chain validation break
- add `+3` if a required trust-chain stage is present but ordered after token
  acceptance or endpoint execution
- add `+2` if verifier-local trust-anchor evidence is missing or ambiguous
- add `+2` if required unit verifier/root-key tests are not run or are blocked
- add `+2` if the runtime root-key cascade or role-attestation PocketIC path is
  not run or is blocked
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
