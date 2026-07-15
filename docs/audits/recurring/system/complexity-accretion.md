# Audit: Complexity Accretion

## Method Contract

- Audit ID: `CANIC-COMPLEXITY-001`
- Method version: `2`
- Disposition: `retain`
- Owner: structural branching, decision-axis, and hub complexity trends
- Kind/profile: `trend` plus manual attribution
- Trace mode: `code_trace`
- Cost/runtime: high; 60-120 minutes
- Prerequisites: Git, ripgrep, GNU awk/coreutils,
  `docs/audits/scripts/measure-complexity-v2.sh`, and a method-compatible
  baseline
- False-positive boundary: size or fan-in alone is pressure, not a defect;
  findings require a concrete ownership, correctness, or change-risk link
- Shared contract: [AUDIT-HOWTO.md](../../AUDIT-HOWTO.md)

## Purpose

Measure conceptual growth, branching pressure, and cognitive load expansion in `canic-core`.

This audit tracks structural entropy over time.

It is NOT:

* A correctness audit
* A style audit
* A redesign proposal exercise

Only evaluate conceptual complexity growth.

## Risk Model / Invariant

This is a drift audit, not a correctness invariant audit.

Risk model:

- growing enum and branch surfaces increase change risk
- growing execution-path multiplicity increases regression risk
- spreading ownership across layers increases coordination cost

## Why This Matters

Unbounded structural entropy slows review, increases bug-introduction probability, and raises release risk over time.

## Run This Audit After

- control-plane feature additions
- request/capability/replay model changes
- large workflow/policy/ops refactors
- pre-release hardening windows

## Report Preamble (Required)

Every report generated from this audit must include:

- Scope
- Compared baseline report path
- Code snapshot identifier
- Method tag/version
- Comparability status

## Audit Checklist

### Hard Constraints

Do NOT discuss:

* Performance
* Code style
* Naming
* Macro aesthetics
* Minor duplication
* Refactors unless risk is high

Focus strictly on:

* Variant growth
* Branch growth
* Flow multiplication
* Concept scattering
* Cognitive stack depth

Assume this audit runs weekly and results are diffed.

---

### Explicit Anti-Shallow Requirement

Do NOT:

* Say "code looks clean"
* Give generic statements
* Provide unquantified claims
* Comment on naming/formatting/macro style

Every claim must reference:

* Count evidence
* Structural pattern
* Growth vector
* Branch multiplier or axis product

---

### Canonical Subsystem Map (Mandatory)

Subsystem ownership for this audit:

| Subsystem | Path Scope |
| ---- | ---- |
| access | `access/**` |
| api | `api/**` |
| bootstrap | `bootstrap/**` |
| cdk | `cdk/**` |
| config | `config/**` |
| dispatch | `dispatch/**` |
| domain | `domain/**` |
| dto | `dto/**` |
| format | `format/**` |
| ids | `ids/**` |
| infra | `infra/**` |
| ingress | `ingress/**` |
| lifecycle | `lifecycle/**` |
| memory | `memory/**` |
| model | `model/**` |
| ops | `ops/**` |
| replay-policy | `replay_policy/**` |
| role-contract | `role_contract/**` |
| root | root-level `*.rs` files under `crates/canic-core/src/` |
| storage | `storage/**` |
| test-support | `test/**`, `tests/**`, `test.rs`, `tests.rs`, `test_support.rs` |
| view | `view/**` |
| workflow | `workflow/**` |

Rules:

* Each file must be assigned to exactly one subsystem.
* If a file spans domains, classify by primary responsibility.
* A new top-level scope makes the run `partial`; add it only through a
  method-version update and rerun the original product baseline.

---

### Module Definition

A module for this audit means a Rust source file (`.rs`).
Module counts are file-level counts.

---

### LOC Counting Rule

`LOC` is the count of lines whose whitespace-trimmed form is nonempty and does
not start with `//`. This intentionally does not parse block comments or Rust
syntax. The exact counter is owned by
`docs/audits/scripts/measure-complexity-v2.sh`; changing that implementation is
a method-version change.

