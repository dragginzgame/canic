# Audit: Module Structure / Visibility Discipline

`canic-core` plus facade/support crates where relevant

## Purpose

Verify that architectural boundaries remain:

* layered
* directional
* encapsulated
* narrowly exposed
* intentionally public

This audit measures **structural containment** and **visibility discipline**.

It does **NOT** evaluate:

* correctness
* performance
* features
* coding style
* refactoring opportunities unless a structural violation is severe enough that containment cannot be described accurately without naming the corrective direction

---

## Audit Objective

Determine whether crate topology, module topology, and visibility scopes still enforce the intended architecture, or whether they have drifted toward:

* unstable public commitments
* widening internal coordination surfaces
* cross-responsibility coupling
* facade leakage
* demo/test/audit seam erosion
* hub concentration that increases future drift risk

This is a **structure-and-containment audit**, not a correctness audit.

---

## Risk Model / Structural Invariant

Primary structural risks:

* `pub` exposure creates durable external commitments
* wide `pub(crate)` surfaces increase internal coordination cost
* broad `mod.rs` or root modules become gravity wells for unrelated work
* public/internal seam drift weakens demo, test, and audit containment
* upward or cross-responsibility dependencies reduce architectural clarity
* facade crates may re-export implementation details by convenience
* test or audit support may leak into runtime or demo/reference surfaces

Structural invariant:

> Each crate and subsystem should expose only the minimum surface required for its intended role, and dependencies should flow downward through the declared layer model or across approved data-support seams only.

---

## Why This Matters

Layering can appear technically clean while topology and visibility hygiene still drift into a shape where:

* internal helpers become de facto public API
* convenience re-exports freeze unstable implementation details
* test or audit helpers widen runtime blast radius
* a few coordination hubs attract unrelated responsibilities
* public contracts and implementation representations start to blur

This audit exists to catch that drift before it hardens.

---

## Run This Audit After

* facade or crate-topology changes
* public API cleanup passes
* large module splits or consolidations
* demo/test/audit boundary changes
* pre-release architecture review windows
* introduction of a new support crate
* major `pub use` additions at crate roots
* movement of files across top-level subsystem roots

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

* **Scope**: exact crates/directories included in this run
* **Compared baseline report path**: prior comparable report path or `N/A`
* **Code snapshot identifier**: commit SHA, tree hash, or equivalent immutable snapshot reference
* **Method tag/version**: e.g. `module-structure-current`
* **Comparability status**:

  * `comparable`
  * `non-comparable: <reason>`
* **Exclusions applied**: explicit list, or `none`
* **Notable methodology changes vs baseline**: explicit list, or `none`

---

## Audit Rules (Mandatory)

### Evidence Standard

Every non-trivial claim MUST identify:

* module or file
* dependency or exposed item
* visibility scope (`pub`, `pub(crate)`, `pub(super)`, private)
* directional or exposure impact

Additional evidence rules:

* Medium, High, and Critical findings MUST be supported by inspected file or module context.
* Symbol counts, grep hits, or import frequency may identify **pressure candidates**, but are not sufficient by themselves for Medium+ violations.
* Re-export findings MUST name both:

  * the re-exporting path
  * the original owning path
* Public reachability claims MUST be based on actual root-reachable paths, not local `pub` visibility alone.
* Claims about narrowest plausible visibility MUST be grounded in observed use sites or clearly bounded call graph context.

### Severity Rules

* **Low**: acceptable structure, minor exposure breadth, or mild pressure worth monitoring
* **Medium**: avoidable boundary pressure, unnecessary exposure, or local layering strain without confirmed architectural breach
* **High**: confirmed architectural violation, unstable exposure of implementation-owned surface, or meaningful seam breach
* **Critical**: confirmed cross-layer breach, public leak of internals that materially harms containment, or real crate/subsystem cycle

Severity calibration rules:

* Same-layer coupling is not automatically a violation.
* Pressure alone must not be escalated to High unless directional or exposure rules are actually breached.
* A public item is not risky merely because it is broad; it must also be unstable, implementation-owned, or misaligned with stated crate responsibility.

### Counting + Comparability Rules

Definitions used throughout this audit:

* **Publicly reachable from root**: reachable from crate root by public path via any combination of:

  * `pub mod`
  * `pub use`
  * public nested items under publicly reachable modules
* **Public item**: an item declared `pub`, whether or not root-reachable
* **Externally reachable item**: a public item that is root-reachable from a published/public-facing crate
* **Subsystem dependency**: dependency across top-level subsystem roots, not intra-subsystem references
* **Cross-layer dependency**: dependency on a subsystem outside the expected responsibility layer of the depending subsystem
* **Cycle**: real subsystem-level or crate-level mutual dependency or back-reference, not two subsystems sharing a lower utility
* **Hub module**: module with either:

  * imports from 5 or more sibling subsystem roots, or
  * materially mixed coordination across facade/runtime/test seams

Ignore unless explicitly stated otherwise:

* code behind `#[cfg(test)]`
* test files and benches
* generated artifacts not committed as runtime API surface
* dev-only examples and scripts outside audited scope
* macro expansion internals, except where macro-export surface itself is public

Treat macros separately from runtime API.

### Pressure vs Violation Rule

* **Pressure** = breadth, coordination load, or containment strain
* **Violation** = directional breach, public/internal seam leak, real cycle, or confirmed unstable exposure

Do not collapse these terms in findings.

Mandatory phrasing rule:

* If something is only a breadth/cohesion concern, call it **pressure**
* If it breaks a direction, seam, or exposure rule, call it a **violation**

---

## Canonical Crate Topology (Mandatory)

Use this crate map for top-level ownership and boundary assessment:

| Crate / Area                           | Responsibility                               |
| -------------------------------------- | -------------------------------------------- |
| `crates/canic`                         | public facade and macro entry surface        |
| `crates/canic-core`                    | core runtime, orchestration, and shared DTOs |
| `crates/canic-control-plane`           | root/store control-plane runtime support     |
| `crates/canic-wasm-store`              | canonical publishable `wasm_store` canister  |
| `crates/canic-cdk`                     | curated IC CDK facade                        |
| `crates/canic-memory`                  | stable-memory/runtime helpers                |
| `crates/canic-testkit`                 | public generic PocketIC/test infrastructure  |
| `crates/canic-testing-internal`        | Canic-only internal test harnesses           |
| `crates/canic-tests`                   | integration test entrypoints                 |
| `canisters/**`                         | demo/reference canisters only                |
| `crates/canic-core/test-canisters/**`  | internal correctness/integration canisters   |
| `crates/canic-core/audit-canisters/**` | internal audit/perf probe canisters          |

Rules:

* `canic` is the only intended broad public facade unless a crate is explicitly designed as standalone public infrastructure (`canic-testkit`, `canic-cdk`, `canic-memory`).
* `canic-testing-internal`, `test-canisters`, and `audit-canisters` are not public product API.
* `canisters/**` is demo/reference surface, not generic test or audit plumbing.
* `canic-tests` is a consumer/test entry layer, not reusable runtime infrastructure.
* Public support crates must not silently become alternate facades for `canic-core`.

---

## Canonical Core Subsystem Map (Mandatory)

For `canic-core`, classify files by these top-level subsystem roots:

| Subsystem  | Path Scope               |
| ---------- | ------------------------ |
| facade-api | `api/**`                 |
| workflow   | `workflow/**`            |
| policy     | `policy/**`, `access/**` |
| ops        | `ops/**`                 |
| storage    | `storage/**`             |
| dto        | `dto/**`                 |
| ids        | `ids/**`                 |
| config     | `config/**`              |
| infra      | `infra/**`               |

Rules:

* Every non-generated runtime file must be assigned to exactly one subsystem.
* Ambiguous files must be assigned to the nearest owning subsystem and the ambiguity must be noted.
* `dto` and `ids` are transfer/value layers, not decision owners.
* `config` is support/data configuration, not policy or orchestration.
* If a top-level path does not fit this map, it must be explicitly classified in the report before dependency judgment is made.

---

## Layer Model (Mandatory)

Use this fixed responsibility stack when evaluating directionality:

### Primary stack

1. `facade / endpoints / macros`
2. `workflow`
3. `policy`
4. `ops`
5. `storage / model`
6. `infra / platform utilities`

### Data-support layers

1. `dto`
2. `ids`
3. `config`

Direction rules:

* downward dependencies are acceptable
* support-layer dependencies (`dto`, `ids`, `config`) are acceptable when they remain data/support only
* same-layer dependencies are pressure, not automatic violation
* upward dependencies are violations unless clearly facade-only wrapping with no behavioral coupling
* lower layers must not encode behavior or decisions owned by higher layers
* `dto` must not become execution owner or coordination owner
* `ids` must not accumulate policy or workflow behavior

Interpretation rules:

* storage depending on infra is normal
* ops depending on storage is normal
* policy depending on storage helpers is suspicious and requires context
* workflow bypassing ops into storage internals is usually a violation
* facade crates re-exporting core implementation types is a containment leak unless explicitly stable by contract

---

## Scope Defaults

Default audit scope:

* `crates/canic`
* `crates/canic-core`
* `crates/canic-control-plane`
* `crates/canic-wasm-store`
* `crates/canic-cdk`
* `crates/canic-memory`
* `crates/canic-testkit`
* `crates/canic-testing-internal`
* `crates/canic-tests`
* `canisters/**`
* `crates/canic-core/test-canisters/**`
* `crates/canic-core/audit-canisters/**`

If a run excludes any of these, that must appear in the preamble and the verification outcome may become `BLOCKED` if the missing scope affects seam judgment.

---

# STEP 1 - Public Surface Mapping

## 1A. Crate Root Enumeration

Enumerate for each public-facing crate:

* all `pub mod` at crate root
* all `pub use` re-exports
* all publicly reachable `pub struct`, `pub enum`, `pub trait`
* all publicly reachable `pub fn`
* all publicly reachable `pub type`
* all publicly reachable constants/statics when part of public API
* all publicly reachable macros, tracked separately from runtime API

Public-facing crates to scan by default:

* `crates/canic`
* `crates/canic-testkit`
* `crates/canic-cdk`
* `crates/canic-memory`
* any other crate intentionally published or externally consumed in this run

For each item, record:

| Item | Kind | Path | Publicly Reachable From Root? | Classification | Visibility Scope | Exposure Impact | Risk |
| ---- | ---- | ---- | ----------------------------- | -------------- | ---------------- | --------------- | ---- |

Additional rules:

* Distinguish `declared pub` from `root-reachable`.
* Re-exports must be listed even if the original item is defined elsewhere.
* Macros must be separated from runtime API surface.
* Items behind feature gates must be marked as such if materially relevant to comparability.

## 1B. Exposure Classification

For each public item, classify as one of:

* intended external API
* facade-support item
* macro-support item
* internal plumbing exposed for convenience
* accidentally exposed
* unclear / requires judgment

Exposure scan must explicitly check:

* executor or dispatcher internals
* replay or recovery machinery
* raw storage types
* `__internal`-style namespaces or equivalents
* internal diagnostics or test helpers
* unstable implementation wiring types
* builder/constructor APIs that smuggle implementation-owned types into public contracts

Guidance:

* convenience overexposure is usually **pressure**
* exposure of implementation-owned or unstable types is a **violation**
* a stable DTO is not an accidental exposure merely because it is broad
* public generic helpers in `canic-testkit`, `canic-cdk`, or `canic-memory` are acceptable when aligned to stated crate responsibility

