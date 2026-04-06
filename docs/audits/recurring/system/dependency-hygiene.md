# Audit: Dependency Hygiene / Feature / Publish Surface

`Cargo.toml` workspace graph plus public support crates where relevant

## Purpose

Verify that dependency structure remains:

* directional
* minimal
* intentional
* publish-safe
* feature-disciplined

This audit measures **crate dependency hygiene**, **feature-surface discipline**,
and **package/publish containment**.

It does **NOT** evaluate:

* correctness
* runtime performance
* module visibility inside files
* style
* redesign ideas unless dependency drift is severe enough that the report
  cannot describe the problem accurately without naming the corrective direction

---

## Audit Objective

Determine whether the workspace dependency graph, feature graph, and package
surface still enforce the intended architecture, or whether they have drifted
toward:

* unnecessary crate coupling
* upward or reverse dependency pressure
* public crates depending on internal crates by convenience
* test-only dependencies leaking too high in the graph
* feature-flag sprawl
* package surface mismatch between local use and published intent
* support crates quietly becoming alternate facades

This is a **dependency and package-boundary audit**, not a runtime audit.

---

## Risk Model / Structural Invariant

Primary dependency risks:

* public crates may pick up internal-only edges
* convenience dependencies freeze unnecessary contracts
* features may widen the compile graph beyond the owning responsibility
* test or audit crates may start driving runtime package structure
* publishable crates may expose or depend on items that only work inside the
  workspace
* duplicate or overlapping support crates may blur ownership

Structural invariant:

> Each published crate should depend only on the minimum crates and features
> required for its stated role, and internal-only dependencies should stay
> one-way and clearly contained.

---

## Why This Matters

Even when module structure looks clean, dependency drift can still harden the
wrong architecture:

* publishable crates become difficult to package independently
* test helpers start shaping runtime crate boundaries
* features turn into hidden global switches
* downstream users inherit more graph complexity than the API requires
* dead or redundant edges make future refactors slower and riskier

This audit exists to catch that drift before it becomes release surface.

---

## Run This Audit After

* adding a new workspace crate
* publishing or productizing an internal helper crate
* adding or removing Cargo features
* moving code between `canic-testkit`, `canic-testing-internal`, and `canic-tests`
* build/install/release tooling changes
* public facade cleanup passes
* pre-release packaging review windows

---

## Report Preamble (Required)

Every report generated from this audit must include:

* Scope
* Compared baseline report path
* Code snapshot identifier
* Method tag/version
* Comparability status
* Exclusions applied
* Notable methodology changes vs baseline

Required fields:

* **Scope**: exact crates/manifests included
* **Compared baseline report path**: prior comparable report path or `N/A`
* **Code snapshot identifier**: commit SHA, tree hash, or equivalent
* **Method tag/version**: e.g. `dependency-hygiene-v1`
* **Comparability status**:
  * `comparable`
  * `non-comparable: <reason>`
* **Exclusions applied**: explicit list, or `none`
* **Notable methodology changes vs baseline**: explicit list, or `none`

---

## Audit Rules (Mandatory)

### Evidence Standard

Every non-trivial claim MUST identify:

* manifest or crate
* dependency edge, feature, or package field
* whether the edge is normal, optional, build, dev, or workspace-inherited
* directional or publish impact

Additional rules:

* Medium, High, and Critical findings MUST be supported by inspected manifest
  context.
* `cargo tree` or grep may identify pressure candidates, but Medium+ findings
  require manual manifest confirmation.
* Publish/package findings MUST name the crate and the exact field or edge that
  creates the risk.
* Feature findings MUST identify both:
  * the defining crate
  * the enabling dependency or re-export path when relevant

### Severity Rules

* **Low**: acceptable dependency breadth or mild feature/package pressure
* **Medium**: avoidable dependency breadth, feature sprawl, or package exposure
  worth narrowing
* **High**: confirmed public/internal seam breach, publish-surface mismatch, or
  upward dependency pattern that weakens containment