### Mechanical Measurement Identity

The v2 script is the canonical owner of file scope, test classification,
subsystem assignment, logical LOC, large-file counts, capability-mention file
count, fixed enum reference-file counts, fixed fan-in reference-file counts,
and lexical branch density.

Run it as:

```bash
bash docs/audits/scripts/measure-complexity-v2.sh <full-source-commit>
```

The report records the script fingerprint and retains its complete normalized
stdout. Search commands elsewhere in this definition are navigation aids, not
alternative counters.

V2 scope is the sorted Git tree at the named full commit under
`crates/canic-core/src/**/*.rs`. A file is test support when any relative path
component is `test` or `tests`, or its basename is `test.rs`, `tests.rs`, or
`test_support.rs`. Test classification takes precedence over subsystem
assignment. Every other file maps by its first path component using the table
above; a root-level file maps to `root`. Any unmapped file makes the run
`partial` and requires a method-version update.

Manual classifications remain necessary for switch sites, decision axes,
domain spread, decision owners, execution consumers, plumbing, effective-flow
constraints, and call depth. For each manual count, the report must retain the
exact file/function set and cardinality rationale. A bare count is invalid
evidence. Two reviewers or an explicit single-review waiver are required when
a manual classification creates or changes a P0/P1 finding.

---

### Domain Categories (Canonical)

Use this fixed set when counting domain spread:

* auth/attestation/delegation/root-proof provisioning
* capability/replay
* rpc/request-dispatch
* policy/topology constraints
* storage/state projection
* lifecycle/timer/runtime

---

### Layer Model (Mandatory)

Semantic layers (behavior ownership):

1. `policy` (authorization and protocol rules)
2. `workflow` (execution orchestration)
3. `ops` (side effects and system calls)
4. `model/storage` (state and projections)

Transport layers (data movement):

1. `dto`
2. `api`
3. `endpoints`

Rules:

* `semantic_layer_count` measures decision-logic spread.
* Transport layers do not count as semantic layers.

---

### STEP 0 — Baseline Capture (Mandatory)

Capture baseline values before computing current metrics.

Baseline rule:

* Use the first run of the current day (`<scope>.md`) as `Previous`.
* If this is the first run of the day, mark `Previous` as `N/A` and treat this run as baseline.
* Do not compare reruns against other reruns on the same day.
* For release-cycle trend analysis, `Previous` is the most recent prior run within the same release cycle.

| Metric | Previous | Current | Delta |
| ---- | ----: | ----: | ----: |
| Total runtime files in scope |  |  |  |
| Runtime LOC |  |  |  |
| Files >= 600 LOC |  |  |  |
| Non-test runtime files |  |  |  |
| Non-test runtime LOC |  |  |  |
| Non-test files >= 600 LOC |  |  |  |
| Capability mentions |  |  |  |
| Capability decision owners |  |  |  |
| Capability execution consumers |  |  |  |
| Capability plumbing modules |  |  |  |

---

### STEP 1 — Variant Surface Growth + Branch Multiplier

Quantify the following:

* `dto::rpc::Request`
* `dto::rpc::Response`
* `dto::capability::CapabilityProof`
* `dto::capability::CapabilityService`
* `access::expr::BuiltinPredicate`
* `workflow::rpc::request::handler::RootCapability`
* root capability metric event enums
* auth/delegation/attestation error enums
* infra error envelope enums (`InfraError`, `InternalErrorClass`, equivalents)

| Enum | Variants | Previous | Delta | Variant Velocity | Switch Sites | Branch Multiplier | Enum Density | Mixed Domain? | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ----: | ---- | ---- |

Definitions:

* `switch_sites = count(match/switch sites that alter control flow)`
* `branch_multiplier = variants x switch_sites`
* `enum_density = modules_using_enum / total_modules_in_scope`
* `variant_velocity = delta_variants_per_week` (use `Delta` if weekly cadence is unchanged)

Switch Site Rule:

* Count only control-flow switches.
* Do NOT count:
  * serialization switches
  * debug/display formatting
  * test-only matches

Mixed Domain Enum Rule:

* If variants span more than one domain category (for example auth + replay + transport), mark `Mixed Domain`.

Switch-Site Search Examples:

```bash
rg -n 'match .*CapabilityProof' crates/canic-core/src -g '*.rs'
rg -n 'match .*Request' crates/canic-core/src -g '*.rs'
rg -n 'match .*Response' crates/canic-core/src -g '*.rs'
```

Flag:

* `branch_multiplier` trend up week-over-week
* enums `> 8` variants and still growing
* `enum_density > 0.25` and `variants > 6`
* mixed-domain enums with positive variant velocity

---

### STEP 2 — Execution Branching Pressure (Trend-Based)

Identify high-branch-density functions and compare against previous run.

| Function | Module | Branch Layers | Match Depth | Domains Mixed | Axis Coupling Index | Previous Branch Layers | Delta | Risk |
| ---- | ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- |

Axis families to detect:

* capability family
* capability proof mode (`Structural`)
* auth proof lifecycle (`root delegation batch`, `active proof install`,
  `issuer token proof`, `signed role attestation`)
* replay state (miss/hit/conflict/expired)
* caller topology relation (root/child/registered-to-subnet)
* policy outcome (allow/deny)
* metadata validity (`request_id`, `ttl`, skew)

Definitions:

* `branch_layers = number of independent decision layers that alter execution flow in a function`
* `match_depth = maximum nested match/if decision depth within the function`
* `axis_coupling_index = branch_layers x domains_mixed`

`match_depth` example:

```rust
match a {
    A => {
        if cond {
            match b {
                B => {}
                _ => {}
            }
        }
    }
    _ => {}
}
```

In this example, `match_depth = 2`.

Interpretation:

* `<= 4` low
* `5-8` moderate
* `> 8` high

Flag:

* `domains_mixed > 3`
* positive weekly branch-layer growth
* functions where enum growth increased branch layers
* high axis coupling index

---

### STEP 3 — Execution Path Multiplicity (Effective Flows)

For each core operation, compute flow count via decision axes.

Current core operation set:

* `response_capability_v1`
* `create_canister`
* `upgrade_canister`
* `cycles`
* `prepare_delegated_token`
* `role_attestation_prepare/get`
* `root_delegation_proof_batch_prepare/get/install`
* `active_delegation_proof_install/status`

Model:

1. `theoretical_space = product(axis cardinalities)`
2. apply contract constraints and remove illegal combinations
3. `effective_flows = sum(valid combinations)`

Required axis set (add/remove only with explicit note):

* capability family
* proof mode or auth-proof lifecycle phase
* replay status
* policy decision
* key/material availability
* caller topology relation

Axis Constraint Rule:

* Exclude combinations invalid by protocol design.
* Document removed combinations explicitly.

Axis Cardinality Rule:

* `axis cardinality = number of valid runtime states considered for that axis`
* example: `proof mode cardinality = number of valid proof variants`
* example: `replay status cardinality = number of replay states`

| Operation | Axes Used | Axis Cardinalities | Theoretical Space | Removed Combinations | Effective Flows | Previous Effective Flows | Delta | Shared Core? | Risk |
| ---- | ---- | ---- | ----: | ----: | ----: | ----: | ----: | ---- | ---- |

Flag:

* `effective_flows > 4`
* `axis_count >= 4`
* growth in effective flows without equivalent owner consolidation

---

### STEP 4 — Cross-Cutting Concern Spread (Authority vs Plumbing)

For each concept, classify usage by ownership and layer.

Target concepts:

* capability envelope validation
* capability hash binding
* replay key + payload hash semantics
* role attestation verification + key-set refresh behavior
* root delegation proof batch prepare/get/install
* active delegation proof install/status
* delegated-token issuer proof verification
* error origin mapping (`InfraError` / `InternalError` / boundary `Error`)