## 1C. Public Field Exposure

Scan for:

* `pub struct` with `pub` fields
* public enums exposing representation-heavy or internal variants
* public types exposing `Raw*`, storage-entry, replay, commit, recovery, or executor-owned representations
* public constructors requiring internal representation types
* public newtypes or aliases that accidentally expose internal storage/model identities

Produce:

| Type | Public Fields? | Representation Leakage? | Stable DTO/Facade Contract? | Exposure Impact | Risk |
| ---- | -------------- | ----------------------- | --------------------------- | --------------- | ---- |

Rule:

* do not mark a public field or type as risky when it is clearly part of a stable DTO or facade contract
* do mark it risky if its representation is implementation-owned, persistence-owned, replay-owned, or recovery-owned without explicit API intent

---

# STEP 2 - Subsystem Boundary Mapping

Evaluate:

* crate-to-crate direction
* `canic-core` subsystem direction
* public/internal seam direction (`canic-testkit` vs `canic-testing-internal`)
* demo/test/audit canister ownership
* re-export ownership and whether re-exports preserve or blur responsibility

## 2A. Dependency Direction

For each crate or subsystem, identify:

* what it imports from
* what imports it
* whether those dependencies are lower-layer, same-layer, upward, or cross-seam

Produce:

| Subsystem / Crate | Depends On | Depended On By | Lower-Layer Dependencies | Same-Layer Dependencies | Upward Dependency Found? | Direction Assessment (Pressure/Violation) | Risk |
| ----------------- | ---------- | -------------- | -----------------------: | ----------------------: | ------------------------ | ----------------------------------------- | ---- |

Minimum checks:

* `canic` must not depend upward on testing crates
* `canic-testkit` must not depend on `canic-testing-internal`
* demo canisters must not depend on audit canisters
* audit canisters must not become demo/reference dependencies
* `workflow` must not reach into storage internals directly
* `policy` must not depend on ops, workflow, or runtime side effects
* `storage` must not depend upward on `ops`, `policy`, or `workflow`
* support crates must not depend on demo/reference canisters

Judgment rules:

* imports through stable facades count differently from imports into implementation internals
* same-layer imports should be discussed as pressure unless they create a seam breach
* dependency counts alone are insufficient; the report must name the imported symbols or module ownership when claiming violation

## 2B. Circular Dependency Check

Report only real subsystem-level or crate-level mutual dependency patterns.

Produce:

| Subsystem A | Subsystem B | Real Cycle? | Evidence | Risk |
| ----------- | ----------- | ----------- | -------- | ---- |

Rules:

* shared dependency on a lower utility is not a cycle
* bidirectional dependency through re-export also counts if it materially couples ownership
* if a cycle is conditional or feature-gated, note that explicitly

## 2C. Implementation Leakage

Explicitly check:

* planner/policy-like files referencing executor or transport internals
* workflow referencing storage implementation details instead of ops facades
* testkit exposing Canic-only harness concepts
* internal test harness depending on demo canister-only assumptions where a test or audit canister should own the behavior
* facade crates exposing implementation-owned core internals
* demo/reference crates re-exporting audit/test-only helpers
* public constructors or traits in facade crates that require internal core types

Produce:

| Violation | Location | Dependency | Description | Directional Impact | Risk |
| --------- | -------- | ---------- | ----------- | ------------------ | ---- |

Evidence requirement:

* Medium/High/Critical leakage findings must cite the inspected owning module context, not only a re-export line

---

# STEP 3 - Visibility Hygiene Audit

Evaluate usage of:

* `pub`
* `pub(crate)`
* `pub(super)`
* private (default)

## 3A. Overexposure

Identify:

* `pub` items that appear crate-internal only
* `pub(crate)` helpers used only in one module or narrow parent chain
* helper constructors or accessors wider than their observed call graph requires
* modules made public only for tests or convenience imports
* root-level `pub use` flattening that widens intended ownership boundaries

