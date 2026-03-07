# WEEKLY AUDIT — Complexity Accretion (canic-core)

## Purpose

Measure **conceptual growth, branching pressure, and cognitive load expansion** in `canic-core`.

This audit tracks structural entropy over time.

It is NOT a correctness audit.
It is NOT a style audit.
It is NOT a redesign proposal exercise.

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

# STEP 0 — Baseline Capture (Mandatory)

Capture previous-run values before computing current metrics.

Produce:

| Metric | Previous | Current | Delta |
| ---- | ----: | ----: | ----: |
| Total runtime files in scope |  |  |  |
| Runtime LOC |  |  |  |
| Files >= 600 LOC |  |  |  |
| Capability mentions |  |  |  |
| Capability decision owners |  |  |  |
| Capability execution consumers |  |  |  |
| Capability plumbing modules |  |  |  |

If previous values are unavailable, mark `N/A` and treat this run as baseline.

---

# STEP 1 — Variant Surface Growth + Branch Multiplier

Quantify the following:

* `dto::rpc::Request` variant count
* `dto::rpc::Response` variant count
* `dto::capability::CapabilityProof` variant count
* `dto::capability::CapabilityService` variant count
* `access::expr::BuiltinPredicate` variant count
* `workflow::rpc::request::handler::RootCapability` variant count
* Root capability metric event enum variants
* Auth/delegation/attestation error enum variants
* Infra error envelope variants (`InfraError`, `InternalErrorClass`, equivalents)

For each:

| Enum | Variants | Switch Sites | Branch Multiplier | Domain Scope | Mixed Domains? | Growth Risk |
| ---- | ----: | ----: | ----: | ---- | ---- | ---- |

Definitions:

* `switch_sites` = number of distinct match/switch callsites over that enum in runtime scope.
* `branch_multiplier` = `variants × switch_sites`.

Flag:

* `branch_multiplier` trend up week-over-week.
* Enums >8 variants and still growing.
* Enums mixing auth + policy + transport + storage semantics.

---

# STEP 2 — Execution Branching Pressure (Trend-Based)

Identify high-branch-density functions and compare against previous run.

For each hotspot:

| Function | Module | Branch Layers | Match Depth | Previous Branch Layers | Delta | Domains Mixed | Risk |
| ---- | ---- | ----: | ----: | ----: | ----: | ----: | ---- |

Also detect axis coupling in each function:

* capability family
* proof mode (`Structural` / `RoleAttestation` / `DelegatedGrant`)
* replay state (miss/hit/conflict/expired)
* caller topology relation (root/child/registered-to-subnet)
* policy outcome (allow/deny)
* metadata validity (`request_id`, `ttl`, skew)

Flag:

* Any function with `domains_mixed > 3`.
* Positive weekly branch-layer growth.
* Functions where enum growth directly increased branch layers.

---

# STEP 3 — Execution Path Multiplicity (Effective Flows)

For each core operation (`response_capability_v1`, `create_canister`, `upgrade_canister`, `cycles`, `issue_delegation`, `issue_role_attestation`), compute flow count via decision axes.

Use this model:

1. `theoretical_space = Π(axis cardinalities)`
2. Apply contract constraints and remove illegal combinations.
3. `effective_flows = sum(valid combinations)`

Required axis set (add/remove only with explicit note):

* capability family
* proof mode
* replay status
* policy decision
* key/material availability
* caller topology relation

Produce:

| Operation | Axes Used | Axis Cardinalities | Theoretical Space | Effective Flows | Previous Effective Flows | Delta | Shared Core? | Risk |
| ---- | ---- | ---- | ----: | ----: | ----: | ----: | ---- | ---- |

Flag:

* `effective_flows > 4` (pressure)
* `axis_count >= 4` (multiplication onset)
* growth in effective flows without equivalent owner consolidation

---

# STEP 4 — Cross-Cutting Concern Spread (Authority vs Plumbing)

For each concept, classify usage by ownership and layer.

Target concepts:

* Capability envelope validation
* Capability hash binding
* Replay key + payload hash semantics
* Role attestation verification + key-set refresh behavior
* Delegated grant verification path
* Error origin mapping (`InfraError` / `InternalError` / boundary `Error`)

Produce:

| Concept | Decision Owners | Execution Consumers | Plumbing Modules | Total Modules | Semantic Layers | Transport Layers | Risk |
| ---- | ----: | ----: | ----: | ----: | ---- | ---- | ---- |

Definitions:

* `decision owners` = modules that define protocol rules/policies.
* `execution consumers` = modules that branch on concept state to execute behavior.
* `plumbing modules` = DTO/transport/projection modules that only carry values.

Risk should be driven by `decision owners` and `semantic layers`, not raw mention totals.

Flag:

* `semantic_layer_count >= 3` (architectural leakage).
* semantic owner growth without explicit boundary consolidation.

---

# STEP 5 — Cognitive Load Indicators (Hub + Call Depth)

Compute structural mental-load signals:

1. Functions > 80–100 logical lines.
2. Deep core-operation call depth.
3. Hub pressure modules.

Hub pressure definition:

* `LOC > 600` AND `domain_count >= 3`

Domain count categories:

* auth/attestation/delegation
* capability/replay
* rpc/request-dispatch
* policy/topology constraints
* storage/state projection
* lifecycle/timer/runtime

Produce:

| Module/Operation | LOC or Call Depth | Domain Count | Previous | Delta | Risk |
| ---- | ----: | ----: | ----: | ----: | ---- |

Flag:

* `call_depth > 6` for core operations.
* rising hub pressure across consecutive runs.

---

# STEP 6 — Drift Sensitivity (Axis Count)

Quantify areas where growth vectors multiply structural cost.

Produce:

| Area | Decision Axes | Axis Count | Branch Multiplier | Drift Sensitivity | Risk |
| ---- | ---- | ----: | ----: | ---- | ---- |

Flag:

* `axis_count >= 4`
* branch multiplier growth tied to new variants

---

# STEP 7 — Complexity Risk Index (Semi-Mechanical)

Score each bucket 1–10, then compute weighted aggregate:

* variant explosion risk ×2
* branching pressure trend ×2
* flow multiplicity ×2
* cross-layer spread ×3
* hub pressure + call depth ×2

Produce:

| Area | Score (1-10) | Weight | Weighted Score |
| ---- | ----: | ----: | ----: |

`overall_index = weighted_sum / weight_sum`

Interpretation:

* 1–3 = Low risk / structurally healthy
* 4–6 = Moderate risk / manageable pressure
* 7–8 = High risk / requires monitoring
* 9–10 = Critical risk / structural instability

---

# STEP 8 — Refactor Noise Filter

Before finalizing risk, apply this filter:

* If concept mentions increase **and** decision owners decrease/hold,
  mark as `refactor transient`.
* If file count increases due module split **and** hub pressure decreases,
  mark as `structural improvement`.

Produce:

| Signal | Raw Trend | Noise Filter Result | Adjusted Interpretation |
| ---- | ---- | ---- | ---- |

---

# Required Summary

1. Overall Complexity Risk Index
2. Fastest Growing Concept Families
3. Highest Branch Multipliers
4. Flow Multiplication Risks (axis-based)
5. Cross-Layer Spread Risks (owner vs plumbing aware)
6. Hub Pressure + Call-Depth Warnings
7. Refactor-Transient vs True-Entropy Findings

---

# Explicit Anti-Shallow Requirement

Do NOT:

* Say "code looks clean"
* Give generic statements
* Provide unquantified claims
* Comment on naming
* Comment on macro usage
* Comment on formatting

Every claim must reference:

* Count
* Structural pattern
* Growth vector
* Branch multiplier or axis product

---

# Long-Term Goal of This Audit

Detect:

* Capability-variant explosion before branching explosion
* Flow multiplication before policy/dispatch divergence
* Concept leakage before cross-layer drift
* Cognitive load growth before fragility

This audit measures structural entropy, not quality.