| Concept | Decision Owners | Execution Consumers | Plumbing Modules | Total Modules | Semantic Layers | Transport Layers | Decision Concentration | Concept Fragmentation | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ----: | ----: | ---- |

Definitions:

* `decision_owners = modules defining protocol rules/policies`
* `execution_consumers = modules branching on concept state`
* `plumbing_modules = DTO/transport/projection carriers`
* `decision_concentration = top_owner_mentions / total_decision_mentions`
* `concept_fragmentation = decision_owners + execution_consumers`

Interpretation:

* `decision_concentration > 0.60` strong ownership
* `decision_concentration 0.40-0.60` distributed
* `decision_concentration < 0.40` fragmented

Flag:

* `semantic_layer_count >= 3`
* concept fragmentation `>= 7`
* decreasing decision concentration with growing module spread

---

### STEP 5 — Cognitive Load Indicators (Hub + Call Depth)

Compute structural mental-load signals:

1. functions >80-100 logical lines
2. deep core-operation call depth
3. hub pressure modules

Hub pressure definition:

* `LOC > 600` and `domain_count >= 3`
* `domain_count = number of canonical domain categories referenced within the module's public functions`

Hub escalation rule:

* Flag module if `LOC delta > 20%` week-over-week and `domain_count >= 3`.

| Module/Operation | LOC or Call Depth | LOC Delta % | Domain Count | Previous | Delta | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ---- |

Flag:

* `call_depth > 6` for core operations
* rising hub pressure across consecutive runs
* hub escalation condition met

---

### STEP 6 — Drift Sensitivity (Axis Count)

Quantify areas where growth vectors multiply structural cost.

| Area | Decision Axes | Axis Count | Branch Multiplier | Drift Sensitivity | Risk |
| ---- | ---- | ----: | ----: | ---- | ---- |

Flag:

* `axis_count >= 4`
* branch multiplier growth tied to new variants

Axis families allowed in this audit:

* capability family
* proof mode or auth-proof lifecycle phase
* replay state
* policy outcome
* caller topology
* lifecycle phase

Optional hotspot metric:

* `branch_entropy = branch_multiplier x axis_coupling_index`

---

### STEP 7 — Complexity Risk Index (Deterministic)

Score each bucket 1-10, then compute weighted aggregate:

* variant explosion risk x2
* branching pressure trend x2
* flow multiplicity x2
* cross-layer spread x3
* hub pressure + call depth x2

| Area | Score (1-10) | Weight | Weighted Score |
| ---- | ----: | ----: | ----: |

`overall_index = weighted_sum / weight_sum`

Each bucket starts at 1 and adds only its owned signals below, clamped to 10.
A signal may be discussed elsewhere but contributes to exactly one bucket.

- Variant explosion: `+2` when any listed enum has more than 8 variants;
  `+2` when any branch multiplier is at least 40; `+2` when any listed enum is
  referenced by at least 10 non-test files; `+1` when any listed enum is mixed
  domain; `+2` when a comparable run has positive variant velocity.
- Branching pressure: `+2` when any ACI is above 8; `+2` when at least three
  hotspots have ACI above 8; `+2` when any non-test hotspot has branch density
  above 3.0; `+1` when match depth is above 3; `+2` when a comparable run has
  positive branch-layer growth.
- Flow multiplicity: `+2` when any effective-flow count is above 4; `+2` when
  at least five operations exceed 4; `+2` when any operation exceeds 12;
  `+1` when at least five operations use four or more axes; `+2` when a
  comparable run has positive effective-flow growth.
- Cross-layer spread: `+2` when any concept spans at least three semantic
  layers; `+2` when any concept fragmentation is at least 15; `+2` when any
  decision concentration is below 0.40; `+1` when any concept spans all three
  transport layers; `+2` when a fixed fan-in surface is referenced by at least
  12 non-test files across multiple subsystems; `+2` when a comparable run has
  positive fragmentation growth.
- Hub pressure and call depth: `+2` when any strict hub exists; `+2` when at
  least three strict hubs exist; `+2` when any selected operation has call
  depth above 6; `+1` when any non-test file has at least 1,000 logical LOC;
  `+2` when a comparable run meets the hub-escalation condition. CAF is
  informational and never changes this score.

Round the weighted result to the nearest integer, with exact `.5` rounding
up. That rounded value is the report risk score; v2 permits no additive
override or duplicate modifier.

Interpretation:

* 1-3 = low risk / structurally healthy
* 4-6 = moderate risk / manageable pressure
* 7-8 = high risk / requires monitoring
* 9-10 = critical risk / structural instability

---

### STEP 8 — Structural Entropy Drift

Track slow architecture drift signals.

| Signal | Previous | Current | Delta | Risk |
| ---- | ----: | ----: | ----: | ---- |
| enum_density_avg |  |  |  |  |
| axis_coupling_avg |  |  |  |  |
| concept_fragmentation_avg |  |  |  |  |
| hub_modules |  |  |  |  |

Flag:

* if any two metrics increase in the same week, escalate drift risk

---

### STEP 9 — Refactor Noise Filter

Before finalizing risk, apply this filter:

* if concept mentions increase and decision owners decrease/hold, mark `refactor transient`
* if file count increases due to module split and hub pressure decreases, mark `structural improvement`

| Signal | Raw Trend | Noise Filter Result | Adjusted Interpretation |
| ---- | ---- | ---- | ---- |

---

### Required Summary

1. Overall Complexity Risk Index
2. Fastest Growing Concept Families (rank by `growth_score = variant_delta + switch_site_delta + owner_delta`)
3. Highest Branch Multipliers
4. Highest Axis Coupling Index Hotspots
5. Flow Multiplication Risks (axis-based)
6. Cross-Layer Spread Risks (owner vs plumbing aware)
7. Concept Fragmentation Warnings
8. Hub Pressure + Call-Depth Warnings
9. Structural Entropy Drift Findings
10. Refactor-Transient vs True-Entropy Findings

---

### Audit Stability Rule

Metrics must be computed using the same search patterns each week.

If a metric definition, search pattern, or counting scope changes:

* mark report as `methodology change`
* reset metric baselines for impacted measures
* mark impacted deltas as `N/A (methodology change)`

---

### Long-Term Goal of This Audit

Detect:

* capability-variant explosion before branching explosion
* flow multiplication before policy/dispatch divergence
* concept leakage before cross-layer drift
* cognitive load growth before fragility

This audit measures structural entropy, not code quality.

## Structural Hotspots

List concrete files/modules/structs that contribute to complexity pressure.

Navigation commands (do not use as alternative counters):

```bash
rg '^use ' crates/ -g '*.rs'
rg 'crate::workflow|crate::ops|crate::api|crate::policy' crates/ -g '*.rs'
rg 'pub struct|impl ' crates/ -g '*.rs'
git log --name-only -n 20 -- crates/
```

Large-file scans must report both total and non-test counts. Total `>= 600 LOC`
files identify maintainability pressure, but non-test `>= 600 LOC` files drive
runtime complexity risk by default.

### Runtime Complexity Hotspots

Runtime hotspots are production complexity signals and should drive primary risk scoring.

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `access/expr/*` | auth predicate evaluators | dense branch and predicate dispatch surface | Medium |
| `workflow/rpc/request/handler/*` | request/capability handlers | multi-axis decision branching | High |
| `ops/replay/guard.rs` | replay decision path | axis coupling between ttl/request-id/payload state | Medium |
| `ops/auth/delegation/mod.rs` | root proof batch provisioning | prepare/get/install branching and pending metadata lifecycle | Medium |
| `ops/auth/delegated/*` | delegated-token verifier flows | auth variant expansion and branching | Medium |

### Test Complexity Hotspots

Test hotspots are tracked for maintainability but must not inflate runtime risk by default.

| Test File / Module | Reason | Tracking Impact |
| --- | --- | --- |
| `<tests path>` | `<harness complexity>` | `<Low/Medium/High>` |