Produce:

| Item | Path | Current Visibility | Narrowest Plausible Visibility | Why Narrower Seems Valid | Risk |
| ---- | ---- | ------------------ | ------------------------------ | ------------------------ | ---- |

Rules:

* do not recommend narrowing unless observed usage supports it
* “could theoretically be narrower” is insufficient
* if visibility is broad because of macro expansion, generated bindings, or deliberate facade ergonomics, say so explicitly

## 3B. Under-Containment Signals

Explicitly detect:

* deep internal helpers used across multiple subsystems
* utility modules acting as unofficial cross-layer bridges
* large modules with unusually broad `pub(crate)` surface
* `mod.rs` files acting as large public or crate-internal coordination hubs
* convenience prelude-like modules that flatten ownership boundaries
* shared internal helper modules imported from unrelated responsibility roots

Produce:

| Area | Signal | Evidence | Pressure or Violation | Risk |
| ---- | ------ | -------- | --------------------- | ---- |

Rules:

* a broad internal surface is usually pressure, not violation
* it becomes a violation if it creates upward dependency, seam breach, or effective public leak

## 3C. Test Leakage

Check:

* test-only modules or helpers exposed outside `#[cfg(test)]`
* runtime modules importing test utilities
* test helper re-exports leaking into non-test builds
* audit-only helpers leaking into demo/reference crates
* `canic-testing-internal` concepts surfacing through `canic-testkit`
* test canister support re-used by runtime/demo code

Produce:

| Item | Location | Leakage Type | Build Impact | Risk |
| ---- | -------- | ------------ | ------------ | ---- |

Rules:

* distinguish harmless test organization from actual non-test build leakage
* leakage requires non-test reachability or non-test dependency impact

---

# STEP 4 - Layering Integrity Validation

## 4A. No Upward References

Explicitly test:

* `storage` does not depend on `workflow`
* `ops` does not depend on `workflow`
* `policy` does not depend on `ops` or runtime side effects
* lower layers do not encode endpoint or facade policy
* `canic-testkit` does not encode Canic-internal runtime semantics
* data-support layers do not accumulate orchestration or side-effect behavior

Produce:

| Layer / Rule | Upward Dependency Found? | Description | Risk |
| ------------ | ------------------------ | ----------- | ---- |

Rule:

* upward dependency means actual behavioral or ownership dependency, not mere mention in docs/comments or type names

## 4B. Workflow / Policy / Ops Separation

Explicitly validate:

* `policy` decides but does not act
* `workflow` orchestrates but does not own storage schema
* `ops` performs bounded execution steps but does not embed business policy
* `dto` remains transfer-only and does not act as execution owner
* `ids` remains identity/value support and does not accumulate workflow or policy behavior

Produce:

| Separation Rule | Breach Found? | Evidence | Risk |
| --------------- | ------------- | -------- | ---- |

Rules:

* helper validation logic inside DTOs is not automatically a breach
* multi-step orchestration in ops is suspicious and must be inspected
* policy files that perform I/O, write state, or select runtime side effects are breaches

## 4C. Facade Containment

Explicitly validate:

* facade crates do not re-export core internals accidentally
* `canic` does not expose storage-owned or replay-owned representation types
* `canic-testkit` exposes generic PocketIC/test helpers but not Canic-only root harness internals
* demo canisters do not re-absorb test or audit helper surface
* `canic-cdk` and `canic-memory` remain aligned to their support roles and do not become alternate runtime façades

Produce:

| Facade Item | Leak Type | Exposure Impact | Risk |
| ----------- | --------- | --------------- | ---- |

---

# STEP 5 - Structural Pressure Indicators

Pressure is not automatic violation unless directional breach, leak, or cycle evidence is present.

Explicitly identify:

* crates importing 5+ sibling crates or subsystem roots
* high-coordination hub modules
* modules spanning facade + runtime + test concerns
* enums, traits, or utility seams spanning multiple conceptual layers
* low-level helper crates becoming de facto coordination centers
* root modules or preludes flattening too many ownership boundaries

Produce:

| Area | Pressure Type | Why This Is Pressure (Not Yet Violation) | Drift Sensitivity | Risk |
| ---- | ------------- | ---------------------------------------- | ----------------- | ---- |

Rules:

* pressure findings must explain why they are not yet violations
* import breadth alone is not enough; explain the coordination consequence

## 5A. Hub Import Pressure (Required Metric)

For each high-coordination hub module or seam, include:

* `crates/canic-core/src/access/expr/mod.rs` (or equivalent root if moved)
* `crates/canic-testkit/src/pic/mod.rs`
* `crates/canic-testing-internal/src/pic/mod.rs`
* any other current high-fan-in module surfaced by the audit

Required for each hub:

1. top imported sibling subsystems by imported symbol count
2. unique sibling subsystem count
3. cross-layer dependency count
4. delta vs previous report
5. HIP calculation

Produce:

| Hub Module | Top Imported Sibling Subsystems (by Symbol Count) | Unique Sibling Subsystems Imported | Cross-Layer Dependency Count | Delta vs Previous Report | HIP | Pressure Band | Risk |
| ---------- | ------------------------------------------------- | ---------------------------------: | ---------------------------: | ------------------------ | --: | ------------- | ---- |

Formula:

`HIP = cross_layer_dependency_count / max(1, total_unique_imported_subsystems)`

Interpretation bands:

* `< 0.30`: low pressure
* `0.30 - 0.60`: moderate pressure
* `> 0.60`: high pressure

Rules:

* count only imported sibling subsystems actually referenced in code
* exclude standard library and third-party crates from HIP
* delta must compare like-for-like module paths where possible
* if a module moved, state whether delta is path-adjusted or non-comparable
* if counts increased, include one sentence explaining the increase based on observed code movement

---

# STEP 6 - Encapsulation Risk Index

Score each category and include a short basis explanation.

Produce:

| Category                  | Risk Index (1-10, lower is better) | Basis |
| ------------------------- | ---------------------------------: | ----- |
| Public Surface Discipline |                                    |       |
| Layer Directionality      |                                    |       |
| Circularity Safety        |                                    |       |
| Visibility Hygiene        |                                    |       |
| Facade Containment        |                                    |       |

Then provide:

## Overall Structural Risk Index (1-10, lower is better)

Scoring rule:

* overall score must reflect the **worst confirmed boundary condition**, not a polite average
* a single High violation should normally push overall risk to at least `7`
* a Critical cycle or public-internal leak should normally push overall risk to at least `9`

Interpretation:

* `1-3` = low risk / structurally healthy
* `4-6` = moderate risk / manageable pressure
* `7-8` = high risk / requires monitoring
* `9-10` = critical risk / structural instability

---

# STEP 7 - Drift Sensitivity Analysis

Include only growth vectors supported by observed structure.

Examples to test:

* new public root capability surface
* new testkit helper promoted to public API
* new demo canister role
* new audit canister category
* new auth DTO or proof shape reaching facade crates
* new internal helper crate or crate split
* new root prelude/re-export layer
* new support crate becoming coordination center

Produce:

| Growth Vector | Affected Subsystems | Why Multiple Layers Would Change | Drift Risk |
| ------------- | ------------------- | -------------------------------- | ---------- |

Rules:

* do not speculate beyond observed structural signals
* each vector must name the actual pressure seam that would amplify the drift

---

## Known Intentional Exceptions

Purpose: prevent relitigating deliberate structural choices every run.

Typical Canic examples:

* `canic` facade re-exports of stable DTOs and macros
* `canic-testkit` exposing generic PocketIC helpers
* demo canisters carrying tiny local role constants and no-op lifecycle hooks
* `dto` types being publicly reachable where they are intended contract surface

Produce:

| Exception | Why Intentional | Scope Guardrail | Still Valid This Run? |
| --------- | --------------- | --------------- | --------------------- |

Rules:

* every exception must include a guardrail
* “intentional” does not exempt an item from reevaluation if ownership drift is observed
* if an exception expanded in scope since baseline, that must appear in Delta Since Baseline

---

## Delta Since Baseline

Highlight only:

* newly public items
* newly widened visibilities
* new crate or subsystem dependencies
* new hub-pressure increases
* new re-export paths that broaden root reachability
* newly introduced exceptions or widened exception scope

Produce:

| Delta Type | Item / Subsystem | Previous | Current | Impact |
| ---------- | ---------------- | -------- | ------- | ------ |

Rules:

* delta must compare only against a **comparable** baseline
* if baseline is non-comparable or absent, report `N/A` and do not invent directional trend claims
* same-day reruns compare against that day’s `module-structure.md` baseline
* first run of the day compares against latest prior comparable report, else `N/A`

---

## Required Output Sections

0. Run Metadata + Comparability Note
1. Public Surface Map
2. Subsystem Dependency Graph
3. Circularity Findings
4. Visibility Hygiene Findings
5. Layering Violations
6. Structural Pressure Areas
7. Drift Sensitivity Summary
8. Structural Risk Index
9. Verification Readout (`PASS` / `FAIL` / `BLOCKED`)

### Section 0 requirements

Run metadata must include:

* target scope
* compared baseline report path
* code snapshot identifier
* method tag/version
* comparability status (`comparable` or `non-comparable` with reason)
* exclusions applied
* notable methodology changes vs baseline

### Verification rules

* `PASS` = no confirmed High or Critical structural violations; only Low or Medium pressure/findings
* `FAIL` = any confirmed High or Critical layering, exposure, seam, or cycle violation
* `BLOCKED` = insufficient scope, inaccessible repo state, or missing evidence for comparable judgment

If `BLOCKED`, the report must state exactly which scope/evidence gap prevented judgment.

---

## Required Method Discipline

The auditor must, at minimum:

1. enumerate root public surface for all public-facing crates in scope
2. inspect crate-root `pub mod` and `pub use` paths
3. inspect module context for every Medium+ finding
4. classify `canic-core` files into the declared subsystem map
5. check directionality against the fixed layer model
6. separate runtime API from macro API
7. inspect demo/test/audit seam boundaries
8. compute hub pressure for required hub modules
9. compare against a comparable baseline where available
10. distinguish pressure from violation in every non-trivial finding

---

## Anti-Shallow Requirement

Do NOT:

* give praise
* comment on naming or formatting
* propose redesign unless a severe violation requires stating the corrective containment direction
* produce Medium/High/Critical findings from grep-only conclusions
* label same-layer coupling a violation without directional evidence
* flag public DTOs as leaks merely because they are widely reachable

Rules:

* do not infer structural violations from symbol mentions alone
* inspect file or module context for every Medium/High/Critical finding
* every claim must identify module/file, dependency or exposed item, visibility scope, and directional or exposure impact
* every recommendation to narrow visibility must be backed by observed usage evidence
* every public leak claim must show root reachability, not merely local `pub`

---

## Optional Report Footer

If useful, include a short footer:

* total public root items by crate
* total re-export count by crate
* total confirmed violations by severity
* total pressure findings by severity
* count of non-comparable deltas omitted

This footer is optional and must not replace the required sections.

---

## Auditor Output Contract

The report must be written so that another reviewer can answer, for every finding:

* what is exposed or coupled
* where it lives
* how visible it is
* why it is pressure or a violation
* what directional or containment rule it touches
* whether it changed since the baseline

If the report cannot support those questions, the run should be marked `BLOCKED` rather than overstated.