* **Critical**: confirmed reverse dependency cycle, publish-blocking internal
  leak, or dependency structure that materially defeats crate ownership

### Counting + Comparability Rules

Definitions used throughout this audit:

* **Published crate**: a crate intended for packaging outside the local
  workspace
* **Internal crate**: `publish = false` or otherwise explicitly non-public
* **Runtime edge**: normal dependency used in non-test builds
* **Test-only edge**: `dev-dependency` or `#[cfg(test)]`-only crate usage
* **Support crate**: crate whose role is facade, tooling, memory/runtime
  substrate, or generic testing infrastructure
* **Package surface**: manifest fields and dependency graph that affect
  packaging or downstream consumption
* **Feature leak**: feature or optional dependency that materially widens a
  crate’s responsibility beyond its stated role

Ignore unless explicitly stated otherwise:

* lockfile-only noise without manifest consequence
* generated outputs
* local shell scripts unless they are part of published tooling
* test-only fixture crates when evaluating runtime package surface, except for
  explicit leakage checks

### Pressure vs Violation Rule

* **Pressure** = breadth, overlap, or coordination strain
* **Violation** = public/internal seam breach, reverse direction, package
  mismatch, or confirmed publish-risk edge

Do not collapse these terms in findings.

---

## Canonical Crate Map (Mandatory)

Use this map for top-level ownership and package judgment:

| Crate / Area | Responsibility |
| --- | --- |
| `crates/canic` | public facade and macro entry surface |
| `crates/canic-core` | core runtime/orchestration |
| `crates/canic-control-plane` | root/store control-plane runtime |
| `crates/canic-wasm-store` | canonical publishable `wasm_store` canister |
| `crates/canic-cdk` | curated IC CDK facade |
| `crates/canic-memory` | stable-memory/runtime helpers |
| `crates/canic-testkit` | public generic PocketIC/test infrastructure |
| `crates/canic-testing-internal` | Canic-only internal test harnesses |
| `crates/canic-tests` | integration test entrypoints |
| `canisters/**` | demo/reference canisters only |
| `crates/canic-core/test-canisters/**` | internal correctness/integration canisters |
| `crates/canic-core/audit-canisters/**` | internal audit/perf probe canisters |

Rules:

* `canic`, `canic-cdk`, `canic-memory`, and `canic-testkit` are expected public
  support crates.
* `canic-testing-internal`, `canic-tests`, `test-canisters`, and
  `audit-canisters` are not public product API.
* Demo canisters are not generic reusable infrastructure.
* Published crates must not silently depend on internal crates.

---

## Audit Checklist

### STEP 0 — Baseline Capture (Mandatory)

Capture baseline values first.

Baseline rule:

* Use the first run of the current day (`<scope>.md`) as `Previous`.
* If this is the first run of the day, mark `Previous` as `N/A`.
* For release-cycle trend analysis, `Previous` is the most recent prior
  comparable run.

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Published crates with internal runtime edges |  |  |  |
| Published crates with test-only leakage concerns |  |  |  |
| Optional features reviewed |  |  |  |
| Publish-surface mismatches |  |  |  |
| Duplicate or overlapping support seams |  |  |  |

---

### STEP 1 — Crate Dependency Direction

For each top-level crate, identify:

* normal runtime dependencies
* optional runtime dependencies
* build dependencies
* dev dependencies
* whether the crate is published or internal

Produce:

| Crate | Published? | Runtime Depends On | Optional Depends On | Build Depends On | Dev Depends On | Upward/Internal Runtime Edge Found? | Risk |
| --- | --- | --- | --- | --- | --- | --- | --- |

Rules:

* published crates depending on internal crates at runtime are violations
* internal crates depending on public support crates are usually fine
* dev-dependency edges from runtime crates into test infrastructure are pressure,
  not automatic violations, but must be called out explicitly

---

### STEP 2 — Public/Internal Seam Checks

Explicitly check:

* `canic-testkit` does not depend on `canic-testing-internal`
* published crates do not depend on `canic-tests`
* demo canisters do not depend on test or audit canisters
* support crates do not quietly become alternate facades for `canic-core`

Produce:

| Seam | Status | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- |

---

### STEP 3 — Feature Hygiene

Inspect features in public/support crates:

* feature count
* optional dependency count
* features that widen responsibility beyond crate role
* features that only exist for workspace-local testing or build quirks

Produce:

| Crate | Feature | Enables | Public/User-Facing? | Responsibility Fit | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- | --- |

Flag:

* feature aliases that exist only to tunnel internal crate structure outward
* public features that only support test or audit-only behavior
* optional dependencies that are effectively always-on in practice but still
  increase graph complexity

---

### STEP 4 — Package / Publish Surface

Inspect publish-relevant manifest hygiene:

* `publish = false`
* package metadata expectations
* path-only edges in published crates
* workspace inheritance that may hide package constraints
* examples/docs implying unsupported external use

Produce:

| Crate | Publish Intent | Package Surface Concern | Evidence | Risk |
| --- | --- | --- | --- | --- |

Flag:

* published crates that appear to rely on workspace-only topology
* internal crates accidentally looking publishable
* mismatches between crate README/docs and actual manifest/package posture

---

### STEP 5 — Redundant / Overlapping Support Seams

Check for duplicate or overlapping crate roles, especially:

* facade overlap (`canic` vs lower-level public crates)
* memory/runtime helpers appearing in more than one public support crate
* PocketIC/test helper duplication across `canic-testkit`,
  `canic-testing-internal`, and `canic-tests`
* installer/build/release tooling overlap

Produce:

| Area | Overlap Signal | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- |

Rule:

* overlap is pressure unless it creates genuine public ambiguity or a publish
  mismatch

---

### STEP 6 — Dead / Convenience Edge Review

Inspect likely convenience edges:

* dependencies present only for one narrow type or helper
* facade re-exports that could be removed without affecting intended use
* support crates brought in only to avoid a more precise lower-level import

Produce:

| Crate | Edge / Re-export | Why It Exists | Narrower Alternative? | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- |

Do not mark something as a problem merely because it is broad; evidence must
show that the edge is convenience-only or misaligned with crate purpose.

---

### STEP 7 — Dependency Risk Index

Score each category and include a short basis explanation.

Produce:

| Category | Risk Index (1-10, lower is better) | Basis |
| --- | ---: | --- |
| Runtime Dependency Direction |  |  |
| Public/Internal Seam Discipline |  |  |
| Feature Hygiene |  |  |
| Package / Publish Surface |  |  |
| Support-Crate Ownership Clarity |  |  |

Then provide:

### Overall Dependency Hygiene Risk Index (1-10, lower is better)

Rule:

* overall score must reflect the worst real dependency/package risk, not a
  polite average

Interpretation:

* `1-3` = low risk / clean dependency structure
* `4-6` = moderate risk / manageable pressure
* `7-8` = high risk / requires monitoring
* `9-10` = critical risk / dependency structure is undermining crate ownership

---

### Delta Since Baseline

Highlight only:

* new crate edges
* removed crate edges
* newly widened feature surfaces
* new publish/package concerns
* reduced or increased support-seam overlap

Produce:

| Delta Type | Crate / Edge / Feature | Previous | Current | Impact |
| --- | --- | --- | --- | --- |

---

### Verification Readout (`PASS` / `FAIL` / `BLOCKED`)

Rules:

* `PASS` = no high/critical dependency/package violations; only low/moderate pressure
* `FAIL` = any confirmed high/critical public/internal dependency breach or publish mismatch
* `BLOCKED` = insufficient manifest/repo visibility for comparable judgment

---

## Anti-Shallow Rule

Do NOT:

* praise the graph
* comment on formatting
* propose redesign without a concrete dependency/package reason
* infer medium/high findings from `cargo tree` noise alone

Every claim must name:

* crate or manifest
* dependency/feature/package field
* direction or publish impact