If no test hotspots are detected, state: No test complexity hotspots detected in this run.

If no runtime hotspots are detected, state: No structural hotspots detected in this run.

## Control Surface Detection

Detect central control surfaces that multiply downstream complexity and change impact.

| Control Surface | File | Responsibility | Risk |
| --- | --- | --- | --- |
| `<function/module>` | `<path>` | `<decision/coordination boundary>` | `<Low/Medium/High>` |
| `eval_access` | `access/expr/mod.rs` | capability/auth evaluation engine | Medium |
| `runtime bootstrap` | `workflow/runtime/mod.rs` | system initialization coordination | Medium |
| `intent aggregation` | `ops/storage/intent/mod.rs` | state transition aggregation boundary | Medium |

## Branching Density

Track branch density per hotspot file to detect logic concentration trends.

Detection scans:

```bash
rg -n '\\bmatch\\b' <file>
rg -n '\\bif\\b' <file>
rg -n 'else if' <file>
```

Definitions:

- `branch_count = match_count + if_count + else_if_count`
- `branch_density = (branch_count / logical_loc) * 100`

| File | Logical LOC | `match` | `if` | `else if` | Branch Density (/100 LOC) | Runtime/Test | Risk |
| --- | ---: | ---: | ---: | ---: | ---: | --- | --- |
| `<path>` | `<n>` | `<n>` | `<n>` | `<n>` | `<n.nn>` | `<runtime|test>` | `<Low/Medium/High>` |

Interpretation:

- `< 1.5` low branching density
- `1.5 - 3.0` moderate branching density
- `> 3.0` high branching density

## Hub Module Pressure

Detect modules trending toward gravity-well behavior from import fan-in, cross-layer coupling, and edit frequency.

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `<module>` | `<top import tokens>` | `<n>` | `<n>` | `<1-10>` |

Pressure score guidance:

- 1-3 = low
- 4-6 = moderate
- 7-10 = high

## Amplification Drivers (If Applicable)

When complexity pressure increases due to recent feature slices, record the largest amplifiers.

For v2, CAF is informational and uses the fixed formula
`max(subsystem_count, architecture_layer_count) * flow_axis_count`.
`subsystem_count` uses the canonical subsystem map above.
`architecture_layer_count` uses the exact set `endpoints`, `api`, `workflow`,
`policy`, `ops`, `model/storage`, `dto`, and `other`, derived from the first
matching relative path component; `flow_axis_count` is the count of explicitly
listed runtime decision axes from the allowed axis families. The report must
name the exact commit range, touched file set, subsystem set, layer set, and
axis set. CAF does not alter the v2 risk score.

| Commit | Feature Slice | Files Touched | Subsystems | CAF | Risk |
| --- | --- | ---: | --- | ---: | --- |
| `<commit>` | `<feature>` | `<n>` | `<subsystems>` | `<caf>` | `<risk>` |

Navigation commands (do not use as alternative counters):

```bash
git log --name-only -n 20 -- crates/
```

## Red Flags

- branch multipliers trending up without owner consolidation
- effective flow count growth across core operations
- rising concept fragmentation across semantic layers
- repeated hub pressure growth without decomposition

## Severity

- Low: bounded growth with stable ownership
- Medium: moderate growth with increasing coupling
- High: rapid growth in branching/path multiplicity
- Critical: sustained unstable growth across multiple key signals

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

Use the rounded `overall_index` from Step 7 as the risk score. No override,
post-hoc modifier, or second scoring algorithm is permitted. Hub, enum,
cross-layer, capability-surface, and fan-in signals contribute only through
their single owning Step 7 bucket.

If no confirmed findings and no hotspot/hub/amplification signals are present, score must remain `0-2`.

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
- Baseline:
- Overall index:
- Key hotspots:
- Method/comparability:
- Structural Hotspots:
- Hub Module Pressure:
- Amplification Drivers:
- Early Warning Signals:
- Dependency Fan-In Pressure:
- Risk Score:
- Verification Readout:
- Follow-up actions:
