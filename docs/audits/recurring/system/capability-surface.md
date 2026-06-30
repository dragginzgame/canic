# Audit: Capability Surface

## Purpose

Track drift in Canic's exposed capability surface across endpoint bundles,
wire/protocol definitions, RPC capability DTOs, and operator/admin entrypoints.

This is a surface-governance audit.

It is NOT:

- a correctness audit
- a security proof audit
- a wasm-size-only audit
- a naming/style audit

## Risk Model / Invariant

This is a drift audit, not a security invariant audit.

Risk model:

- growing capability surface increases review cost and misuse risk
- global endpoint bundles amplify one local change across many canisters
- mixed-purpose endpoints create coupling between unrelated subsystems
- latent or dead globally exposed surface is over-bundling debt

Invariant:

> Capability surfaces must grow intentionally, stay attributable, and avoid
> unnecessary bundling across unrelated canister roles.

## Why This Matters

Canic ships shared macros and shared DTOs. A small endpoint or protocol change
can silently spread across many canisters, expand `.did` output, and increase
review burden far beyond the original feature.

This audit verifies:

- endpoint bundle growth
- wire/protocol constant growth
- RPC capability enum growth
- admin/internal surface growth
- bundling versus usage alignment
- global amplification factor for shared-surface changes

## Run This Audit After

- endpoint macro changes
- new admin/operator APIs
- RPC/capability DTO changes
- auth/delegation/attestation feature work
- root proof provisioning endpoint changes
- issuer-local delegated-token prepare/get/install/status changes
- topology/bootstrap/template control-plane changes
- parent/child capability routing changes

## Report Preamble (Required)

Every report generated from this audit must include:

- Scope
- Compared baseline report path
- Code snapshot identifier
- Method tag/version
- Comparability status
- Generated artifact environment and retained public roster
- Artifact refresh commands and their status

## Executive Summary Block (Required)

Every report must begin the findings section with a short executive summary:

- Risk Score
- Delta summary
- Largest growth contributor
- Over-bundled families (or explicit `none`)
- Follow-up required (`yes` / `no`)

## Report Completeness Gate (Required)

Every run must include all sections marked `Mandatory` in this template and all
sections required by `docs/audits/recurring/README.md`.

At minimum, a complete recurring report must include:

- Structural Hotspots
- Hub Module Pressure
- Global Amplification Factor
- Compatibility Signals
- Early Warning Signals
- Endpoint / RPC Alignment
- Dependency Fan-In Pressure
- Deterministic Risk Score
- Verification Readout

If any required section is omitted or blocked, mark the report
`PARTIAL (section gap)` in the comparability status and add a follow-up action
to complete or intentionally retire that section before the next run.

## Scope

Primary code areas:

- `crates/canic/src/macros/endpoints/**`
- `crates/canic/src/macros/start.rs`
- `crates/canic-core/src/protocol.rs`
- `crates/canic/src/protocol.rs`
- `crates/canic-core/src/dto/capability/**`
- `crates/canic-core/src/dto/rpc.rs`
- `crates/canic-core/src/api/rpc/**`
- generated `.did` files under `.icp/<environment>/canisters/**`

## False-Positive Filters (Required)

Exclude these from counts unless the audit explicitly says otherwise:

- `tests/`
- generated code outside canonical generated `.did` outputs
- internal test-only canisters that are present under `.icp/<environment>/canisters/**`
  but not part of the current fleet/environment roster
- comments / docstrings
- deprecated or legacy modules explicitly marked as such

If a report includes filtered exceptions, list them explicitly.

## Roster Selection (Required)

Generated `.did` directories can contain stale local artifacts from earlier
builds. Every run must derive the retained DID roster from the selected fleet
configuration before counting surface.

Record:

- selected fleet config path
- selected environment
- retained role list
- filtered local artifacts

Suggested role-list command:

```bash
scripts/ci/list-config-canisters.sh --config <fleet-config> --ci-order
```

Suggested retained-roster scan pattern:

```bash
for role in <retained roles>; do
    rg -n '^  canic_' ".icp/<environment>/canisters/$role/$role.did"
done
```

Do not use an unfiltered `.icp/<environment>/canisters/**` scan as the final
surface count when stale local artifacts are present.

## Capability Surface Unit (Normative)

For this audit, one capability surface unit is exactly one of:

- `1` endpoint method (`canic_*`)
- `1` protocol constant exposed to wire/API
- `1` RPC request variant
- `1` RPC response variant
- `1` capability proof variant

Growth must be measured in units and deltas across runs.

## Hard vs Drift Split (Required)

Every run must separate:

### A. Hard Surface Violations

These are binary `PASS` / `FAIL` checks.

Examples:

- endpoint unintentionally exposed globally because cfg-gating is missing
- admin/controller-only endpoint exposed in the wrong bundle
- `.did` mismatch across canisters where uniformity is expected
- protocol constant removed or renamed without compatibility note

### B. Surface Drift / Growth

These are trend checks and must be classified as:

- `STABLE`
- `GROWING`
- `OVER-BUNDLED`

Do not mix hard failures and drift interpretation in one table.

## Bundling Mode Definitions (Normative)

Every endpoint family must be classified as exactly one of:

- `global`: emitted in all canisters via shared macro composition
- `root-only`: emitted only in root canister bundles
- `non-root-only`: emitted only in non-root bundles
- `cfg-gated`: emitted behind compile-time cfgs/features
- `role-scoped`: required by only specific canister roles, even if currently emitted globally

Rule:

> Any role-scoped capability emitted as `global` is a bundling risk.

## Audit Checklist

### 1. Hard Surface Violations

Run hard checks first.

Suggested scans:

```bash
rg -n '^macro_rules!' crates/canic/src/macros/endpoints -g '*.rs'
rg -n 'canic_response_capability_v1|canic_wasm_store_|canic_upsert_root_issuer_policy|canic_upsert_root_issuer_renewal_template|canic_root_issuer_renewal_status|canic_get_or_create_chain_key_delegation_proof|canic_prepare_delegated_token|canic_get_delegated_token|canic_install_active_delegation_proof|canic_active_delegation_proof_status|canic_prepare_role_attestation|canic_get_role_attestation|canic_delegation_set_' crates/canic/src/macros/endpoints -g '*.rs'
rg -n '^  canic_.*_admin :' .icp/local/canisters -g '*.did'
rg -n 'cfg\\(canic_' crates/canic/src/macros/endpoints -g '*.rs'
```

Required checks:

- admin/controller-only endpoints are not exposed outside intended bundles
- shared parent/cycles receiver surface exists where expected
- root-only families are not present on non-root canisters unless explicitly intended
- root issuer policy/template/status endpoints are root-only and controller-gated
- chain-key lazy-repair proof retrieval is root-only, internal, and
  registered-subnet gated
- issuer-local delegated-token endpoints are not present on root unless explicitly intended
- protocol constant removals/renames are called out in compatibility notes

Record as:

| Hard Check | Result | Evidence |
| --- | --- | --- |
| `<check>` | `PASS/FAIL` | `<file, count, or grep result>` |

### 2. Endpoint Bundle Inventory

Count and classify generated endpoint bundles.

Suggested scans:

```bash
rg -n '^macro_rules!' crates/canic/src/macros/endpoints -g '*.rs'
rg -n '#\\[.*canic_(query|update)' crates/canic/src/macros/endpoints -g '*.rs'
rg -n 'admin\\(' crates/canic/src/macros/endpoints -g '*.rs'
```

Record:

- total endpoint bundle macros
- total generated endpoints
- internal endpoints
- controller-only/admin endpoints
- root-only endpoints
- non-root-only endpoints
- globally bundled endpoints

### 3. Wire Surface Inventory

Measure wire/protocol growth.

Suggested scans:

```bash
rg -n '^pub const ' crates/canic-core/src/protocol.rs
rg -n '^pub const ' crates/canic/src/protocol.rs
rg -n '^pub enum ' crates/canic-core/src/dto/{capability,rpc}.rs crates/canic-core/src/dto/capability -g '*.rs'
rg -n 'RequestFamily|CapabilityProof|CapabilityService' crates/canic-core/src -g '*.rs'
```

Record:

- `canic-core::protocol` constant count
- `canic::protocol` facade-only constant count
- `dto::rpc::Request` variant count
- `dto::rpc::Response` variant count
- `dto::capability::CapabilityProof` variant count
- `dto::capability::CapabilityService` variant count

### 4. Baseline Delta Summary (Mandatory)

Every report must include a delta table, even on first run.

Required format:

| Category | Previous | Current | Delta | % Change |
| --- | ---: | ---: | ---: | ---: |
| Endpoint methods |  |  |  |  |
| Protocol constants |  |  |  |  |
| RPC request variants |  |  |  |  |
| RPC response variants |  |  |  |  |
| Capability proof variants |  |  |  |  |

First run of day:

- `Previous = N/A`
- `Delta = N/A`
- `% Change = N/A`

### 5. Bundling vs Usage Alignment

Identify surfaces that are bundled globally but only exercised by a subset of
roles.

Suggested scans:

```bash
rg -n 'canic_response_capability_v1|canic_upsert_root_issuer_policy|canic_upsert_root_issuer_renewal_template|canic_root_issuer_renewal_status|canic_get_or_create_chain_key_delegation_proof|canic_prepare_delegated_token|canic_get_delegated_token|canic_install_active_delegation_proof|canic_active_delegation_proof_status|canic_prepare_role_attestation|canic_get_role_attestation|canic_delegation_set_|canic_wasm_store_|canic_sync_' .icp/local/canisters -g '*.did'
rg -n 'cfg\\(canic_' crates/canic/src/macros/endpoints -g '*.rs'
rg -n 'canic_response_capability_v1|canic_upsert_root_issuer_policy|canic_upsert_root_issuer_renewal_template|canic_root_issuer_renewal_status|canic_get_or_create_chain_key_delegation_proof|canic_prepare_delegated_token|canic_get_delegated_token|canic_install_active_delegation_proof|canic_active_delegation_proof_status|canic_prepare_role_attestation|canic_get_role_attestation|canic_delegation_set_|canic_wasm_store_|canic_sync_' crates/canic-core/src crates/canic/src -g '*.rs'
```

For each notable endpoint family, record:

- roles exposing it
- roles known to require it
- bundling mode
- risk if it grows further

At minimum, classify these auth/provisioning families when present:

- root delegated-auth renewal: `canic_upsert_root_issuer_policy`,
  `canic_upsert_root_issuer_renewal_template`,
  `canic_root_issuer_renewal_status`,
  `canic_get_or_create_chain_key_delegation_proof`
- issuer-local delegated token: `canic_prepare_delegated_token`,
  `canic_get_delegated_token`, `canic_install_active_delegation_proof`,
  `canic_active_delegation_proof_status`
- role-attestation: `canic_prepare_role_attestation`,
  `canic_get_role_attestation`
- retired compatibility guard: `canic_delegation_set_*`

### 6. Surface Utilization (Mandatory)

For each notable endpoint family, determine:

- `defined`: present in macro surface
- `exposed`: present in generated `.did`
- `used`: referenced in `api`, `workflow`, `ops`, or known external call sites

Suggested scans:

```bash
rg -n 'canic_<family>' crates/canic-core/src crates/canic/src crates/canic-tests/tests -g '*.rs'
```

Classify each family as:

- `active`
- `latent`
- `dead`

Rule:

> `latent` or `dead` + `global` = over-bundling candidate

### 7. DID Surface Growth

Use generated `.did` files for the current fleet/environment roster as the
consumer-facing surface proxy. See
`docs/architecture/build-artifacts.md` for the current artifact vocabulary.

If `.icp/<environment>/canisters/**` also contains internal test or audit
canisters, filter them out explicitly and name the retained roster in the
report preamble.

Suggested scans:

```bash
scripts/ci/list-config-canisters.sh --config <fleet-config> --ci-order
for role in <retained roles>; do
    rg -n '^service :' ".icp/local/canisters/$role/$role.did"
    rg -n '^  canic_' ".icp/local/canisters/$role/$role.did"
done
```

Required output:

#### Per-Canister Surface Table

| Canister | Total Methods | `canic_*` | Non-`canic` | Notes |
| --- | ---: | ---: | ---: | --- |
| `<role>` |  |  |  |  |

#### Outlier Rule

Flag a canister as an outlier if either:

- total method count > `minimal` baseline `+20%`, or
- `canic_*` methods exceed `minimal` baseline by more than `5`

Also record:

- shared methods present on all canisters
- large DTO type families that appear in many `.did` files

### 8. Surface Growth Attribution

For each shared endpoint family, assess whether growth pressure is coming from:

- lifecycle/runtime
- capability/RPC
- auth/delegation/attestation
- topology/state
- wasm/template control plane

Mark families as:

- `STABLE`
- `GROWING`
- `OVER-BUNDLED`

### 9. Structural Hotspots

List concrete files/modules driving capability-surface growth.

Detection commands:

```bash
rg '^use ' crates/ -g '*.rs'
rg 'pub enum|pub struct|pub fn|macro_rules!' crates/ -g '*.rs'
git log --name-only -n 20 -- crates/
```

Required format:

| File / Module | Surface Driver | Evidence | Risk Contribution |
| --- | --- | --- | --- |
| `<path>` | `<driver>` | `<count / churn / fan-in evidence>` | `<Low/Medium/High>` |

If none are detected in a given run, state:

`No structural hotspots detected in this run.`

### 10. Hub Module Pressure

Detect modules trending toward gravity-well behavior from import fan-in,
cross-layer coupling, and edit frequency.

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `<module>` | `<top import tokens>` | `<n>` | `<n>` | `<1-10>` |

Pressure score guidance:

- `1-3` = low
- `4-6` = moderate
- `7-10` = high

### 11. Global Amplification Factor (Mandatory)

Measure how many canisters are affected by one shared-surface addition.

Define:

> Global Amplification Factor (GAF) = number of canisters affected by one surface addition

Required table:

| Surface Change | Affected Canisters | GAF | Risk |
| --- | ---: | ---: | --- |
| `<family or endpoint>` |  |  |  |

Rule:

> Any change with `GAF >= 5` is automatically at least `Medium` risk.

### 12. Compatibility Signals

Track whether surface changes are only additive or carry compatibility risk.

Check:

- protocol constant changes
- enum variant removal/reordering
- RPC shape changes
- endpoint family renames/removals

Mark each as:

- `additive`
- `compatible but growing`
- `breaking risk`

### 13. Early Warning Signals

Detect predictive surface-bloat patterns before they become hard compatibility
or review problems.

Suggested scans:

```bash
rg 'pub enum |pub struct |pub fn ' crates/canic-core/src/{dto,api,workflow,ops} -g '*.rs'
rg '^  canic_' .icp/local/canisters -g '*.did'
git log --name-only -n 20 -- crates/
```

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| global endpoint family growth | `<path>` | `<count / diff evidence>` | `<Low/Medium/High>` |
| shared DTO fan-out | `<type/path>` | `<did or import spread>` | `<Low/Medium/High>` |
| admin surface clustering | `<path>` | `<count / growth evidence>` | `<Low/Medium/High>` |
| root proof provisioning surface growth | `<path>` | `<count / DID evidence>` | `<Low/Medium/High>` |
| issuer-local auth surface growth | `<path>` | `<count / DID evidence>` | `<Low/Medium/High>` |
| latent global surface | `<path>` | `<defined/exposed/used mismatch>` | `<Low/Medium/High>` |

### 14. Endpoint / RPC Alignment

Check both directions:

- RPC capability growth without corresponding endpoint usage indicates unused surface
- endpoint growth without RPC mapping indicates direct-call surface expansion and tighter coupling

Record mismatches explicitly.

### 15. Dependency Fan-In Pressure

Measure fan-in for surface-defining modules and DTOs.

Suggested scans:

```bash
rg -n 'dto::capability|dto::rpc|protocol::|canic_endpoints_' crates/ -g '*.rs'
```

| Module / Type | Referencing Files | Referencing Subsystems | Pressure | Notes |
| --- | ---: | --- | --- | --- |
| `dto::capability` |  |  |  |  |
| `dto::rpc` |  |  |  |  |
| `macros/endpoints/**` |  |  |  |  |
| `canic-core/src/protocol.rs` |  |  |  |  |
| `canic/src/protocol.rs` |  |  |  |  |

## Deterministic Risk Score (Required)

Start at `0`, then add:

- `+2` if endpoint count delta > `10%`
- `+2` if any DTO enum grows by more than `3` variants
- `+2` if a global bundle adds a new endpoint family
- `+2` if unused (`latent` or `dead`) endpoints exist in a global bundle
- `+1` if `.did` outliers are detected
- `+1` if DTO fan-out spans `>= 3` subsystems

Clamp final score to `0-10`.

Report:

- `Risk Score: X / 10`
- one paragraph explaining the score with file-backed evidence

## Verification Readout

Record command outcomes with:

- `PASS`
- `FAIL`
- `BLOCKED`

Minimum commands to report:

```bash
rg -n '^macro_rules!' crates/canic/src/macros/endpoints -g '*.rs'
rg -n '^pub const ' crates/canic-core/src/protocol.rs
rg -n '^pub const ' crates/canic/src/protocol.rs
rg -n '^  canic_' .icp/local/canisters -g '*.did'
rg -n 'canic_response_capability_v1|canic_upsert_root_issuer_policy|canic_upsert_root_issuer_renewal_template|canic_root_issuer_renewal_status|canic_get_or_create_chain_key_delegation_proof|canic_prepare_delegated_token|canic_get_delegated_token|canic_install_active_delegation_proof|canic_active_delegation_proof_status|canic_prepare_role_attestation|canic_get_role_attestation|canic_delegation_set_|canic_wasm_store_|canic_sync_' crates/canic-core/src crates/canic/src -g '*.rs'
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Follow-up Actions

If risk is `>= 6`, or any hard check is `FAIL`, or any key section is
`PARTIAL`/`BLOCKED`, include:

- owner boundary
- action
- target report date/run

If no follow-up is required, state that explicitly.
