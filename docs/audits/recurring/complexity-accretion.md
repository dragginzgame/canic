# WEEKLY AUDIT — Complexity Accretion (`canic-core`)

## Purpose

Measure conceptual growth, branching pressure, and cognitive load expansion in `canic-core`.

This audit tracks structural entropy over time.

It is NOT:

* A correctness audit
* A style audit
* A redesign proposal exercise

Only evaluate conceptual complexity growth.

---

# Hard Constraints

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

# Explicit Anti-Shallow Requirement

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

# Canonical Subsystem Map (Mandatory)

Subsystem ownership for this audit:

| Subsystem | Path Scope |
| ---- | ---- |
| endpoints | `endpoints/**`, `macros/**` |
| workflow | `workflow/**` |
| policy | `policy/**`, `access/**` |
| ops | `ops/**` |
| dto | `dto/**` |
| model | `model/**` |
| storage | `storage/**` |
| api | `api/**` |

Rules:

* Each file must be assigned to exactly one subsystem.
* If a file spans domains, classify by primary responsibility.

---

# Layer Model (Mandatory)

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

# STEP 0 — Baseline Capture (Mandatory)

Capture baseline values before computing current metrics.

Baseline rule:

* Use the first run of the current day (`<scope>.md`) as `Previous`.
* If this is the first run of the day, mark `Previous` as `N/A` and treat this run as baseline.
* Do not compare reruns against other reruns on the same day.

| Metric | Previous | Current | Delta |
| ---- | ----: | ----: | ----: |
| Total runtime files in scope |  |  |  |
| Runtime LOC |  |  |  |
| Files >= 600 LOC |  |  |  |
| Capability mentions |  |  |  |
| Capability decision owners |  |  |  |
| Capability execution consumers |  |  |  |
| Capability plumbing modules |  |  |  |

---

# STEP 1 — Variant Surface Growth + Branch Multiplier

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

| Enum | Variants | Previous | Delta | Switch Sites | Branch Multiplier | Enum Density | Mixed Domain? | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- | ---- |

Definitions:

* `switch_sites = count(match/switch sites that alter control flow)`
* `branch_multiplier = variants x switch_sites`
* `enum_density = modules_using_enum / total_modules_in_scope`
* `variant_velocity = delta_per_week` (use `Delta` if weekly cadence is unchanged)

Switch Site Rule:

* Count only control-flow switches.
* Do NOT count:
  * serialization switches
  * debug/display formatting
  * test-only matches

Mixed Domain Enum Rule:

* If variants span more than one domain category (for example auth + replay + transport), mark `Mixed Domain`.

Flag:

* `branch_multiplier` trend up week-over-week
* enums `> 8` variants and still growing
* `enum_density > 0.25` and `variants > 6`
* mixed-domain enums with positive variant velocity

---

# STEP 2 — Execution Branching Pressure (Trend-Based)

Identify high-branch-density functions and compare against previous run.

| Function | Module | Branch Layers | Match Depth | Domains Mixed | Axis Coupling Index | Previous Branch Layers | Delta | Risk |
| ---- | ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- |

Axis families to detect:

* capability family
* proof mode (`Structural`, `RoleAttestation`, `DelegatedGrant`)
* replay state (miss/hit/conflict/expired)
* caller topology relation (root/child/registered-to-subnet)
* policy outcome (allow/deny)
* metadata validity (`request_id`, `ttl`, skew)

Definitions:

* `axis_coupling_index = branch_layers x domains_mixed`

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

# STEP 3 — Execution Path Multiplicity (Effective Flows)

For each core operation (`response_capability_v1`, `create_canister`, `upgrade_canister`, `cycles`, `issue_delegation`, `issue_role_attestation`), compute flow count via decision axes.

Model:

1. `theoretical_space = product(axis cardinalities)`
2. apply contract constraints and remove illegal combinations
3. `effective_flows = sum(valid combinations)`

Required axis set (add/remove only with explicit note):

* capability family
* proof mode
* replay status
* policy decision
* key/material availability
* caller topology relation

Axis Constraint Rule:

* Exclude combinations invalid by protocol design.
* Document removed combinations explicitly.

| Operation | Axes Used | Axis Cardinalities | Theoretical Space | Removed Combinations | Effective Flows | Previous Effective Flows | Delta | Shared Core? | Risk |
| ---- | ---- | ---- | ----: | ----: | ----: | ----: | ----: | ---- | ---- |

Flag:

* `effective_flows > 4`
* `axis_count >= 4`
* growth in effective flows without equivalent owner consolidation

---

# STEP 4 — Cross-Cutting Concern Spread (Authority vs Plumbing)

For each concept, classify usage by ownership and layer.

Target concepts:

* capability envelope validation
* capability hash binding
* replay key + payload hash semantics
* role attestation verification + key-set refresh behavior
* delegated grant verification path
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

# STEP 5 — Cognitive Load Indicators (Hub + Call Depth)

Compute structural mental-load signals:

1. functions >80-100 logical lines
2. deep core-operation call depth
3. hub pressure modules

Hub pressure definition:

* `LOC > 600` and `domain_count >= 3`

Hub escalation rule:

* Flag module if `LOC delta > 20%` week-over-week and `domain_count >= 3`.

Domain count categories:

* auth/attestation/delegation
* capability/replay
* rpc/request-dispatch
* policy/topology constraints
* storage/state projection
* lifecycle/timer/runtime

| Module/Operation | LOC or Call Depth | LOC Delta % | Domain Count | Previous | Delta | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ---- |

Flag:

* `call_depth > 6` for core operations
* rising hub pressure across consecutive runs
* hub escalation condition met

---

# STEP 6 — Drift Sensitivity (Axis Count)

Quantify areas where growth vectors multiply structural cost.

| Area | Decision Axes | Axis Count | Branch Multiplier | Drift Sensitivity | Risk |
| ---- | ---- | ----: | ----: | ---- | ---- |

Flag:

* `axis_count >= 4`
* branch multiplier growth tied to new variants

---

# STEP 7 — Complexity Risk Index (Semi-Mechanical)

Score each bucket 1-10, then compute weighted aggregate:

* variant explosion risk x2
* branching pressure trend x2
* flow multiplicity x2
* cross-layer spread x3
* hub pressure + call depth x2

| Area | Score (1-10) | Weight | Weighted Score |
| ---- | ----: | ----: | ----: |

`overall_index = weighted_sum / weight_sum`

Interpretation:

* 1-3 = low risk / structurally healthy
* 4-6 = moderate risk / manageable pressure
* 7-8 = high risk / requires monitoring
* 9-10 = critical risk / structural instability

---

# STEP 8 — Structural Entropy Drift

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

# STEP 9 — Refactor Noise Filter

Before finalizing risk, apply this filter:

* if concept mentions increase and decision owners decrease/hold, mark `refactor transient`
* if file count increases due to module split and hub pressure decreases, mark `structural improvement`

| Signal | Raw Trend | Noise Filter Result | Adjusted Interpretation |
| ---- | ---- | ---- | ---- |

---

# Required Summary

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

# Audit Stability Rule

Metrics must be computed using the same search patterns each week.

If a metric definition, search pattern, or counting scope changes:

* mark report as `methodology change`
* reset metric baselines for impacted measures
* mark impacted deltas as `N/A (methodology change)`

---

# Long-Term Goal of This Audit

Detect:

* capability-variant explosion before branching explosion
* flow multiplication before policy/dispatch divergence
* concept leakage before cross-layer drift
* cognitive load growth before fragility

This audit measures structural entropy, not code quality.
