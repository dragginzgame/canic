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
* workspace inheritance may hide packaging assumptions that do not hold outside
  the workspace
* default features may silently widen downstream graph surface

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
* packaging succeeds only because the whole workspace is present

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
* converting direct dependencies to workspace-inherited dependencies
* introducing proc-macro/build-script support crates
* changing default features in a public crate

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
* **Method tag/version**: e.g. `dependency-hygiene-current`
* **Comparability status**:

  * `comparable`
  * `non-comparable: <reason>`
* **Exclusions applied**: explicit list, or `none`
* **Notable methodology changes vs baseline**: explicit list, or `none`

If any workspace crates are excluded, the report must state whether that
prevents judgment on public/internal seams or publish containment.

---

## Audit Rules (Mandatory)

### Evidence Standard

Every non-trivial claim MUST identify:

* manifest or crate
* dependency edge, feature, or package field
* whether the edge is normal, optional, build, dev, proc-macro, or workspace-inherited
* directional or publish impact

Additional rules:

* Medium, High, and Critical findings MUST be supported by inspected manifest
  context.
* `cargo tree`, `cargo metadata`, or grep may identify pressure candidates, but
  Medium+ findings require manual manifest confirmation.
* Publish/package findings MUST name the crate and the exact field or edge that
  creates the risk.
* Feature findings MUST identify both:

  * the defining crate
  * the enabling dependency, default feature, or re-export path when relevant
* Claims about “minimal dependency set” must be grounded in observed edge role,
  not vague intuition that the graph looks broad.
* If a finding depends on workspace inheritance, the report must state whether
  the risk is in:

  * the leaf crate manifest
  * the workspace dependency definition
  * the combined effect

### Severity Rules

* **Low**: acceptable dependency breadth or mild feature/package pressure
* **Medium**: avoidable dependency breadth, feature sprawl, or package exposure
  worth narrowing
* **High**: confirmed public/internal seam breach, publish-surface mismatch, or
  upward dependency pattern that weakens containment
* **Critical**: confirmed reverse dependency cycle, publish-blocking internal
  leak, or dependency structure that materially defeats crate ownership

Severity calibration rules:

* `dev-dependencies` from runtime crates into test infrastructure are usually
  **pressure**, not violations, unless they shape runtime package posture or
  published features.
* broad public support dependencies are not automatically risky; they become
  risky when they widen responsibility, leak internal seams, or create publish
  mismatch.
* optional dependencies are not automatically disciplined; if they are
  effectively always-on, they still count as graph complexity.

### Counting + Comparability Rules

Definitions used throughout this audit:

* **Published crate**: a crate intended to be packaged or consumed outside the
  local workspace
* **Internal crate**: `publish = false` or otherwise explicitly non-public
* **Publishable crate**: a crate not marked `publish = false`, even if it is
  not currently released
* **Runtime edge**: normal dependency used in non-test builds
* **Optional runtime edge**: dependency behind a feature or `optional = true`
* **Build edge**: `build-dependency`
* **Test-only edge**: `dev-dependency` or `#[cfg(test)]`-only usage
* **Support crate**: crate whose role is facade, tooling, memory/runtime
  substrate, proc-macro, or generic testing infrastructure
* **Package surface**: manifest fields and dependency graph that affect
  packaging or downstream consumption
* **Feature leak**: feature or optional dependency that materially widens a
  crate’s responsibility beyond its stated role
* **Workspace-inherited edge**: dependency defined via `workspace = true` or by
  inheriting version/default-feature policy from workspace state
* **Path-only edge**: dependency whose usable resolution depends on local
  filesystem topology and does not translate cleanly to published consumption
* **Default-feature widening**: graph breadth introduced merely by depending on
  a crate without `default-features = false` where the defaults materially
  exceed the stated role

Ignore unless explicitly stated otherwise:

* lockfile-only noise without manifest consequence
* generated outputs
* local shell scripts unless they are part of published tooling
* test-only fixture crates when evaluating runtime package surface, except for
  explicit leakage checks
* transitive duplication that does not stem from manifest choices relevant to
  this audit

### Pressure vs Violation Rule

* **Pressure** = breadth, overlap, coordination strain, or packaging fragility
* **Violation** = public/internal seam breach, reverse direction, package
  mismatch, or confirmed publish-risk edge

Do not collapse these terms in findings.

Mandatory phrasing rule:

* call breadth/complexity concerns **pressure**
* call seam, publish, or direction breaches **violations**

---

## Canonical Crate Map (Mandatory)

Use this map for top-level ownership and package judgment:

| Crate / Area                           | Responsibility                              |
| -------------------------------------- | ------------------------------------------- |
| `crates/canic`                         | public facade and macro entry surface       |
| `crates/canic-core`                    | core runtime/orchestration                  |
| `crates/canic-control-plane`           | root/store control-plane runtime            |
| `crates/canic-wasm-store`              | canonical publishable `wasm_store` canister |
| `crates/canic-cdk`                     | curated IC CDK facade                       |
| `crates/canic-memory`                  | stable-memory/runtime helpers               |
| `crates/canic-testkit`                 | public generic PocketIC/test infrastructure |
| `crates/canic-testing-internal`        | Canic-only internal test harnesses          |
| `crates/canic-tests`                   | integration test entrypoints                |
| `fleets/**`                            | config-defined operator fleets              |
| `canisters/test/**`                    | internal correctness/integration fixtures   |
| `canisters/audit/**`                   | internal audit/perf probe canisters         |
| `canisters/sandbox/**`                 | manual sandbox canisters                    |

Rules:

* `canic`, `canic-cdk`, `canic-memory`, and `canic-testkit` are expected public
  support crates.
* `canic-testing-internal`, `canic-tests`, `canisters/test`, and
  `canisters/audit` are not public product API.
* Fleet canisters are not generic reusable infrastructure.
* `canisters/sandbox/**` is manual scratch space and must not become product API.
* Published crates must not silently depend on internal crates.
* Public support crates must not become alternate general-purpose facades for
  internal runtime ownership without that being explicitly stated.

---

## Scope Defaults

Default scope for this audit includes:

* workspace root manifest
* all crate manifests under `crates/**`
* fleet manifests under `fleets/**`
* internal test/audit/sandbox canister manifests under `canisters/**`

If the workspace root manifest is excluded, publish/package judgments that rely
on shared dependency policy must be marked `BLOCKED` or explicitly narrowed.

---

## Audit Checklist

### STEP 0 — Baseline Capture (Mandatory)

Capture baseline values first.

Baseline rule:

* Use the first run of the current day (`<scope>.md`) as `Previous`.
* If this is the first run of the day, mark `Previous` as `N/A`.
* For release-cycle trend analysis, `Previous` is the most recent prior
  comparable run.

Produce:

| Metric                                                                   | Previous | Current | Delta |
| ------------------------------------------------------------------------ | -------: | ------: | ----: |
| Published crates with internal runtime edges                             |          |         |       |
| Published crates with test-only leakage concerns                         |          |         |       |
| Optional features reviewed                                               |          |         |       |
| Publish-surface mismatches                                               |          |         |       |
| Duplicate or overlapping support seams                                   |          |         |       |
| Published crates with path-only or workspace-fragile package assumptions |          |         |       |
| Public crates with default-feature widening concerns                     |          |         |       |

Rules:

* deltas must compare only against a comparable baseline
* if no comparable baseline exists, state `N/A`
* do not invent trend claims when the prior method differs materially

---

### STEP 1 — Crate Dependency Direction

For each top-level crate, identify:

* normal runtime dependencies
* optional runtime dependencies
* build dependencies
* dev dependencies
* proc-macro/build-script dependencies where relevant
* whether the crate is published, publishable, or internal

Produce:

| Crate | Publish Intent | Runtime Depends On | Optional Depends On | Build Depends On | Dev Depends On | Internal Runtime Edge Found? | Reverse/Upward Pressure Found? | Risk |
| ----- | -------------- | ------------------ | ------------------- | ---------------- | -------------- | ---------------------------- | ------------------------------ | ---- |

Rules:

* published crates depending on internal crates at runtime are violations
* publishable crates with internal runtime edges are package-risk pressure even
  if not yet released
* internal crates depending on public support crates are usually fine
* dev-dependency edges from runtime crates into test infrastructure are pressure,
  not automatic violations, but must be called out explicitly
* build-dependencies count toward package surface if they rely on internal
  workspace topology or hidden support crates
* proc-macro crates must be treated as part of public package surface when used
  by published crates

---

### STEP 2 — Public/Internal Seam Checks

Explicitly check:

* `canic-testkit` does not depend on `canic-testing-internal`
* published crates do not depend on `canic-tests`
* demo canisters do not depend on test or audit canisters
* support crates do not quietly become alternate facades for `canic-core`
* public support crates do not rely on internal crates through build-dependencies
  or optional runtime features
* published crates do not inherit internal seams via workspace-defined aliases

Produce:

| Seam | Status | Evidence | Pressure or Violation | Risk |
| ---- | ------ | -------- | --------------------- | ---- |

Rules:

* if the seam is clean only because the dependency is dev-only, say so
* optional edges count if a public feature can enable them
* build-script edges must be checked separately from runtime edges

---

### STEP 3 — Feature Hygiene

Inspect features in public/support crates:

* feature count
* optional dependency count
* default feature behavior
* features that widen responsibility beyond crate role
* features that only exist for workspace-local testing or build quirks
* features that merely alias internal crate layout
* feature coupling caused by workspace inheritance or transitive defaults

Produce:

| Crate | Feature | Enables | Default? | Public/User-Facing? | Responsibility Fit | Pressure or Violation | Risk |
| ----- | ------- | ------- | -------- | ------------------- | ------------------ | --------------------- | ---- |

Flag:

* feature aliases that exist only to tunnel internal crate structure outward
* public features that only support test or audit-only behavior
* optional dependencies that are effectively always-on in practice but still
  increase graph complexity
* default features whose breadth exceeds the crate’s stated role
* features that make a published crate depend on workspace-only assumptions

Rules:

* not every feature is user-facing; say whether it is intended for downstream
  use, internal build control, or publishing ergonomics
* if a feature only exists to work around workspace-local topology, that is
  usually pressure and may be a publish violation
* when a public crate re-exports another crate’s feature surface indirectly,
  note the coupling explicitly

---

### STEP 4 — Package / Publish Surface

Inspect publish-relevant manifest hygiene:

* `publish = false`
* package metadata expectations
* path-only edges in published or publishable crates
* workspace inheritance that may hide package constraints
* `readme`, `repository`, `documentation`, `license`, and related intent signals when helpful
* examples/docs implying unsupported external use
* build scripts or proc-macro support that assume workspace-local files
* dependency declarations that only resolve because of workspace membership

Produce:

| Crate | Publish Intent | Package Surface Concern | Evidence | Pressure or Violation | Risk |
| ----- | -------------- | ----------------------- | -------- | --------------------- | ---- |

Flag:

* published crates that appear to rely on workspace-only topology
* internal crates accidentally looking publishable
* mismatches between crate README/docs and actual manifest/package posture
* path-only dependencies in publishable crates
* workspace-inherited dependencies whose publish posture is unclear outside the workspace

Rules:

* distinguish `published today` from merely `publishable`
* if a crate is intentionally internal, accidental publishability is still a
  package concern
* examples/docs only matter when they materially imply unsupported external use

---

### STEP 5 — Redundant / Overlapping Support Seams

Check for duplicate or overlapping crate roles, especially:

* facade overlap (`canic` vs lower-level public crates)
* memory/runtime helpers appearing in more than one public support crate
* PocketIC/test helper duplication across `canic-testkit`,
  `canic-testing-internal`, and `canic-tests`
* installer/build/release tooling overlap
* proc-macro or facade overlap that causes users to choose between multiple
  public entry surfaces for the same responsibility

Produce:

| Area | Overlap Signal | Evidence | Pressure or Violation | Risk |
| ---- | -------------- | -------- | --------------------- | ---- |

Rule:

* overlap is pressure unless it creates genuine public ambiguity, ownership
  confusion, or a publish mismatch

---

### STEP 6 — Dead / Convenience Edge Review

Inspect likely convenience edges:

* dependencies present only for one narrow type or helper
* facade re-exports that could be removed without affecting intended use
* support crates brought in only to avoid a more precise lower-level import
* edges retained after prior refactors that no longer serve the current role
* build dependencies retained only for workspace convenience

Produce:

| Crate | Edge / Re-export | Why It Exists | Narrower Alternative? | Pressure or Violation | Risk |
| ----- | ---------------- | ------------- | --------------------- | --------------------- | ---- |

Rules:

* do not mark something as a problem merely because it is broad
* evidence must show that the edge is convenience-only, stale, or misaligned
  with crate purpose
* “could theoretically be split” is insufficient

---

### STEP 7 — Feature / Package Pressure Indicators

This step is for graph breadth that is not yet a violation.

Explicitly identify:

* public crates with 5+ runtime dependencies from sibling workspace crates
* public crates with 3+ optional dependencies
* public crates with multiple feature gates that do not map cleanly to the
  crate’s stated role
* support crates whose package posture depends heavily on workspace inheritance
* crates whose build script, macro, or examples materially widen downstream
  compile surface

Produce:

| Crate / Area | Pressure Type | Why This Is Pressure (Not Yet Violation) | Drift Sensitivity | Risk |
| ------------ | ------------- | ---------------------------------------- | ----------------- | ---- |

Rule:

* explain why the issue is not yet a seam or publish violation

---

### STEP 8 — Dependency Risk Index

Score each category and include a short basis explanation.

Produce:

| Category                        | Risk Index (1-10, lower is better) | Basis |
| ------------------------------- | ---------------------------------: | ----- |
| Runtime Dependency Direction    |                                    |       |
| Public/Internal Seam Discipline |                                    |       |
| Feature Hygiene                 |                                    |       |
| Package / Publish Surface       |                                    |       |
| Support-Crate Ownership Clarity |                                    |       |

Then provide:

## Overall Dependency Hygiene Risk Index (1-10, lower is better)

Rule:

* overall score must reflect the worst real dependency/package risk, not a
  polite average
* a confirmed High seam or publish violation should usually push overall risk to
  at least `7`
* a Critical publish-blocking or internal-leak condition should usually push
  overall risk to at least `9`

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
* new default-feature widening
* new workspace-inheritance fragility affecting publish posture

Produce:

| Delta Type | Crate / Edge / Feature | Previous | Current | Impact |
| ---------- | ---------------------- | -------- | ------- | ------ |

Rules:

* delta must compare only against a comparable baseline
* if baseline is absent or non-comparable, use `N/A`
* do not make directional claims from incomplete scope

---

### Verification Readout (`PASS` / `FAIL` / `BLOCKED`)

Rules:

* `PASS` = no High/Critical dependency or package violations; only Low/Medium pressure
* `FAIL` = any confirmed High/Critical public/internal dependency breach, feature leak, or publish mismatch
* `BLOCKED` = insufficient manifest/repo visibility for comparable judgment

If `BLOCKED`, the report must name the missing scope or manifest evidence.

---

## Required Method Discipline

At minimum, the auditor must:

1. inspect the workspace root manifest
2. inspect all public/support crate manifests in scope
3. confirm runtime vs optional vs build vs dev edges manually for Medium+ findings
4. inspect feature declarations and default features for public/support crates
5. inspect publish-relevant fields and path/workspace inheritance posture
6. check public/internal seams explicitly
7. separate pressure from violation in every non-trivial finding
8. compare only against a comparable baseline

---

## Anti-Shallow Rule

Do NOT:

* praise the graph
* comment on formatting
* propose redesign without a concrete dependency/package reason
* infer Medium/High findings from `cargo tree` noise alone
* call broad public support edges violations unless they actually breach seam,
  publish, or responsibility rules

Every claim must name:

* crate or manifest
* dependency/feature/package field
* edge type
* direction or publish impact

---

## Auditor Output Contract

The report must make it easy for a reviewer to answer, for every finding:

* which crate is affected
* what edge, feature, or package field is involved
* whether the issue is runtime, optional, build, dev, or publish-related
* why it is pressure or a violation
* whether it changed since baseline

If the report cannot support those questions, the run should be marked `BLOCKED`
rather than overstated.
