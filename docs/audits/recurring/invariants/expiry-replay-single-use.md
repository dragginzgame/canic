# Audit: Expiry Replay Single-Use Invariant

## Purpose

Ensure credentials are rejected when stale, replayed, or reused beyond the system's defined freshness rules.

## Risk Model / Invariant

A credential must be rejected unless it satisfies the system's freshness rules.

Freshness rules include expiry validation, replay protection where applicable, and enforcement of single-use semantics where required.

Credentials must be rejected when:

- expired
- not yet valid (if applicable)
- replayed where replay protection applies (for credentials/requests with nonce, request-id, or `jti`-style identifiers)
- reused after single-use consumption (if applicable)

Single-use update credentials must be consumed before protected mutation so a
second active use fails closed. Query credentials remain stateless unless a
specific endpoint opts into durable replay protection.

## Why This Matters

Even correctly bound credentials become dangerous when freshness controls are bypassed.

## Relationship to Token Trust Chain

This invariant verifies credential freshness semantics.

The Token Trust Chain Invariant verifies issuer authenticity and chain validity.

## Run This Audit After

- expiry/TTL semantics changes
- replay store/nonce model changes
- request-id/jti changes
- clock skew policy changes
- update/query authentication boundary changes
- replay capacity or per-caller reservation limit changes

## Report Preamble (Required)

Every report generated from this audit must include:

- Scope
- Compared baseline report path
- Code snapshot identifier
- Method tag/version
- Comparability status

## Audit Checklist

### 1. Locate Freshness Checks

Search terms:

```text
exp
nbf
nonce
replay
used_token
token_uses
seen_jti
issued_at
```

Confirm:

- expiry is enforced centrally
- replay/nonce checks are enforced where required
- replay protection is enforced for credentials/requests carrying nonce, request-id, or `jti` identifiers
- update-call delegated tokens are consumed once by `(issuer_shard_pid, subject, cert_hash, nonce)`
- query-call delegated tokens do not write durable consumed-token state
- freshness logic is not optional in production paths

### 2. Verify State Interaction

Confirm replay markers or nonce records are updated atomically with the protected action they guard.

For update-call delegated tokens, confirm active consumed-token markers are
written before protected mutation and expire at the token expiry boundary.

For root capability requests, confirm per-caller replay reservation limits are
checked before global capacity so one caller cannot fill the shared replay
table.

### 3. Verify Clock Assumptions

Confirm clock skew tolerance is explicit and bounded.

When applicable, verify skew tolerance does not exceed token TTL.

### 4. Test Expectations

- expired token => rejection
- reused token or nonce => rejection
- reused update delegated token => rejection
- repeated query delegated token => success without durable consumption
- token used before `nbf` => rejection (if applicable)
- fresh token / nonce => success

Current suggested commands:

```bash
cargo test -p canic-core --lib update_token_consume_rejects_active_replay -- --nocapture
cargo test -p canic-core --lib query_token_consume_is_stateless -- --nocapture
cargo test -p canic-core --lib consume_rejects_active_replay -- --nocapture
cargo test -p canic-core --lib consume_allows_nonce_after_expiry_prune -- --nocapture
cargo test -p canic-core --lib consume_fails_closed_at_capacity -- --nocapture
cargo test -p canic-core --lib reserve_root_replay_rejects_caller_capacity_before_global_capacity -- --nocapture
```

## Structural Hotspots

List concrete files/modules/structs that carry freshness and replay risk.

Detection commands (run and record output references):

```bash
rg '^use ' crates/ -g '*.rs'
rg 'crate::workflow|crate::ops|crate::api|crate::policy' crates/ -g '*.rs'
rg 'pub struct|impl ' crates/ -g '*.rs'
git log --name-only -n 20 -- crates/
```

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `ops/auth/verify.rs` | `verify_time_bounds` | canonical expiry/nbf checks | High |
| `access/auth/token.rs` | `consume_update_token_once` | update/query delegated-token consumption boundary | High |
| `storage/stable/auth/token_uses.rs` | `consume_delegated_token_use` | durable consumed-token marker insertion/pruning | High |
| `ops/auth/token.rs` | `consume_delegated_token_use` | ops boundary for delegated-token replay consumption | High |
| `ops/replay/guard.rs` | replay guard decision surface | duplicate/conflict/ttl handling | High |
| `ops/replay/mod.rs` | reserve/commit/abort replay markers and per-caller cap | root replay freshness state transitions | Medium |
| `workflow/rpc/request/handler/replay.rs` | replay preflight orchestration | replay gate integration point | Medium |

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

- optional freshness guards in production flow
- replay marker writes detached from protected action
- broad/unbounded clock skew acceptance
- freshness checks implemented outside canonical verifier path
- replay store or nonce mechanism disabled/bypassed in production configuration
- consumed-token state keyed without issuer, subject, cert hash, or nonce
- root replay global capacity checked before per-caller capacity

## Severity

High to Critical depending on replay impact.

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
- add `+4` for any confirmed expiry/replay/single-use break
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
- Freshness mechanism reviewed:
- Replay store / nonce mechanism:
- Result: `PASS` | `FAIL` | `PARTIAL`
- Freshness evidence:
- Structural Hotspots:
- Hub Module Pressure:
- Early Warning Signals:
- Dependency Fan-In Pressure:
- Risk Score:
- Verification Readout:
- Follow-up actions:
