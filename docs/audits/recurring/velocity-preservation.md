# WEEKLY AUDIT — Velocity Preservation

`canic-core`

## Purpose

Evaluate whether the current architecture still supports:

* Rapid feature iteration
* Contained feature changes
* Low cross-layer amplification
* Predictable extension

This is NOT:

* A correctness audit
* A DRY audit
* A style audit
* A redesign proposal exercise

This audit measures structural feature agility.

---

# Core Principle

Low-risk velocity architecture has:

* Contained change surfaces
* Stable layer boundaries
* Low cross-cutting amplification
* Clear ownership per subsystem
* Predictable growth vectors

Velocity degrades when:

* Features require edits across `endpoints -> workflow -> policy -> ops -> model/storage`
* Capability/auth/replay concerns are tightly coupled
* Modules become gravity wells
* A single enum addition multiplies branch count across services and layers

---

# Canonical Subsystem Map (Mandatory)

Use this subsystem map for all counting in this audit:

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

* Every file must be attributed to exactly one subsystem.
* If a file spans multiple domains, classify by primary ownership.
* Count each subsystem once per feature slice, even if many files are touched.

---

# Layer Model (Mandatory)

Use this fixed layer model for CAF and change-surface calculations:

1. `endpoints` (RPC entrypoints, macros)
2. `workflow` (request orchestration)
3. `policy` (authorization and decision logic)
4. `ops` (side effects and system calls)
5. `model/storage` (state and projections)

Rule:

* `layer_count` is the number of distinct layers touched by a feature slice from this list only.

---

# Flow Axis Rule (Mandatory)

A flow axis is any condition that alters control flow across layers.

Common flow axes:

* capability proof mode
* replay state
* caller topology relation
* lifecycle phase (`init`, `post_upgrade`, timer runtime)
* role/subnet context
* request type
* funding source

Rules:

* Count each axis once per feature slice.
* Do not duplicate axis counts for repeated branches of the same axis.

---

# CAF Measurement Rule (Mandatory)

For each feature slice:

* `revised_caf = max(subsystems, layers) x flow_axes`
* `subsystems` must use the canonical subsystem map above
* `layers` must use the fixed layer model above
* `flow_axes` must follow the flow axis rule above

---

# Evidence Requirement (Mandatory)

Every risk claim must include:

* module name
* file count or LOC evidence
* subsystem count
* dependency direction or boundary crossing evidence

---

# STEP 0 — Baseline Capture (Mandatory)

Capture baseline values first.

Baseline rule:

* Use the first run of the current day (`<scope>.md`) as `Previous`.
* If this is the first run of the day, mark `Previous` as `N/A` and treat this run as baseline.
* Do not compare reruns against other reruns on the same day.

| Metric | Previous | Current | Delta |
| ---- | ----: | ----: | ----: |
| Velocity Risk Index |  |  |  |
| Cross-layer leakage crossings |  |  |  |
| Avg files touched per feature slice |  |  |  |
| p95 files touched |  |  |  |
| Top gravity-well fan-in |  |  |  |

---

# STEP 1 — Change Surface Mapping (Empirical, Revised CAF)

Analyze the last 3–5 major feature slices.

| Feature | Files Modified | Subsystems | Layers | Flow Axes | Revised CAF | ELS | Feature Locality Index | Containment Score | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ----: | ----: | ---- |

Definitions:

* `ELS (Extension Locality Score) = primary_subsystem_files / total_files_modified`
* `feature_locality_index = files_in_primary_module / total_files_modified`
* `containment_score = subsystems_modified / total_subsystems`

Interpretation:

* `ELS`: `>0.70` good, `0.40-0.70` moderate, `<0.40` poor
* `feature_locality_index`: `>0.70` localized, `0.40-0.70` distributed, `<0.40` cross-system
* `containment_score`: `<=0.30` strongly contained, `0.30-0.60` moderate, `>0.60` cross-system

Flag:

* Revised CAF trend up week-over-week
* Low ELS or low feature locality on routine slices
* High containment on routine slices

---

# STEP 2 — Edit Blast Radius (Empirical)

Use feature slices in the current audit window (or PR history when available).

| Metric | Current | Previous | Delta |
| ---- | ----: | ----: | ----: |
| average files touched per feature slice |  |  |  |
| median files touched |  |  |  |
| p95 files touched |  |  |  |

If PR-level history is unavailable locally, compute from audited feature slices and mark as `slice-sampled`.

---

# STEP 3 — Boundary Leakage (Mechanical)

Track import and type-reference crossings with explicit rules.

Required checks:

* endpoints/macros -> model/storage direct references
* workflow -> model/storage direct references
* policy -> dto/ops/runtime references
* ops -> policy-style decision ownership growth
* auth/capability DTOs leaking into model/storage ownership

| Boundary | Import Crossings | Previous | Delta | Risk |
| ---- | ----: | ----: | ----: | ---- |

This step must be regex/mechanical first, then manually triaged.

---

# STEP 4 — Change Multiplier Matrix (Deterministic)

Map feature axes to subsystems, then compute deterministic multiplier.

| Feature Axis | Endpoints | Workflow | Policy | Ops | Model/Storage | Subsystem Count |
| ---- | ---- | ---- | ---- | ---- | ---- | ----: |

`subsystem_count = number of checked cells`

Then summarize:

| Candidate Feature | Axes Involved | Subsystem Count | Friction |
| ---- | ---- | ----: | ---- |

---

# STEP 5 — Enum Shock Radius (Density-Adjusted)

Track enum expansion velocity impact.

| Enum | Variants | Switch Sites | Modules Using Enum | Switch Density | Subsystems | Shock Radius | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- |

Definitions:

* `switch_density = switch_sites / modules_using_enum`
* `shock_radius = variants x switch_density x subsystems`

Track high-impact enums (examples):

* `dto::rpc::Request`
* `dto::rpc::Response`
* `dto::capability::CapabilityProof`
* `access::expr::BuiltinPredicate`
* root capability/replay workflow enums

High-risk enum rule:

* If `switch_density > 3` and `subsystems >= 4`, mark enum as `structural hotspot`.

---

# STEP 6 — Gravity Well Growth Rate

Identify gravity-well modules using growth-rate signals.

| Module | LOC | LOC Delta | Fan-In | Fan-In Delta | Domains | Edit Frequency (30d) | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- |

Gravity-well condition:

* `LOC delta > 2x weekly average` and `fan-in delta > 1`

Escalation condition:

* high fan-in and high edit frequency

Domain count categories:

* auth/attestation/delegation
* capability/replay
* rpc/request dispatch
* policy/topology decisions
* storage/projection adapters
* lifecycle/runtime timers

---

# STEP 7 — Subsystem Independence Score (Size-Adjusted)

Measure subsystem self-sufficiency with small-module noise suppression.

| Subsystem | Internal Imports | External Imports | LOC | Independence | Adjusted Independence | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ---- |

Definitions:

* `independence = internal / (internal + external)`
* `adjusted_independence = independence x log(module_loc)`

Low adjusted independence means feature work is coupling-driven in materially sized subsystems.

---

# STEP 8 — Decision-Axis Growth (Independence-Aware)

Track axis growth for core operations.

| Operation | Axes | Axis Count | Independent Axes | Previous Independent Axes | Delta | Risk |
| ---- | ---- | ----: | ----: | ----: | ----: | ---- |

Risk should be driven by `independent_axes`, not raw axis count.

---

# STEP 9 — Decision Surface Size

Track where behavior is actually decided for key enums.

| Enum | Decision Sites | Previous | Delta | Risk |
| ---- | ----: | ----: | ----: | ---- |

`decision_sites = match/if decision points over that enum`

This is an early warning for branch growth before variant growth.

---

# STEP 10 — Refactor Noise Filter

Before finalizing risk, classify transient spikes.

Rules:

* If module split increases file count but reduces fan-in, mark `structural improvement`.
* If change surface grows while revised CAF and shock radius are flat/down, mark `refactor transient`.
* Mark as structural improvement when all hold:
  * file count increases
  * fan-in decreases
  * decision sites decrease

| Signal | Raw Trend | Noise Classification | Adjusted Interpretation |
| ---- | ---- | ---- | ---- |

---

# STEP 11 — Velocity Risk Index (Semi-Mechanical)

Score each bucket (1–10), then apply weighted aggregate:

* enum shock radius x3
* CAF trend x2
* cross-layer leakage x2
* gravity-well growth x2
* edit blast radius x1

| Area | Score | Weight | Weighted Score |
| ---- | ----: | ----: | ----: |

`overall_index = weighted_sum / weight_sum`

Coupling regression rule:

* Escalate final risk level by one tier if at least two increased week-over-week:
  * CAF
  * enum shock radius
  * edit blast radius
  * gravity-well fan-in

Interpretation:

* 1–3 = Low risk / structurally healthy
* 4–6 = Moderate risk / manageable pressure
* 7–8 = High risk / requires monitoring
* 9–10 = Critical risk / structural instability

---

# STEP 12 — Structural Drift Check

Detect slow architectural drift over time.

| Signal | Previous | Current | Delta | Risk |
| ---- | ----: | ----: | ----: | ---- |
| subsystem fan-in concentration |  |  |  |  |
| top 3 modules LOC share |  |  |  |  |
| cross-subsystem imports |  |  |  |  |
| policy-layer decision ownership |  |  |  |  |

Flag if:

* top module LOC share `> 12%`
* subsystem fan-in concentration increases week-over-week
* policy decisions migrate into ops or workflow

---

# STEP 13 — Synthetic Feature Simulation

Simulate extension pressure for:

1. new capability proof mode
2. new RPC request variant
3. new policy rule

| Synthetic Feature | Files Touched | Subsystems | Layers | Risk |
| ---- | ----: | ----: | ----: | ---- |

Purpose:

* predict hidden coupling before it appears in shipped features

---

# Final Output

1. Velocity Risk Index (1–10, lower is better)
2. Revised CAF + ELS + Feature Locality + Containment summary
3. Edit Blast Radius Summary
4. Boundary Leakage Trend Table
5. Change Multiplier Matrix
6. Enum Shock Radius Hotspots (including structural hotspots)
7. Gravity-Well Growth + Edit Frequency Table
8. Subsystem Independence Scores
9. Independent-Axis Growth Warnings
10. Decision Surface Size Trends
11. Refactor-Transient vs True-Drag Findings
12. Structural Drift Table
13. Synthetic Feature Simulation Table

---

# Anti-Shallow Rule

Do NOT say:

* "Seems modular"
* "Looks maintainable"
* "Separation is clear"

Every claim must include concrete evidence from the Evidence Requirement section.

---

# Why This Audit Matters

Velocity audits measure whether the system still bends without breaking when features are added.

That is architectural longevity.
